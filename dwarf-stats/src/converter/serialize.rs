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
        let writer = WriteWrapper::new(writer);
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

    fn write<T>(&mut self, data: &T) -> std::io::Result<usize> {
        let pointer = data as *const T as *const u8;
        let len = std::mem::size_of::<T>();
        // SAFETY: both pointer and len are derived directly from data/T and are valid.
        let buf = unsafe { std::slice::from_raw_parts(pointer, len) };
        self.writer.write_all(buf)?;
        Ok(len)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Stats {}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SerializeError {}
