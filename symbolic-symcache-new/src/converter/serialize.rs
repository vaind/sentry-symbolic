use std::io::Write;

use thiserror::Error;

use super::*;
use crate::format::raw;
use crate::ErrorSink;

impl Converter {
    /// Serialize the converted data.
    ///
    /// This writes the SymCache binary format into the given [`Write`].
    /// Any errors raised during serialization will be handed to the given [`ErrorSink`].
    pub fn serialize<W: Write, E: ErrorSink<SerializeError>>(
        self,
        writer: &mut W,
        mut error_sink: E,
    ) -> std::io::Result<Stats> {
        let _ = &mut error_sink;
        let mut writer = WriteWrapper::new(writer);

        let num_strings = self.strings.len() as u32;
        let num_files = self.files.len() as u32;
        let num_functions = self.functions.len() as u32;
        let num_source_locations = (self.source_locations.len() + self.ranges.len()) as u32;
        let num_ranges = self.ranges.len() as u32;
        let string_bytes = self.string_bytes.len() as u32;

        let header = raw::Header {
            magic: raw::SYMCACHE_MAGIC,
            version: raw::SYMCACHE_VERSION,
            num_strings,
            num_files,
            num_functions,
            num_source_locations,
            num_ranges,
            string_bytes,
        };

        writer.write(&[header])?;
        writer.align()?;

        writer.write(&self.range_threshold.to_ne_bytes())?;
        writer.align()?;

        for (_, s) in self.strings {
            writer.write(&[s])?;
        }
        writer.align()?;

        for f in self.files {
            writer.write(&[f])?;
        }
        writer.align()?;

        for f in self.functions {
            writer.write(&[f])?;
        }
        writer.align()?;

        for s in self.source_locations {
            writer.write(&[s])?;
        }
        for s in self.ranges.values() {
            writer.write(std::slice::from_ref(s))?;
        }
        writer.align()?;

        for r in self.ranges.keys() {
            writer.write(&[raw::Range(*r)])?;
        }
        writer.align()?;

        writer.write(&self.string_bytes)?;

        Ok(Stats {})
    }
}

struct WriteWrapper<W> {
    writer: W,
    position: usize,
}

impl<W: Write> WriteWrapper<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            position: 0,
        }
    }

    fn write<T>(&mut self, data: &[T]) -> std::io::Result<usize> {
        let pointer = data.as_ptr() as *const u8;
        let len = std::mem::size_of_val(data);
        // SAFETY: both pointer and len are derived directly from data/T and are valid.
        let buf = unsafe { std::slice::from_raw_parts(pointer, len) };
        self.writer.write_all(buf)?;
        self.position += len;
        Ok(len)
    }

    fn align(&mut self) -> std::io::Result<usize> {
        let buf = &[0u8; 7];
        let len = raw::align_to_eight(self.position);
        self.write(&buf[0..len])
    }
}

/// Some statistics about the finished/serialized SymCache.
#[derive(Debug)]
#[non_exhaustive]
pub struct Stats {}

/// Errors than can happen during SymCache serialization.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SerializeError {}
