use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::{env, mem};

use anyhow::{anyhow, bail, ensure, Context, Result};
use bstr::BStr;
use nondestructive::yaml;

/// Compare the processing of some document with a known good source, in this
/// instance `serde_yaml` which is based on `LibYAML`.
#[test]
fn compare_with_libyaml() -> Result<()> {
    let manifest_path =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").context("missing CARGO_MANIFEST_DIR")?)
            .join("tests")
            .join("yaml");

    for e in fs::read_dir(&manifest_path)? {
        let e = e?;
        let path = e.path();
        compare_path(&path).with_context(|| anyhow!("{}", path.display()))?;
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
        let mut leading = true;

        for step in &self.path {
            let leading = !mem::take(&mut leading);

            match step {
                Step::Key(key) => {
                    if !leading {
                        write!(f, ".")?;
                    }

                    write!(f, "{key}")?;
                }
                Step::Index(index) => {
                    write!(f, "[{index}]")?;
                }
            }
        }

        Ok(())
    }
}

fn compare_path(path: &Path) -> Result<(), anyhow::Error> {
    let contents = fs::read(path)?;
    let a = yaml::from_slice(&contents).context("nondestructive failed to deserialize")?;
    let a = a.as_ref();
    let b: serde_yaml::Value =
        serde_yaml::from_slice(&contents).context("serde_yaml failed to deserialize")?;
    let mut trace = Trace::default();
    compare(&mut trace, &a, &b)?;
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
        (yaml::Any::Scalar(a), serde_yaml::Value::Bool(b)) => {
            let a = a
                .as_bool()
                .with_context(|| anyhow!("{trace}: nondestructive is not a bool"))?;

            ensure!(a == *b, "{trace}: {a} != {b}");
        }
        (yaml::Any::Scalar(a), serde_yaml::Value::String(b)) => {
            let a = a
                .as_str()
                .with_context(|| anyhow!("{trace}: nondestructive is not a string"))?;

            ensure!(a == *b, "{trace}: {a} != {b}");
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

    for (key, b) in b.iter() {
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
