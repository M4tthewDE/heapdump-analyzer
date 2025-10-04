use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
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
    pub identifier_size: u32,
    pub timestamp: DateTime<Utc>,
}

pub fn parse(path: &Path) -> Result<Heap> {
    let mut file = File::open(path)?;

    let mut version_buf = [0; 18];
    file.read_exact(&mut version_buf)?;

    let version_str = String::from_utf8(version_buf.to_vec())?;

    // skip 0-byte
    file.read(&mut [0; 1])?;

    let mut identifier_size_buf = [0; 4];
    file.read_exact(&mut identifier_size_buf)?;
    let identifier_size = u32::from_be_bytes(identifier_size_buf);

    let mut timestamp_buf = [0; 8];
    file.read_exact(&mut timestamp_buf)?;
    let timestamp = u64::from_be_bytes(timestamp_buf);
    let timestamp =
        DateTime::from_timestamp_millis(timestamp as i64).context("invalid timestamp")?;

    Ok(Heap {
        version: Version::new(&version_str)?,
        identifier_size,
        timestamp,
    })
}
