use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use std::{fs::File, io::Read, path::Path};

#[derive(Debug)]
pub enum Version {
    JavaProfile102,
}

impl Version {
    fn new(version_str: &str) -> Result<Self> {
        match version_str {
            "JAVA PROFILE 1.0.2" => Ok(Self::JavaProfile102),
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
    pub records: Vec<Record>,
}

pub fn parse(path: &Path) -> Result<Heap> {
    let mut file = File::open(path)?;

    let version = read_utf8(&mut file, 18)?;

    // skip 0-byte
    read_u8(&mut file)?;

    let identifier_size = read_u32(&mut file)?;

    let timestamp = DateTime::from_timestamp_millis(read_u64(&mut file)? as i64)
        .context("invalid timestamp")?;

    let mut records = Vec::new();
    loop {
        let record = Record::parse(&mut file)?;
        if matches!(record.tag, Tag::HeapDumpEnd) {
            records.push(record);
            break;
        }

        records.push(record);
    }

    Ok(Heap {
        version: Version::new(&version)?,
        identifier_size,
        timestamp,
        records,
    })
}
#[derive(Debug, Clone)]
pub enum Tag {
    Utf8,
    HeapDumpEnd,
}

impl Tag {
    fn new(byte: u8) -> Result<Tag> {
        match byte {
            1 => Ok(Self::Utf8),
            _ => Err(anyhow!("invalid tag: {}", byte)),
        }
    }
}

#[derive(Debug)]
pub struct Record {
    pub tag: Tag,
}

impl Record {
    fn parse(file: &mut File) -> Result<Record> {
        let tag = Tag::new(read_u8(file)?)?;

        bail!("not implemented {:?}", tag)
    }
}

fn read_utf8(r: &mut impl Read, size: usize) -> Result<String> {
    let mut buf = vec![0; size];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf.to_vec())?)
}

fn read_u8(r: &mut impl Read) -> Result<u8> {
    let mut buf = [0; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32(r: &mut impl Read) -> Result<u32> {
    let mut buf = [0; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn read_u64(r: &mut impl Read) -> Result<u64> {
    let mut buf = [0; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}
