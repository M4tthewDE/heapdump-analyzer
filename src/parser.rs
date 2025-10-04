use anyhow::{Result, anyhow};
use std::{fs::File, io::Read, path::Path};

#[derive(Debug)]
pub enum Version {
    JavaProfile102,
}

impl Version {
    fn new(version_str: &str) -> Result<Version> {
        match version_str {
            "JAVA PROFILE 1.0.2" => Ok(Version::JavaProfile102),
            _ => Err(anyhow!("Invalid version: {}", version_str)),
        }
    }
}

// https://github.com/openjdk/jdk17/blob/4afbcaf55383ec2f5da53282a1547bac3d099e9d/src/hotspot/share/services/heapDumper.cpp#L62
#[derive(Debug)]
pub struct Heap {
    pub version: Version,
}

pub fn parse(path: &Path) -> Result<Heap> {
    let mut file = File::open(path)?;

    let mut version_buf = [0; 18];
    file.read_exact(&mut version_buf)?;

    let version_str = String::from_utf8(version_buf.to_vec())?;

    Ok(Heap {
        version: Version::new(&version_str)?,
    })
}
