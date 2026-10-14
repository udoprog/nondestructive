use alloc::format;
use alloc::string::{String, ToString};

use anyhow::{Context, Result};

use crate::yaml::{self, Block, Chomp};

#[derive(Clone, Copy)]
enum Step {
    Key(&'static str),
    Index(usize),
}

use Step::{Index, Key};

fn get(doc: &yaml::Document, path: &[Step]) -> Option<String> {
    let mut id = doc.as_ref().id();

    for step in path {
        let value = doc.value(id);

        id = match *step {
            Key(key) => value.as_mapping()?.get(key)?.id(),
            Index(index) => value.as_sequence()?.get(index)?.id(),
        };
    }

    doc.value(id).as_str().map(String::from)
}

fn get_mut<'a>(mut value: yaml::ValueMut<'a>, path: &[Step]) -> Option<yaml::ValueMut<'a>> {
    for step in path {
        value = match *step {
            Key(key) => value.into_mapping_mut()?.get_into_mut(key)?,
            Index(index) => value.into_sequence_mut()?.get_into_mut(index)?,
        };
    }

    Some(value)
}

fn reference(input: &str, path: &[Step]) -> Result<Option<String>> {
    let mut value: serde_yaml::Value = serde_yaml::from_str(input)?;

    for step in path {
        value = match *step {
            Key(key) => value.get(key).context("missing key")?.clone(),
            Index(index) => value.get(index).context("missing index")?.clone(),
        };
    }

    Ok(match value {
        serde_yaml::Value::Null => None,
        serde_yaml::Value::String(string) => Some(string),
        other => anyhow::bail!("not a string: {other:?}"),
    })
}

/// Where a block is built: the input document, how the block is added, and
/// the path to the resulting value.
struct Position {
    input: &'static str,
    edit: fn(&mut yaml::Document, &[&str], Block),
    path: &'static [Step],
}

fn insert(doc: &mut yaml::Document, lines: &[&str], block: Block) {
    let mut root = doc.as_mut().into_mapping_mut().expect("mapping");
    root.insert_block("b", lines.iter().copied(), block);
}

fn push(doc: &mut yaml::Document, lines: &[&str], block: Block) {
    let mut root = doc.as_mut().into_sequence_mut().expect("sequence");
    root.push_block(lines.iter().copied(), block);
}

fn insert_seq(doc: &mut yaml::Document, lines: &[&str], block: Block) {
    let mut root = doc.as_mut().into_sequence_mut().expect("sequence");
    root.insert_block(1, lines.iter().copied(), block);
}

fn push_nested(doc: &mut yaml::Document, lines: &[&str], block: Block) {
    let mut seq = get_mut(doc.as_mut(), &[Key("a"), Key("b")])
        .and_then(yaml::ValueMut::into_sequence_mut)
        .expect("sequence");
    seq.push_block(lines.iter().copied(), block);
}

fn set_nested(doc: &mut yaml::Document, lines: &[&str], block: Block) {
    let mut value = get_mut(doc.as_mut(), &[Key("a"), Key("k")]).expect("value");
    value.set_block(lines.iter().copied(), block);
}

fn set_root(doc: &mut yaml::Document, lines: &[&str], block: Block) {
    doc.as_mut().set_block(lines.iter().copied(), block);
}

fn set_inner_item(doc: &mut yaml::Document, lines: &[&str], block: Block) {
    let mut value = get_mut(doc.as_mut(), &[Index(0), Index(0)]).expect("value");
    value.set_block(lines.iter().copied(), block);
}

const POSITIONS: &[Position] = &[
    Position {
        input: "k: v\n",
        edit: insert,
        path: &[Key("b")],
    },
    Position {
        input: "a:\n  k: v\n",
        edit: insert,
        path: &[Key("b")],
    },
    Position {
        input: "- v\n",
        edit: push,
        path: &[Index(1)],
    },
    Position {
        input: "a:\n  b:\n    - k\n",
        edit: push_nested,
        path: &[Key("a"), Key("b"), Index(1)],
    },
    Position {
        input: "a:\n    k: v\nz: 1\n",
        edit: set_nested,
        path: &[Key("a"), Key("k")],
    },
    Position {
        input: "a:\n    k:\n      x: 1\nz: 1\n",
        edit: set_nested,
        path: &[Key("a"), Key("k")],
    },
    Position {
        input: "a:\n  k:\n    - x\nz: 1\n",
        edit: set_nested,
        path: &[Key("a"), Key("k")],
    },
    Position {
        input: "v\n",
        edit: set_root,
        path: &[],
    },
    Position {
        input: "- - v\n",
        edit: set_inner_item,
        path: &[Index(0), Index(0)],
    },
    Position {
        input: "k: v",
        edit: insert,
        path: &[Key("b")],
    },
    Position {
        input: "a:\n  k: v",
        edit: insert,
        path: &[Key("b")],
    },
    Position {
        input: "- v",
        edit: push,
        path: &[Index(1)],
    },
    Position {
        input: "a:\n  b:\n    - k",
        edit: push_nested,
        path: &[Key("a"), Key("b"), Index(1)],
    },
    Position {
        input: "a:\n    k: v",
        edit: set_nested,
        path: &[Key("a"), Key("k")],
    },
    Position {
        input: "v",
        edit: set_root,
        path: &[],
    },
    Position {
        input: "- - v",
        edit: set_inner_item,
        path: &[Index(0), Index(0)],
    },
    Position {
        input: "k: v\n\n",
        edit: insert,
        path: &[Key("b")],
    },
    Position {
        input: "b: v\n\nz: 1\n",
        edit: insert,
        path: &[Key("b")],
    },
    Position {
        input: "a:\n    k: v\n\nz: 1\n",
        edit: set_nested,
        path: &[Key("a"), Key("k")],
    },
    Position {
        input: "a:\n    k: v\n  \n\n# comment\n\nz: 1\n",
        edit: set_nested,
        path: &[Key("a"), Key("k")],
    },
    Position {
        input: "a:\n  k:\n    - x\n\n  z: 1\n",
        edit: set_nested,
        path: &[Key("a"), Key("k")],
    },
    Position {
        input: "- one\n\n- two\n",
        edit: insert_seq,
        path: &[Index(1)],
    },
    Position {
        input: "- - v\n\n- w\n",
        edit: set_inner_item,
        path: &[Index(0), Index(0)],
    },
];

const LINES: &[&[&str]] = &[
    &["x", "y"],
    &["  indented", "x"],
    &["\tindented", "x"],
    &["", "  indented", "x"],
    &["   "],
    &["a\nb"],
    &["a\n"],
    &["x", ""],
    &["x", "", ""],
    &["", "x"],
    &["x", "", "y"],
    &["x", "", "", "y"],
    &["x", "  more", "y"],
    &["x", "  more", "", "y"],
    &["# not a comment", "- not a sequence"],
    &[""],
    &[],
];

const BLOCKS: &[Block] = &[
    Block::Literal(Chomp::Strip),
    Block::Literal(Chomp::Clip),
    Block::Literal(Chomp::Keep),
    Block::Folded(Chomp::Strip),
    Block::Folded(Chomp::Clip),
    Block::Folded(Chomp::Keep),
];

/// The value of a built block agrees with `serde_yaml` and with reparsing the
/// output, and no line gains trailing whitespace.
#[test]
fn built_blocks_reparse_to_their_value() -> Result<()> {
    for position in POSITIONS {
        for lines in LINES {
            for &block in BLOCKS {
                let mut doc = yaml::from_slice(position.input)?;
                (position.edit)(&mut doc, lines, block);
                let output = doc.to_string();
                let context = format!(
                    "{lines:?} as {block:?} into {:?}: {output:?}",
                    position.input
                );

                let value = get(&doc, position.path)
                    .with_context(|| format!("missing value for {context}"))?;

                let expected = reference(&output, position.path)
                    .with_context(|| format!("serde_yaml rejects {context}"))?
                    .unwrap_or_default();
                assert_eq!(value, expected, "serde_yaml disagrees for {context}");

                let reparsed = yaml::from_slice(&output)?;
                assert_eq!(
                    get(&reparsed, position.path).unwrap_or_default(),
                    value,
                    "reparse disagrees for {context}"
                );

                let has_trailing_ws = lines.iter().any(|l| l.ends_with([' ', '\t']))
                    || position.input.lines().any(|l| l.ends_with(' '));

                if !has_trailing_ws {
                    assert!(
                        output.lines().all(|l| !l.ends_with(' ')),
                        "trailing whitespace for {context}"
                    );
                }
            }
        }
    }

    Ok(())
}

#[test]
fn leading_space_gets_indentation_indicator() -> Result<()> {
    let mut doc = yaml::from_slice("k: v\n")?;
    insert(&mut doc, &["  indented", "x"], Block::Literal(Chomp::Clip));
    assert_eq!(doc.to_string(), "k: v\nb: |2\n    indented\n  x\n");

    let b = doc.as_ref().as_mapping().and_then(|m| m.get("b"));
    assert_eq!(b.and_then(|v| v.as_str()), Some("  indented\nx\n"));

    let mut doc = yaml::from_slice("k: v\n")?;
    insert(&mut doc, &["", "  indented"], Block::Folded(Chomp::Strip));
    assert_eq!(doc.to_string(), "k: v\nb: >2-\n\n    indented\n");
    Ok(())
}

#[test]
fn embedded_newlines_split_lines() -> Result<()> {
    let mut doc = yaml::from_slice("k: v\n")?;
    insert(&mut doc, &["a\nb"], Block::Literal(Chomp::Clip));
    assert_eq!(doc.to_string(), "k: v\nb: |\n  a\n  b\n");

    let b = doc.as_ref().as_mapping().and_then(|m| m.get("b"));
    assert_eq!(b.and_then(|v| v.as_str()), Some("a\nb\n"));
    Ok(())
}

#[test]
fn trailing_empty_lines_follow_chomping() -> Result<()> {
    let cases = [
        (Chomp::Strip, "k: v\nb: |-\n  x\n\n", "x"),
        (Chomp::Clip, "k: v\nb: |\n  x\n\n", "x\n"),
        (Chomp::Keep, "k: v\nb: |+\n  x\n\n", "x\n\n"),
    ];

    for (chomp, output, value) in cases {
        let mut doc = yaml::from_slice("k: v\n")?;
        insert(&mut doc, &["x", ""], Block::Literal(chomp));
        assert_eq!(doc.to_string(), output);

        let b = doc.as_ref().as_mapping().and_then(|m| m.get("b"));
        assert_eq!(b.and_then(|v| v.as_str()), Some(value));
    }

    Ok(())
}

#[test]
fn set_block_indents_past_parent() -> Result<()> {
    let mut doc = yaml::from_slice("a:\n    k: v\n")?;
    set_nested(&mut doc, &["x", "y"], Block::Literal(Chomp::Clip));
    assert_eq!(doc.to_string(), "a:\n    k: |\n      x\n      y\n");
    Ok(())
}

#[test]
fn empty_block() -> Result<()> {
    let mut doc = yaml::from_slice("k: v\n")?;
    insert(&mut doc, &[], Block::Literal(Chomp::Clip));
    assert_eq!(doc.to_string(), "k: v\nb: |\n");

    let b = doc.as_ref().as_mapping().and_then(|m| m.get("b"));
    assert_eq!(b.and_then(|v| v.as_str()), Some(""));

    Ok(())
}

#[test]
fn block_at_end_without_newline_gets_one() -> Result<()> {
    let mut doc = yaml::from_slice("k: v")?;
    insert(&mut doc, &["x"], Block::Literal(Chomp::Clip));
    assert_eq!(doc.to_string(), "k: v\nb: |\n  x\n");

    let b = doc.as_ref().as_mapping().and_then(|m| m.get("b"));
    assert_eq!(b.and_then(|v| v.as_str()), Some("x\n"));

    let mut doc = yaml::from_slice("k: v")?;
    insert(&mut doc, &["x"], Block::Literal(Chomp::Strip));
    assert_eq!(doc.to_string(), "k: v\nb: |-\n  x");
    Ok(())
}

#[test]
fn keep_takes_following_empty_lines() -> Result<()> {
    let input = "a:\n    k: v\n\n  \n# comment\n\nz: 1\n";
    let mut doc = yaml::from_slice(input)?;
    set_nested(&mut doc, &["x"], Block::Literal(Chomp::Keep));

    let output = doc.to_string();
    assert_eq!(output, "a:\n    k: |+\n      x\n\n  \n# comment\n\nz: 1\n");
    assert_eq!(get(&doc, &[Key("a"), Key("k")]).as_deref(), Some("x\n\n\n"));

    let z = doc.as_ref().as_mapping().and_then(|m| m.get("z"));
    assert_eq!(z.and_then(|v| v.as_u32()), Some(1));
    Ok(())
}
