use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, ensure, Context, Result};
use bstr::{BStr, ByteSlice};
use nondestructive::yaml;

/// Compare the processing of some document with a known good source, in this
/// instance `serde_yaml` which is based on `LibYAML`.
#[test]
fn compare_with_libyaml() -> Result<()> {
    let manifest_path =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").context("missing CARGO_MANIFEST_DIR")?)
            .join("tests")
            .join("yaml");

    test_dir(&manifest_path)?;
    Ok(())
}

#[test]
#[ignore = "This test does not pass right now"]
fn compare_yaml_test_suite() -> Result<()> {
    let manifest_path =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").context("missing CARGO_MANIFEST_DIR")?)
            .join("tests")
            .join("yaml-test-suite")
            .join("src");

    test_dir(&manifest_path)?;
    Ok(())
}

fn test_dir(path: &Path) -> Result<()> {
    let mut paths = Vec::new();

    for e in fs::read_dir(path)? {
        let e = e.with_context(|| anyhow!("Reading directory: {}", path.display()))?;
        paths.push(e.path());
    }

    paths.sort();

    let mut errors = Vec::new();

    for path in paths {
        if let Err(error) = compare_path(&path) {
            errors.push((path, error));
        }
    }

    if !errors.is_empty() {
        for (path, error) in &errors {
            println!("{}:", path.display());
            println!("{error}");

            for cause in error.chain().skip(1) {
                println!("  caused by: {cause}");
            }
        }

        bail!("{} errors in comparison", errors.len());
    }

    Ok(())
}

enum Step {
    Key(String),
    Index(usize),
}

#[derive(Default)]
struct Trace {
    path: Vec<Step>,
}

impl fmt::Display for Trace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for step in &self.path {
            match step {
                Step::Key(key) => {
                    write!(f, ".{key}")?;
                }
                Step::Index(index) => {
                    write!(f, "[{index}]")?;
                }
            }
        }

        Ok(())
    }
}

fn compare_path(path: &Path) -> Result<()> {
    let input = fs::read(path)?;

    let reference: serde_yaml::Value =
        serde_yaml::from_slice(&input).context("serde_yaml failed to deserialize")?;

    let document = yaml::from_slice(&input).context("nondestructive failed to deserialize")?;
    let a = document.as_ref();
    let mut trace = Trace::default();
    compare(&mut trace, &a, &reference)?;

    let mut output = Vec::new();

    document
        .write_to(&mut output)
        .context("nondestructive failed to serialize")?;

    if output != input {
        bail!("nondestructive failed to serialize to the same value");
    }

    let c = yaml::from_slice(&output)
        .context("nondestructive failed to deserialize serialized value")?;
    let c = c.as_ref();

    let mut trace = Trace::default();
    compare(&mut trace, &c, &reference)?;
    Ok(())
}

/// Structurally compare two values.
fn compare(trace: &mut Trace, a: &yaml::Value<'_>, b: &serde_yaml::Value) -> Result<()> {
    match (a.as_any(), b) {
        (yaml::Any::Sequence(a), serde_yaml::Value::Sequence(b)) => {
            compare_sequences(trace, &a, b)?;
        }
        (yaml::Any::Mapping(a), serde_yaml::Value::Mapping(b)) => {
            compare_mappings(trace, &a, b)?;
        }
        (yaml::Any::Bool(a), serde_yaml::Value::Bool(b)) => {
            ensure!(a == *b, "{trace}: {a} != {b}");
        }
        (yaml::Any::String(a), serde_yaml::Value::String(b)) => {
            let Ok(a) = a.to_str() else {
                bail!("{trace}: nondestructive is not a utf-8 string, but is a {a:?}");
            };

            ensure!(a == *b, "{trace}: {a:?} != {b:?}");
        }
        (yaml::Any::Number(a), serde_yaml::Value::Number(n)) => 'ok: {
            if let Some(b) = n.as_u64() {
                let Some(a) = a.as_u64() else {
                    bail!("{trace}: nondestructive is not a u64, but is a {a:?}");
                };

                ensure!(a == b, "{trace}: {a} != {b}");
                break 'ok;
            }

            if let Some(b) = n.as_i64() {
                let Some(a) = a.as_i64() else {
                    bail!("{trace}: nondestructive is not a i64, but is a {a:?}");
                };

                ensure!(a == b, "{trace}: {a} != {b}");
                break 'ok;
            }

            if let Some(b) = n.as_f64() {
                const ERROR_MARGIN: f64 = 1e-6;

                let Some(a) = a.as_f64() else {
                    bail!("{trace}: nondestructive is not a f64, but is a {a:?}");
                };

                ensure!((a - b).abs() < ERROR_MARGIN, "{trace}: {a} != {b}");
                break 'ok;
            }

            bail!("{trace}: not comparable: {a:?} == {b:?}");
        }
        _ => {
            bail!("{trace}: not comparable: {a:?} == {b:?}");
        }
    }

    Ok(())
}

/// Structurally compare two sequences.
fn compare_sequences(
    trace: &mut Trace,
    a: &yaml::Sequence<'_>,
    b: &serde_yaml::Sequence,
) -> Result<()> {
    let mut a = a.iter();

    for (index, b) in b.iter().enumerate() {
        let a = a
            .next()
            .with_context(|| anyhow!("{trace}: index {index} missing in nondestructive"))?;

        trace.path.push(Step::Index(index));
        compare(trace, &a, b)?;
        trace.path.pop();
    }

    if let Some(a) = a.next() {
        bail!(
            "{trace}: index {} missing in libyaml: {a:?} != {b:?}",
            b.len()
        );
    }

    Ok(())
}

/// Structurally compare two mappings.
fn compare_mappings(
    trace: &mut Trace,
    a: &yaml::Mapping<'_>,
    b: &serde_yaml::Mapping,
) -> Result<()> {
    let mut expected = a
        .iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect::<HashMap<_, _>>();

    for (key, b) in b {
        let key = key.as_str().context("only string keys supported")?;

        let a = expected
            .remove(BStr::new(key.as_bytes()))
            .with_context(|| anyhow!("{trace}: key {key} missing in nondestructive"))?;

        trace.path.push(Step::Key(key.to_owned()));
        compare(trace, &a, b)?;
        trace.path.pop();
    }

    if !expected.is_empty() {
        let missing = expected
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        bail!("keys `{missing}` missing in libyaml: {a:?} != {b:?}");
    }

    Ok(())
}
