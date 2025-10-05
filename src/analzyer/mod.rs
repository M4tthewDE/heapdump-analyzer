use std::{collections::HashMap, fmt::Display};

use anyhow::{Context, Result};

use crate::parser::{Id, ParsedHeap, Record, sub_record::SubRecord};

#[derive(Clone)]
pub struct Class {
    pub id: Id,
    pub name: String,
}

pub struct Instance {
    pub id: Id,
    pub class: Class,
}

pub struct Frame {
    pub id: Id,
    pub method_name: String,
    pub method_signature: String,
    pub source_file_name: String,
    pub class_serial_number: u32,
    pub line_number: i32,
}

impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}:{}",
            self.source_file_name, self.method_name, self.line_number
        )
    }
}

pub struct AnalyzedHeap {
    pub strings: HashMap<Id, String>,
    pub classes: HashMap<Id, Class>,
    pub frames: Vec<Frame>,
    pub instances: HashMap<Id, Instance>,
}

impl AnalyzedHeap {
    pub fn analyze(parsed_heap: &ParsedHeap) -> Result<Self> {
        let mut strings = HashMap::new();
        let mut classes = HashMap::new();

        for record in &parsed_heap.records {
            match record {
                Record::Utf8 {
                    name_id, content, ..
                } => {
                    strings.insert(*name_id, content.to_string());
                }
                _ => {}
            }
        }

        let mut frames = Vec::new();
        let mut instances = HashMap::new();

        for record in &parsed_heap.records {
            match record {
                Record::Frame {
                    stack_frame_id,
                    method_name_id,
                    method_signature_id,
                    source_file_name_id,
                    class_serial_number,
                    line_number,
                    ..
                } => frames.push(Frame {
                    id: *stack_frame_id,
                    method_name: strings
                        .get(method_name_id)
                        .cloned()
                        .context("method name string not found")?,
                    method_signature: strings
                        .get(method_signature_id)
                        .cloned()
                        .context("method signature string not found")?,
                    source_file_name: strings
                        .get(source_file_name_id)
                        .cloned()
                        .context("source file name string not found")?,
                    class_serial_number: *class_serial_number,
                    line_number: *line_number,
                }),
                Record::LoadClass {
                    class_object_id,
                    class_name_id,
                    ..
                } => {
                    classes.insert(
                        *class_object_id,
                        Class {
                            id: *class_object_id,
                            name: strings
                                .get(class_name_id)
                                .cloned()
                                .context("unknown class name string")?,
                        },
                    );
                }
                Record::HeapDumpSegment { sub_records, .. } => {
                    for sub_record in sub_records {
                        match sub_record {
                            SubRecord::InstanceDump {
                                object_id,
                                class_object_id,
                                ..
                            } => {
                                instances.insert(
                                    *object_id,
                                    Instance {
                                        id: *object_id,
                                        class: classes
                                            .get(class_object_id)
                                            .cloned()
                                            .context("class not found")?,
                                    },
                                );
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            strings,
            frames,
            classes,
            instances,
        })
    }
}
