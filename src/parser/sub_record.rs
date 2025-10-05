use std::{fmt::Display, io::Read};

use anyhow::{Result, bail};

use crate::parser::util::{read_u8, read_u16, read_u32, read_u64};

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
    fn new(r: &mut impl Read) -> Result<Self> {
        let name_id = read_u64(r)?;
        let typ = read_u8(r)?;

        let value = match typ {
            0x02 => FieldValue::NormalObject {
                object_id: read_u64(r)?,
            },
            0x04 => FieldValue::Boolean(read_u8(r)?),
            0x05 => FieldValue::Char(read_u16(r)?),
            0x06 => FieldValue::Float(read_u32(r)?),
            0x07 => FieldValue::Double(read_u64(r)?),
            0x08 => FieldValue::Byte(read_u8(r)?),
            0x09 => FieldValue::Short(read_u16(r)?),
            0x0a => FieldValue::Int(read_u32(r)?),
            0x0b => FieldValue::Long(read_u64(r)?),
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
    ThreadObj {
        object_id: u64,
        sequence_number: u32,
        stack_trace_sequence_number: u32,
    },
    JavaFrame {
        object_id: u64,
        thread_serial_number: u32,
        frame_number: u32,
    },
    JniLocal {
        object_id: u64,
        thread_serial_number: u32,
        frame_number: u32,
    },
    JniGlobal {
        object_id: u64,
        global_ref_id: u64,
    },
    StickyClass {
        object_id: u64,
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
            SubRecord::ThreadObj { .. } => write!(f, "ThreadObj"),
            SubRecord::JavaFrame { .. } => write!(f, "JavaFrame"),
            SubRecord::JniLocal { .. } => write!(f, "JniLocal"),
            SubRecord::JniGlobal { .. } => write!(f, "JniGlobal"),
            SubRecord::StickyClass { .. } => write!(f, "StickyClass"),
            SubRecord::HeapDumpEnd => write!(f, "HeapDumpEnd"),
        }
    }
}

impl SubRecord {
    pub fn new(r: &mut impl Read) -> Result<Self> {
        let sub_record_type = read_u8(r)?;

        match sub_record_type {
            0x01 => Self::jni_global(r),
            0x02 => Self::jni_local(r),
            0x03 => Self::java_frame(r),
            0x05 => Self::sticky_class(r),
            0x08 => Self::thread_obj(r),
            0x20 => Self::class_dump(r),
            0x21 => Self::instance_dump(r),
            0x22 => Self::obj_array_dump(r),
            0x23 => Self::prim_array_dump(r),
            _ => bail!("unknown sub record type: 0x{:x}", sub_record_type),
        }
    }

    fn class_dump(r: &mut impl Read) -> Result<Self> {
        let class_object_id = read_u64(r)?;
        let stack_trace_serial_number = read_u32(r)?;
        let super_class_object_id = read_u64(r)?;
        let class_loader_object_id = read_u64(r)?;
        let signers_object_id = read_u64(r)?;
        let protection_domain_object_id = read_u64(r)?;
        let reserved1 = read_u64(r)?;
        let reserved2 = read_u64(r)?;
        let instance_size = read_u32(r)?;
        let constant_pool_size = read_u16(r)?;

        let number_of_static_fields = read_u16(r)?;
        let mut static_fields = Vec::new();
        for _ in 0..number_of_static_fields {
            static_fields.push(Field::new(r)?);
        }

        let number_of_instance_fields = read_u16(r)?;
        let mut instance_field_descriptors = Vec::new();
        for _ in 0..number_of_instance_fields {
            instance_field_descriptors.push(FieldDescriptor {
                name_id: read_u64(r)?,
                typ: read_u8(r)?,
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

    fn instance_dump(r: &mut impl Read) -> Result<Self> {
        let object_id = read_u64(r)?;
        let stack_trace_serial_number = read_u32(r)?;
        let class_object_id = read_u64(r)?;
        let number_of_bytes = read_u32(r)?;
        let mut raw_field_bytes = vec![0; number_of_bytes as usize];
        r.read_exact(&mut raw_field_bytes)?;

        Ok(Self::InstanceDump {
            object_id,
            stack_trace_serial_number,
            class_object_id,
            number_of_bytes,
            raw_field_bytes,
        })
    }

    fn obj_array_dump(r: &mut impl Read) -> Result<Self> {
        let object_id = read_u64(r)?;
        let stack_trace_serial_number = read_u32(r)?;
        let number_of_elements = read_u32(r)?;
        let array_class_id = read_u64(r)?;
        let mut elements = Vec::new();
        for _ in 0..number_of_elements {
            elements.push(read_u64(r)?);
        }

        Ok(Self::ObjArrayDump {
            object_id,
            stack_trace_serial_number,
            array_class_id,
            elements,
        })
    }

    fn prim_array_dump(r: &mut impl Read) -> Result<Self> {
        let object_id = read_u64(r)?;
        let stack_trace_serial_number = read_u32(r)?;
        let number_of_elements = read_u32(r)?;
        let typ = read_u8(r)?;

        let mut elements = Vec::new();
        for _ in 0..number_of_elements {
            let element = match typ {
                4 => PrimArrayElement::Bool(read_u8(r)?),
                5 => PrimArrayElement::Char(read_u16(r)?),
                6 => PrimArrayElement::Float(read_u32(r)?),
                7 => PrimArrayElement::Double(read_u64(r)?),
                8 => PrimArrayElement::Byte(read_u8(r)?),
                9 => PrimArrayElement::Short(read_u16(r)?),
                10 => PrimArrayElement::Int(read_u32(r)?),
                11 => PrimArrayElement::Long(read_u64(r)?),
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

    fn thread_obj(r: &mut impl Read) -> Result<Self> {
        Ok(Self::ThreadObj {
            object_id: read_u64(r)?,
            sequence_number: read_u32(r)?,
            stack_trace_sequence_number: read_u32(r)?,
        })
    }

    fn java_frame(r: &mut impl Read) -> Result<Self> {
        Ok(Self::JavaFrame {
            object_id: read_u64(r)?,
            thread_serial_number: read_u32(r)?,
            frame_number: read_u32(r)?,
        })
    }

    fn jni_local(r: &mut impl Read) -> Result<Self> {
        Ok(Self::JniLocal {
            object_id: read_u64(r)?,
            thread_serial_number: read_u32(r)?,
            frame_number: read_u32(r)?,
        })
    }

    fn jni_global(r: &mut impl Read) -> Result<Self> {
        Ok(Self::JniGlobal {
            object_id: read_u64(r)?,
            global_ref_id: read_u64(r)?,
        })
    }

    fn sticky_class(r: &mut impl Read) -> Result<Self> {
        Ok(Self::StickyClass {
            object_id: read_u64(r)?,
        })
    }
}
