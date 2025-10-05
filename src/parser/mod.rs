use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use std::{
    fmt::Display,
    io::{Cursor, Read, Seek},
    path::Path,
};

use crate::parser::{
    sub_record::SubRecord,
    util::{read_i32, read_u8, read_u32, read_u64, read_utf8},
};

pub mod sub_record;
mod util;

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
pub struct ParsedHeap {
    pub version: Version,
    pub timestamp: DateTime<Utc>,
    pub records: Vec<Record>,
}

impl ParsedHeap {
    pub fn parse(path: &Path) -> Result<Self> {
        let contents = std::fs::read(path)?;
        let mut cursor = Cursor::new(contents);

        let version = read_utf8(&mut cursor, 18)?;

        // skip 0-byte
        read_u8(&mut cursor)?;

        let identifier_size = read_u32(&mut cursor)?;

        if identifier_size != 8 {
            bail!("only 64bit heapdumps supported");
        }

        let timestamp = DateTime::from_timestamp_millis(read_u64(&mut cursor)? as i64)
            .context("invalid timestamp")?;

        let mut records = Vec::new();
        loop {
            let record = Record::parse(&mut cursor)?;

            if matches!(record, Record::HeapDumpEnd { .. }) {
                records.push(record);
                break;
            }

            records.push(record);
        }

        Ok(Self {
            version: Version::new(&version)?,
            timestamp,
            records,
        })
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct Id(pub u64);

impl From<u64> for Id {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub enum Record {
    Utf8 {
        micros: u32,
        name_id: Id,
        content: String,
    },
    LoadClass {
        micros: u32,
        class_serial_number: u32,
        class_object_id: Id,
        stack_trace_serial_number: u32,
        class_name_id: Id,
    },
    Trace {
        micros: u32,
        stack_trace_serial_number: u32,
        thread_serial_number: u32,
        stack_frame_ids: Vec<Id>,
    },
    Frame {
        micros: u32,
        stack_frame_id: Id,
        method_name_id: Id,
        method_signature_id: Id,
        source_file_name_id: Id,
        class_serial_number: u32,
        line_number: i32,
    },
    HeapDumpSegment {
        micros: u32,
        sub_records: Vec<SubRecord>,
    },
    HeapDumpEnd {
        micros: u32,
    },
}

impl Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Record::Utf8 { .. } => write!(f, "Utf8"),
            Record::LoadClass { .. } => write!(f, "LoadClass"),
            Record::Trace { .. } => write!(f, "Trace"),
            Record::Frame { .. } => write!(f, "Frame"),
            Record::HeapDumpSegment { .. } => write!(f, "HeapDumpSegment"),
            Record::HeapDumpEnd { .. } => write!(f, "HeapDumpEnd"),
        }
    }
}

impl Record {
    fn parse(r: &mut (impl Read + Seek)) -> Result<Record> {
        let tag = read_u8(r)?;
        let micros = read_u32(r)?;
        let bytes_remaining = read_u32(r)? as usize;

        match tag {
            0x01 => Self::utf8(r, micros, bytes_remaining),
            0x02 => Self::load_class(r, micros),
            0x04 => Self::frame(r, micros),
            0x05 => Self::trace(r, micros),
            0x1c => Self::heap_dump_segment(r, micros, bytes_remaining),
            0x2c => Ok(Self::HeapDumpEnd { micros }),
            _ => Err(anyhow!("invalid tag: 0x{:x}", tag)),
        }
    }

    fn utf8(r: &mut impl Read, micros: u32, bytes_remaining: usize) -> Result<Self> {
        let name_id = read_u64(r)?.into();
        let content = read_utf8(r, bytes_remaining - 8)?;
        Ok(Self::Utf8 {
            micros,
            name_id,
            content,
        })
    }

    fn load_class(r: &mut impl Read, micros: u32) -> Result<Self> {
        Ok(Self::LoadClass {
            micros,
            class_serial_number: read_u32(r)?,
            class_object_id: read_u64(r)?.into(),
            stack_trace_serial_number: read_u32(r)?,
            class_name_id: read_u64(r)?.into(),
        })
    }

    fn trace(r: &mut impl Read, micros: u32) -> Result<Self> {
        let stack_trace_serial_number = read_u32(r)?;
        let thread_serial_number = read_u32(r)?;
        let number_of_frames = read_u32(r)?;

        let mut stack_frame_ids = Vec::new();
        for _ in 0..number_of_frames {
            stack_frame_ids.push(read_u64(r)?.into());
        }

        Ok(Self::Trace {
            micros,
            stack_trace_serial_number,
            thread_serial_number,
            stack_frame_ids,
        })
    }

    fn frame(r: &mut impl Read, micros: u32) -> Result<Self> {
        let stack_frame_id = read_u64(r)?.into();
        let method_name_id = read_u64(r)?.into();
        let method_signature_id = read_u64(r)?.into();
        let source_file_name_id = read_u64(r)?.into();
        let class_serial_number = read_u32(r)?;
        let line_number = read_i32(r)?;

        Ok(Self::Frame {
            micros,
            stack_frame_id,
            method_name_id,
            method_signature_id,
            source_file_name_id,
            class_serial_number,
            line_number,
        })
    }

    fn heap_dump_segment(
        r: &mut (impl Read + Seek),
        micros: u32,
        bytes_remaining: usize,
    ) -> Result<Self> {
        let start_position = r.stream_position()?;
        let mut sub_records = Vec::new();
        loop {
            let sub_record = SubRecord::new(r)?;
            if matches!(sub_record, SubRecord::HeapDumpEnd) {
                sub_records.push(sub_record);
                break;
            }
            sub_records.push(sub_record);

            if r.stream_position()? - start_position == bytes_remaining as u64 {
                break;
            }
        }

        Ok(Self::HeapDumpSegment {
            micros,
            sub_records,
        })
    }
}
