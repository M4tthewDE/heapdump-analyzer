use anyhow::Result;
use std::path::PathBuf;

mod parser;

fn main() -> Result<()> {
    let path_arg = std::env::args().skip(1).next().unwrap();
    let path = PathBuf::from(path_arg);
    let heap = parser::parse(&path)?;

    dbg!(heap);
    Ok(())
}
