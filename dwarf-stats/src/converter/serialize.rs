use std::fmt::Display;
use std::io::Write;

use thiserror::Error;

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

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SerializeError {}
