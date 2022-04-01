//! Parser for the Usym format.
//!
//! This format can map il2cpp instruction addresses to managed file names and line numbers.

use std::borrow::Cow;
use std::mem;
use std::ptr;

use anyhow::{Error, Result};

/// The raw C structures.
mod raw {

    /// The header of the usym file format.
    #[derive(Debug, Clone)]
    #[repr(C)]
    pub(super) struct Header {
        /// Magic number identifying the file, `b"usym"`.
        pub(super) magic: u32,

        /// Version of the usym file format.
        pub(super) version: u32,

        /// Number of [`UsymRecord`] entries.
        ///
        /// These follow right after the header, and after them is the string table.
        pub(super) record_count: u32,

        /// UUID of the assembly, offset into string table.
        pub(super) id: u32,

        /// Name of the "assembly", offset into string table.
        pub(super) name: u32,

        /// Name of OS, offset into string table.
        pub(super) os: u32,

        /// Name of architecture, offset into string table.
        pub(super) arch: u32,
    }

    /// A record mapping an IL2CPP instruction address to managed code location.
    ///
    /// This is the raw record as it appears in the file, see [`UsymRecord`] for a record with
    /// the names resolved.
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub(super) struct SourceRecord {
        /// Instruction pointer address, relative to base address of assembly.
        pub(super) address: u64,
        /// Managed symbol name as offset in string table.
        pub(super) symbol: u32,
        /// Reference to the managed source file name in the string table.
        pub(super) file: u32,
        /// Managed line number.
        pub(super) line: u32,
        // These might not even be u64, it's just 128 bits we don't know.
        _unknown0: u64,
        _unknown1: u64,
    }
}

/// A record mapping an IL2CPP instruction address to managed code location.
#[derive(Debug, Clone)]
pub struct UsymSourceRecord<'a> {
    /// Instruction pointer address, relative to the base of the assembly.
    pub address: u64,
    /// Symbol name of the managed code.
    pub symbol: Cow<'a, str>,
    /// File name of the managed code.
    pub file: Cow<'a, str>,
    /// Line number of the managed code.
    pub line: u32,
}

/// Reader for the usym symbols file format.
pub struct UsymSymbols<'a> {
    /// File header.
    header: &'a raw::Header,
    /// Instruction address to managed code mapping records.
    records: &'a [raw::SourceRecord],
    /// All the strings.
    ///
    /// This is not a traditional string table but rather a large slice of bytes with
    /// length-prefixed strings, the length is a little-endian u16.  The header and records
    /// refer to strings by byte offsets into this slice of bytes, which must fall on the
    /// the length prefixed part of the string.
    strings: &'a [u8],
    /// The ID of the assembly.
    id: &'a str,
    /// The name of the assembly.
    name: &'a str,
    /// The operating system.
    os: &'a str,
    /// The architecture.
    arch: &'a str,
}

impl<'a> UsymSymbols<'a> {
    const MAGIC: &'static [u8] = b"usym";

    /// Parse a usym file.
    ///
    /// # Panics
    ///
    /// If `std::mem::size_of::<usize>()` is smaller than `std::mem::size_of::<u32>()` on
    /// the machine being run on.
    pub fn parse(buf: &'a [u8]) -> Result<UsymSymbols<'a>> {
        if buf.as_ptr().align_offset(8) != 0 {
            return Err(Error::msg("Data buffer not aligned to 8 bytes"));
        }
        if buf.len() < mem::size_of::<raw::Header>() {
            return Err(Error::msg("Data smaller than UsymHeader"));
        }
        if buf.get(..4) != Some(Self::MAGIC) {
            return Err(Error::msg("Wrong magic number"));
        }

        // SAFETY: We checked the buffer is large enough above.
        let header = unsafe { &*(buf.as_ptr() as *const raw::Header) };
        if header.version != 2 {
            return Err(Error::msg("Unknown version"));
        }

        let record_count: usize = header.record_count.try_into()?;
        let strings_offset =
            mem::size_of::<raw::Header>() + record_count * mem::size_of::<raw::SourceRecord>();
        if buf.len() < strings_offset {
            return Err(Error::msg("Data smaller than number of records"));
        }

        // SAFETY: We checked the buffer is at least the size_of::<UsymHeader>() above.
        let first_record_ptr = unsafe { buf.as_ptr().add(mem::size_of::<raw::Header>()) };

        // SAFETY: We checked the buffer has enough space for all the line records above.
        let records = unsafe {
            let first_record_ptr: *const raw::SourceRecord = first_record_ptr.cast();
            let records_ptr = ptr::slice_from_raw_parts(first_record_ptr, record_count);
            records_ptr
                .as_ref()
                .ok_or_else(|| Error::msg("lines_offset was null pointer!"))
        }?;

        let strings = buf
            .get(strings_offset..)
            .ok_or_else(|| Error::msg("No strings data found"))?;

        let id = match Self::get_string_from_offset(strings, header.id.try_into().unwrap())
            .ok_or_else(|| Error::msg("No assembly ID found"))?
        {
            Cow::Borrowed(id) => id,
            Cow::Owned(_) => return Err(Error::msg("Assembly ID not UTF-8")),
        };
        let name = match Self::get_string_from_offset(strings, header.name.try_into().unwrap())
            .ok_or_else(|| Error::msg("No assembly name found"))?
        {
            Cow::Borrowed(name) => name,
            Cow::Owned(_) => return Err(Error::msg("Assembly name not UTF-8")),
        };
        let os = match Self::get_string_from_offset(strings, header.os.try_into().unwrap())
            .ok_or_else(|| Error::msg("No OS name found"))?
        {
            Cow::Borrowed(name) => name,
            Cow::Owned(_) => return Err(Error::msg("OS name not UTF-8")),
        };
        let arch = match Self::get_string_from_offset(strings, header.arch.try_into().unwrap())
            .ok_or_else(|| Error::msg("No arch name found"))?
        {
            Cow::Borrowed(name) => name,
            Cow::Owned(_) => return Err(Error::msg("Arch name not UTF-8")),
        };

        Ok(Self {
            header,
            records,
            strings,
            id,
            name,
            os,
            arch,
        })
    }

    /// Returns the version of the usym file these symbols were read from.
    pub fn version(&self) -> u32 {
        self.header.version
    }

    fn get_string_from_offset(data: &[u8], offset: usize) -> Option<Cow<str>> {
        let size_bytes = data.get(offset..offset + 2)?;
        let size: usize = u16::from_le_bytes([size_bytes[0], size_bytes[1]]).into();

        let start_offset = offset + 2;
        let end_offset = start_offset + size;

        let string_bytes = data.get(start_offset..end_offset)?;
        Some(String::from_utf8_lossy(string_bytes))
    }

    /// Returns a string from the string table at given offset.
    ///
    /// Offsets are as provided by some [`UsymLiteHeader`] and [`UsymLiteLine`] fields.
    fn get_string(&self, offset: usize) -> Option<Cow<'a, str>> {
        Self::get_string_from_offset(self.strings, offset)
    }

    /// The ID of the assembly.
    ///
    /// This should match the ID of the debug symbols.
    // TODO: Consider making this return debugid::DebugId
    pub fn id(&self) -> &str {
        self.id
    }

    /// The name of the assembly.
    pub fn name(&self) -> &str {
        self.name
    }

    /// The Operating System name.
    pub fn os(&self) -> &str {
        self.os
    }

    /// The architecture name.
    pub fn arch(&self) -> &str {
        self.arch
    }

    /// Returns a [`UsymSourceRecord`] at the given index it was stored.
    ///
    /// Not that useful, you have no idea what index you want.
    pub fn get_record(&self, index: usize) -> Option<UsymSourceRecord> {
        let raw = self.records.get(index)?;
        Some(UsymSourceRecord {
            address: raw.address,
            symbol: self.get_string(raw.symbol.try_into().unwrap())?,
            file: self.get_string(raw.file.try_into().unwrap())?,
            line: raw.line,
        })
    }

    /// Lookup the managed code source location for an IL2CPP instruction pointer.
    pub fn lookup_source_record(&self, ip: u64) -> Option<UsymSourceRecord> {
        // TODO: need to subtract the image base to get relative address
        match self.records.binary_search_by_key(&ip, |r| r.address) {
            Ok(index) => self.get_record(index),
            Err(index) => self.get_record(index - 1),
        }
    }

    // TODO: Add iterator over records?
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use symbolic_common::ByteView;
    use symbolic_testutils::fixture;

    use super::*;

    #[test]
    fn test_write_usym() {
        // Not really a test but rather a quick and dirty way to write a small usym file
        // given a large one.  This was used to generate a small enough usym file to use as
        // a test fixture, however this still tests the reader and writer can round-trip.

        // let file = File::open(
        //     "/Users/flub/code/sentry-unity-il2cpp-line-numbers/Builds/iOS/UnityFramework.usym",
        // )
        // .unwrap();
        let file = File::open(fixture("il2cpp/artificial.usym")).unwrap();

        let orig_data = ByteView::map_file_ref(&file).unwrap();
        let usyms = UsymSymbols::parse(&orig_data).unwrap();

        // Our strings and helper to build it by pushing new strings.  We keep strings and
        // strings_offsets so we can de-duplicate, raw_strings is the thing we are really
        // building.
        let mut strings: Vec<String> = Vec::new();
        let mut raw_strings: Vec<u8> = Vec::new();
        let mut string_offsets: Vec<u64> = Vec::new();

        let mut push_string = |s: Cow<'_, str>| match strings.iter().position(|i| i == s.as_ref()) {
            Some(pos) => string_offsets[pos],
            None => {
                let offset = raw_strings.len() as u64;
                let len = s.len() as u16;
                raw_strings.extend_from_slice(&len.to_le_bytes());
                raw_strings.extend_from_slice(s.as_bytes());

                strings.push(s.to_string());
                string_offsets.push(offset);

                offset
            }
        };

        // Construct new header.
        let mut header = usyms.header.clone();
        header.id = push_string(usyms.get_string(header.id as usize).unwrap()) as u32;
        header.name = push_string(usyms.get_string(header.name as usize).unwrap()) as u32;
        header.os = push_string(usyms.get_string(header.os as usize).unwrap()) as u32;
        header.arch = push_string(usyms.get_string(header.arch as usize).unwrap()) as u32;

        // Construct new records.
        header.record_count = 5;
        let mut records = Vec::new();
        for mut record in usyms.records.iter().cloned().take(5) {
            record.symbol = push_string(usyms.get_string(record.symbol as usize).unwrap()) as u32;
            record.file = push_string(usyms.get_string(record.file as usize).unwrap()) as u32;
            records.push(record);
        }

        // let mut dest = File::create(
        //     "/Users/flub/code/symbolic/symbolic-testutils/fixtures/il2cpp/artificial.usym",
        // )
        // .unwrap();
        let mut dest = Vec::new();

        // Write the header.
        let data = &[header];
        let ptr = data.as_ptr() as *const u8;
        let len = std::mem::size_of_val(data);
        let buf = unsafe { std::slice::from_raw_parts(ptr, len) };
        dest.write_all(buf).unwrap();

        // Write the records.
        let ptr = records.as_ptr() as *const u8;
        let len = records.len() * std::mem::size_of::<raw::SourceRecord>();
        let buf = unsafe { std::slice::from_raw_parts(ptr, len) };
        dest.write_all(buf).unwrap();

        // Write the strings.
        dest.write_all(&raw_strings).unwrap();

        assert_eq!(orig_data.as_ref(), dest);
    }

    #[test]
    fn test_basic() {
        let file = File::open(fixture("il2cpp/artificial.usym")).unwrap();
        let data = ByteView::map_file_ref(&file).unwrap();
        let usyms = UsymSymbols::parse(&data).unwrap();

        assert_eq!(usyms.version(), 2);
        assert_eq!(usyms.id(), "153d10d10db033d6aacda4e1948da97b");
        assert_eq!(usyms.name(), "UnityFramework");
        assert_eq!(usyms.os(), "mac");
        assert_eq!(usyms.arch(), "arm64");
    }

    #[test]
    fn test_sorted_addresses() {
        let file = File::open(fixture("il2cpp/artificial.usym")).unwrap();
        let data = ByteView::map_file_ref(&file).unwrap();
        let usyms = UsymSymbols::parse(&data).unwrap();

        let mut last_address = usyms.records[0].address;
        for i in 1..usyms.header.record_count as usize {
            assert!(usyms.records[i].address > last_address);
            last_address = usyms.records[i].address;
        }
    }
}
