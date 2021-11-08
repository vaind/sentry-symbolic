use std::fmt::Display;
use std::io::Write;

use super::error::ErrorSink;
use super::*;

impl Converter {
    pub fn serialize<W: Write, E: ErrorSink<SerializeError>>(
        self,
        writer: &mut W,
        error_sink: &mut E,
    ) -> std::io::Result<Stats> {
        Ok(Stats {})
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Stats {}

// TODO: thiserror
#[derive(Debug)]
#[non_exhaustive]
pub enum SerializeError {}

impl Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("serializing symcache failed")
    }
}

impl std::error::Error for SerializeError {}
