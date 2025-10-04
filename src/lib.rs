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
    pub timestamp: DateTime<Utc>,
    pub records: Vec<Record>,
}

pub fn parse(path: &Path) -> Result<Heap> {
    let mut file = File::open(path)?;

    let version = read_utf8(&mut file, 18)?;

    // skip 0-byte
    read_u8(&mut file)?;

    let identifier_size = read_u32(&mut file)?;

    if identifier_size != 8 {
        bail!("only 64bit heapdumps supported");
    }

    let timestamp = DateTime::from_timestamp_millis(read_u64(&mut file)? as i64)
        .context("invalid timestamp")?;

    let mut records = Vec::new();
    loop {
        let record = Record::parse(&mut file)?;
        if matches!(record, Record::HeapDumpEnd) {
            records.push(record);
            break;
        }

        records.push(record);
    }

    Ok(Heap {
        version: Version::new(&version)?,
        timestamp,
        records,
    })
}

#[derive(Debug)]
pub enum Record {
    Utf8 {
        micros: u32,
        id: u64,
        content: String,
    },
    HeapDumpEnd,
}

impl Record {
    fn parse(file: &mut File) -> Result<Record> {
        let tag = read_u8(file)?;
        let micros = read_u32(file)?;
        let bytes_remaining = read_u32(file)? as usize;

        match tag {
            1 => Ok(Self::utf8(file, micros, bytes_remaining)?),
            _ => Err(anyhow!("invalid tag: {}", tag)),
        }
    }

    fn utf8(file: &mut File, micros: u32, bytes_remaining: usize) -> Result<Self> {
        let id = read_u64(file)?;
        let content = read_utf8(file, bytes_remaining - 8)?;
        Ok(Self::Utf8 {
            micros,
            id,
            content,
        })
    }
}

fn read_utf8(r: &mut impl Read, size: usize) -> Result<String> {
    let mut buf = vec![0; size];
    r.read_exact(&mut buf)?;

    // fix java uf8 quirks
    let mut fixed_buf = Vec::new();
    let mut i = 0;
    loop {
        if i == size {
            break;
        }

        let b = buf[i];
        if b == 0xC0 && i < buf.len() - 1 && buf[i + 1] == 0x80 {
            fixed_buf.push(0);
            i += 1;
        } else {
            fixed_buf.push(b);
        }

        i += 1;
    }

    Ok(String::from_utf8(fixed_buf.to_vec())?)
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
