use anyhow::{Context, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let path_arg = std::env::args()
        .skip(1)
        .next()
        .context("no heapdump path provided")?;
    let path = PathBuf::from(path_arg);
    let heap = heapdump_analyzer::parse(&path)?;
    dbg!(heap);
    Ok(())
}
