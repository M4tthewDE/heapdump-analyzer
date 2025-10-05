use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use std::{
    fmt::Display,
    fs::File,
    io::{Read, Seek},
    path::Path,
};

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
    LoadClass {
        micros: u32,
        class_serial_number: u32,
        class_object_id: u64,
        stack_trace_serial_number: u32,
        class_name_id: u64,
    },
    Trace {
        micros: u32,
        stack_trace_serial_number: u32,
        thread_serial_number: u32,
        stack_frame_ids: Vec<u64>,
    },
    Frame {
        micros: u32,
        stack_frame_id: u64,
        method_name_id: u64,
        method_signature_id: u64,
        source_file_name_id: u64,
        class_serial_number: u32,
        line_number: i32,
    },
    HeapDumpSegment {
        micros: u32,
        sub_records: Vec<SubRecord>,
    },
    HeapDumpEnd,
}

impl Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Record::Utf8 { .. } => write!(f, "Utf8"),
            Record::LoadClass { .. } => write!(f, "LoadClass"),
            Record::Trace { .. } => write!(f, "Trace"),
            Record::Frame { .. } => write!(f, "Frame"),
            Record::HeapDumpSegment { .. } => write!(f, "HeapDumpSegment"),
            Record::HeapDumpEnd => write!(f, "HeapDumpEnd"),
        }
    }
}

impl Record {
    fn parse(file: &mut File) -> Result<Record> {
        let tag = read_u8(file)?;
        let micros = read_u32(file)?;
        let bytes_remaining = read_u32(file)? as usize;

        match tag {
            0x01 => Self::utf8(file, micros, bytes_remaining),
            0x02 => Self::load_class(file, micros),
            0x04 => Self::frame(file, micros),
            0x05 => Self::trace(file, micros),
            0x1c => Self::heap_dump_segment(file, micros, bytes_remaining),
            _ => Err(anyhow!("invalid tag: 0x{:x}", tag)),
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

    fn load_class(file: &mut File, micros: u32) -> Result<Self> {
        Ok(Self::LoadClass {
            micros,
            class_serial_number: read_u32(file)?,
            class_object_id: read_u64(file)?,
            stack_trace_serial_number: read_u32(file)?,
            class_name_id: read_u64(file)?,
        })
    }

    fn trace(file: &mut File, micros: u32) -> Result<Self> {
        let stack_trace_serial_number = read_u32(file)?;
        let thread_serial_number = read_u32(file)?;
        let number_of_frames = read_u32(file)?;

        let mut stack_frame_ids = Vec::new();
        for _ in 0..number_of_frames {
            stack_frame_ids.push(read_u64(file)?);
        }

        Ok(Self::Trace {
            micros,
            stack_trace_serial_number,
            thread_serial_number,
            stack_frame_ids,
        })
    }

    fn frame(file: &mut File, micros: u32) -> Result<Self> {
        let stack_frame_id = read_u64(file)?;
        let method_name_id = read_u64(file)?;
        let method_signature_id = read_u64(file)?;
        let source_file_name_id = read_u64(file)?;
        let class_serial_number = read_u32(file)?;
        let line_number = read_i32(file)?;

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

    fn heap_dump_segment(file: &mut File, micros: u32, bytes_remaining: usize) -> Result<Self> {
        let start_position = file.stream_position()?;
        let mut sub_records = Vec::new();
        loop {
            let sub_record = SubRecord::new(file)?;
            if matches!(sub_record, SubRecord::HeapDumpEnd) {
                sub_records.push(sub_record);
                break;
            }
            sub_records.push(sub_record);

            if file.stream_position()? - start_position == bytes_remaining as u64 {
                break;
            }
        }

        Ok(Self::HeapDumpSegment {
            micros,
            sub_records,
        })
    }
}

#[derive(Debug)]
pub enum FieldValue {
    NormalObject { object_id: u64 },
    Boolean(u8),
    Char(u16),
    Float(u32),
    Double(u64),
    Byte(u8),
    Short(u16),
    Int(u32),
    Long(u64),
}

#[derive(Debug)]
pub struct Field {
    pub name_id: u64,
    pub value: FieldValue,
}

impl Field {
    fn new(file: &mut File) -> Result<Self> {
        let name_id = read_u64(file)?;
        let typ = read_u8(file)?;

        let value = match typ {
            0x02 => FieldValue::NormalObject {
                object_id: read_u64(file)?,
            },
            0x04 => FieldValue::Boolean(read_u8(file)?),
            0x05 => FieldValue::Char(read_u16(file)?),
            0x06 => FieldValue::Float(read_u32(file)?),
            0x07 => FieldValue::Double(read_u64(file)?),
            0x08 => FieldValue::Byte(read_u8(file)?),
            0x09 => FieldValue::Short(read_u16(file)?),
            0x0a => FieldValue::Int(read_u32(file)?),
            0x0b => FieldValue::Long(read_u64(file)?),
            _ => bail!("invalid field type: 0x{:x}", typ),
        };

        Ok(Self { name_id, value })
    }
}

#[derive(Debug)]
pub struct FieldDescriptor {
    pub name_id: u64,
    pub typ: u8,
}

#[derive(Debug)]
pub enum PrimArrayElement {
    Bool(u8),
    Byte(u8),
    Char(u16),
    Float(u32),
    Double(u64),
    Short(u16),
    Int(u32),
    Long(u64),
}

#[derive(Debug)]
pub enum SubRecord {
    ClassDump {
        class_object_id: u64,
        stack_trace_serial_number: u32,
        super_class_object_id: u64,
        class_loader_object_id: u64,
        signers_object_id: u64,
        protection_domain_object_id: u64,
        reserved1: u64,
        reserved2: u64,
        instance_size: u32,
        constant_pool_size: u16,
        number_of_static_fields: u16,
        static_fields: Vec<Field>,
        number_of_instance_fields: u16,
        instance_field_descriptors: Vec<FieldDescriptor>,
    },
    InstanceDump {
        object_id: u64,
        stack_trace_serial_number: u32,
        class_object_id: u64,
        number_of_bytes: u32,
        raw_field_bytes: Vec<u8>,
    },
    ObjArrayDump {
        object_id: u64,
        stack_trace_serial_number: u32,
        array_class_id: u64,
        elements: Vec<u64>,
    },
    PrimArrayDump {
        object_id: u64,
        stack_trace_serial_number: u32,
        typ: u8,
        elements: Vec<PrimArrayElement>,
    },
    HeapDumpEnd,
}

impl Display for SubRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubRecord::ClassDump { .. } => write!(f, "ClassDump"),
            SubRecord::InstanceDump { .. } => write!(f, "InstanceDump"),
            SubRecord::ObjArrayDump { .. } => write!(f, "ObjArrayDump"),
            SubRecord::PrimArrayDump { .. } => write!(f, "PrimArrayDump"),
            SubRecord::HeapDumpEnd => write!(f, "HeapDumpEnd"),
        }
    }
}

impl SubRecord {
    pub fn new(file: &mut File) -> Result<Self> {
        let sub_record_type = read_u8(file)?;

        match sub_record_type {
            0x20 => Self::class_dump(file),
            0x21 => Self::instance_dump(file),
            0x22 => Self::obj_array_dump(file),
            0x23 => Self::prim_array_dump(file),
            _ => bail!("unknown sub record type: 0x{:x}", sub_record_type),
        }
    }

    fn class_dump(file: &mut File) -> Result<Self> {
        let class_object_id = read_u64(file)?;
        let stack_trace_serial_number = read_u32(file)?;
        let super_class_object_id = read_u64(file)?;
        let class_loader_object_id = read_u64(file)?;
        let signers_object_id = read_u64(file)?;
        let protection_domain_object_id = read_u64(file)?;
        let reserved1 = read_u64(file)?;
        let reserved2 = read_u64(file)?;
        let instance_size = read_u32(file)?;
        let constant_pool_size = read_u16(file)?;

        let number_of_static_fields = read_u16(file)?;
        let mut static_fields = Vec::new();
        for _ in 0..number_of_static_fields {
            static_fields.push(Field::new(file)?);
        }

        let number_of_instance_fields = read_u16(file)?;
        let mut instance_field_descriptors = Vec::new();
        for _ in 0..number_of_instance_fields {
            instance_field_descriptors.push(FieldDescriptor {
                name_id: read_u64(file)?,
                typ: read_u8(file)?,
            });
        }

        Ok(Self::ClassDump {
            class_object_id,
            stack_trace_serial_number,
            super_class_object_id,
            class_loader_object_id,
            signers_object_id,
            protection_domain_object_id,
            reserved1,
            reserved2,
            instance_size,
            constant_pool_size,
            number_of_static_fields,
            static_fields,
            number_of_instance_fields,
            instance_field_descriptors,
        })
    }

    fn instance_dump(file: &mut File) -> Result<Self> {
        let object_id = read_u64(file)?;
        let stack_trace_serial_number = read_u32(file)?;
        let class_object_id = read_u64(file)?;
        let number_of_bytes = read_u32(file)?;
        let mut raw_field_bytes = vec![0; number_of_bytes as usize];
        file.read_exact(&mut raw_field_bytes)?;

        Ok(Self::InstanceDump {
            object_id,
            stack_trace_serial_number,
            class_object_id,
            number_of_bytes,
            raw_field_bytes,
        })
    }

    fn obj_array_dump(file: &mut File) -> Result<Self> {
        let object_id = read_u64(file)?;
        let stack_trace_serial_number = read_u32(file)?;
        let number_of_elements = read_u32(file)?;
        let array_class_id = read_u64(file)?;
        let mut elements = Vec::new();
        for _ in 0..number_of_elements {
            elements.push(read_u64(file)?);
        }

        Ok(Self::ObjArrayDump {
            object_id,
            stack_trace_serial_number,
            array_class_id,
            elements,
        })
    }

    fn prim_array_dump(file: &mut File) -> Result<Self> {
        let object_id = read_u64(file)?;
        let stack_trace_serial_number = read_u32(file)?;
        let number_of_elements = read_u32(file)?;
        let typ = read_u8(file)?;

        let mut elements = Vec::new();
        for _ in 0..number_of_elements {
            let element = match typ {
                4 => PrimArrayElement::Bool(read_u8(file)?),
                5 => PrimArrayElement::Char(read_u16(file)?),
                6 => PrimArrayElement::Float(read_u32(file)?),
                7 => PrimArrayElement::Double(read_u64(file)?),
                8 => PrimArrayElement::Byte(read_u8(file)?),
                9 => PrimArrayElement::Short(read_u16(file)?),
                10 => PrimArrayElement::Int(read_u32(file)?),
                11 => PrimArrayElement::Long(read_u64(file)?),
                _ => bail!("invalid array type: {}", typ),
            };

            elements.push(element);
        }

        Ok(Self::PrimArrayDump {
            object_id,
            stack_trace_serial_number,
            typ,
            elements,
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

fn read_i32(r: &mut impl Read) -> Result<i32> {
    let mut buf = [0; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

fn read_u8(r: &mut impl Read) -> Result<u8> {
    let mut buf = [0; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u16(r: &mut impl Read) -> Result<u16> {
    let mut buf = [0; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
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
