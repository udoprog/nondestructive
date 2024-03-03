use std::env;
use std::fs;

use anyhow::{Context, Result};

use nondestructive::yaml;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let path = args.next().context("Missing path argument")?;

    let bytes = fs::read(&path).with_context(|| path.clone())?;
    let doc = yaml::from_slice(bytes).with_context(|| path.clone())?;

    print!("{doc}");
    Ok(())
}
