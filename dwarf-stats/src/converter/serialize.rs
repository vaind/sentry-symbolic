use std::io::Write;

use thiserror::Error;

use super::error::ErrorSink;
use super::*;
use crate::format::raw;

impl Converter {
    pub fn serialize<W: Write, E: ErrorSink<SerializeError>>(
        mut self,
        writer: &mut W,
        mut error_sink: E,
    ) -> std::io::Result<Stats> {
        let mut writer = WriteWrapper::new(writer);

        let ranges = std::mem::take(&mut self.ranges);
        let ranges: Vec<_> = ranges
            .into_iter()
            .filter_map(|(addr, source_location)| {
                // TODO: filter out invalid/incomplete source locations
                self.insert_source_location(source_location);
                Some(addr)
            })
            .collect();

        // TODO: check that these are < u32::MAX and raise an error in that case

        let num_strings = self.strings.len() as u32;
        let num_files = self.files.len() as u32;
        let num_functions = self.functions.len() as u32;
        let num_source_locations = self.source_locations.len() as u32;
        let num_ranges = ranges.len() as u32;
        let string_bytes = self.string_bytes.len() as u32;

        let header = raw::Header {
            num_strings,
            num_files,
            num_functions,
            num_source_locations,
            num_ranges,
            string_bytes,
        };

        writer.write(&[header])?;
        writer.align()?;

        for (_, s) in self.strings {
            writer.write(&[raw::String {
                string_offset: s.string_offset,
                string_len: s.string_len,
            }])?;
        }
        writer.align()?;

        for f in self.files {
            writer.write(&[raw::File {
                comp_dir_idx: u32::MAX,
                directory_idx: f.directory_idx.unwrap_or(u32::MAX),
                path_name_idx: f.path_name_idx,
            }])?;
        }
        writer.align()?;

        for f in self.functions {
            writer.write(&[raw::Function {
                name_idx: f.name_idx,
            }])?;
        }
        writer.align()?;

        for s in self.source_locations {
            writer.write(&[raw::SourceLocation {
                file_idx: s.file_idx,
                line: s.line,
                function_idx: s.function_idx,
                inlined_into_idx: s.inlined_into_idx.unwrap_or(u32::MAX),
            }])?;
        }
        writer.align()?;

        writer.write(&ranges)?;
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

#[derive(Debug)]
#[non_exhaustive]
pub struct Stats {}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SerializeError {}
