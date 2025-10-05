use anyhow::{Result, bail};

use crate::parser::ParsedHeap;

pub struct AnalyzedHeap {}

impl AnalyzedHeap {
    pub fn analyze(_parsed_heap: &ParsedHeap) -> Result<Self> {
        bail!("not implemented: analzye")
    }
}
