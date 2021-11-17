use std::vec;
use std::{borrow, ops::Range};

use object::{Object, ObjectSection};
use symbolic_debuginfo::breakpad::BreakpadObject;
use symbolic_symcache::SymCache;

use crate::converter::Converter;
use crate::format;

const SHF_EXECINSTR: u64 = 0x4;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
type Dwarf<'a> = gimli::Dwarf<gimli::EndianSlice<'a, gimli::RunTimeEndian>>;

pub fn get_executable_range(object: &object::File) -> Range<u64> {
    let mut smallest_addr = u64::MAX;
    let mut executable_range = 0;
    for s in object.sections() {
        if let object::SectionFlags::Elf { sh_flags } = s.flags() {
            if sh_flags & SHF_EXECINSTR == SHF_EXECINSTR {
                executable_range += s.size();
                smallest_addr = smallest_addr.min(s.address());
            }
        }
    }
    smallest_addr..smallest_addr + executable_range
}

#[derive(Debug, PartialEq)]
pub struct ResolvedFrame {
    pub function: String,
    pub file: String,
    pub line: u32,
}

impl From<format::SourceLocation<'_>> for ResolvedFrame {
    fn from(source_location: format::SourceLocation<'_>) -> Self {
        let function = source_location
            .function()
            .unwrap()
            .and_then(|function| function.name().unwrap())
            .unwrap_or("")
            .to_owned();
        let file = source_location
            .file()
            .unwrap()
            .and_then(|file| file.full_path().unwrap())
            .unwrap_or_else(String::new);
        let line = source_location.line();

        Self {
            function,
            file,
            line,
        }
    }
}

impl<R: gimli::Reader> From<addr2line::Frame<'_, R>> for ResolvedFrame {
    fn from(frame: addr2line::Frame<'_, R>) -> Self {
        // TODO: return just the name with an empty file/line if there is no location
        let (fun, loc) = (frame.function, frame.location.as_ref());
        let function = fun
            .map(|f| f.raw_name().unwrap().to_string())
            .unwrap_or_default();
        // strip leading `./` to be in-line with symcache output
        let file = loc
            .and_then(|loc| loc.file)
            .map(|f| f.strip_prefix("./").unwrap_or(f))
            .unwrap_or_default()
            .to_string();
        let line = loc.and_then(|loc| loc.line).unwrap_or_default();
        Self {
            function,
            file,
            line,
        }
    }
}

pub fn resolve_lookup(symcache: &format::Format<'_>, addr: u64) -> Vec<ResolvedFrame> {
    let mut lookup = symcache.lookup(addr);
    let mut resolved = vec![];

    while let Some(frame) = lookup.next().unwrap() {
        resolved.push(ResolvedFrame::from(frame));
    }

    resolved
}

pub fn create_addr2line(data: &[u8]) -> Result<addr2line::ObjectContext> {
    let object = object::File::parse(data)?;
    Ok(addr2line::Context::new(&object)?)
}

pub fn lookup_addr2line<R: gimli::Reader>(
    ctx: &addr2line::Context<R>,
    addr: u64,
) -> Result<Vec<ResolvedFrame>> {
    let mut frames = ctx.find_frames(addr)?;

    let mut result = vec![];

    while let Some(frame) = frames.next()? {
        result.push(frame.into());
    }

    Ok(result)
}

pub fn create_symcache(data: &[u8]) -> Result<Vec<u8>> {
    let object = symbolic_debuginfo::elf::ElfObject::parse(data)?;
    let mut symcache_buf = vec![];
    symbolic_symcache::SymCacheWriter::write_object(
        &object,
        std::io::Cursor::new(&mut symcache_buf),
    )?;

    Ok(symcache_buf)
}

pub fn lookup_symcache(symcache: &SymCache, addr: u64) -> Result<Vec<ResolvedFrame>> {
    let frames = symcache.lookup(addr)?;

    let mut result = vec![];

    for frame in frames {
        let frame = frame?;

        let function = frame.function_name().into_string();
        let file = frame.abs_path();
        let line = frame.line();

        result.push(ResolvedFrame {
            function,
            file,
            line,
        });
    }

    Ok(result)
}

fn with_loaded_dwarf<T, F: FnOnce(&Dwarf) -> Result<T>>(data: &[u8], f: F) -> Result<T> {
    let object = object::File::parse(data)?;

    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    // Load a section and return as `Cow<[u8]>`.
    let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>, gimli::Error> {
        match object.section_by_name(id.name()) {
            Some(ref section) => Ok(section
                .uncompressed_data()
                .unwrap_or(borrow::Cow::Borrowed(&[][..]))),
            None => Ok(borrow::Cow::Borrowed(&[][..])),
        }
    };

    // Load all of the sections.
    let dwarf_cow = gimli::Dwarf::load(&load_section)?;

    // Borrow a `Cow<[u8]>` to create an `EndianSlice`.
    let borrow_section: &dyn for<'a> Fn(
        &'a borrow::Cow<[u8]>,
    ) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, endian);

    // Create `EndianSlice`s for all of the sections.
    let dwarf = dwarf_cow.borrow(&borrow_section);

    f(&dwarf)
}

pub fn create_new_symcache_dwarf(data: &[u8]) -> Result<Vec<u8>> {
    with_loaded_dwarf(data, |dwarf| {
        let mut converter = Converter::new();
        converter.process_dwarf(dwarf, |err| panic!("{}", err));

        let mut buf = vec![];
        converter.serialize(&mut buf, |err| panic!("{}", err))?;

        Ok(buf)
    })
}

pub fn create_new_symcache_abstraction(data: &[u8]) -> Result<Vec<u8>> {
    let object = symbolic_debuginfo::Object::parse(data)?;

    let mut converter = Converter::new();
    converter.process_object(&object)?;

    let mut buf = vec![];
    converter.serialize(&mut buf, |err| panic!("{}", err))?;

    Ok(buf)
}

pub fn create_new_symcache_breakpad(data: &[u8]) -> Result<Vec<u8>> {
    let breakpad = BreakpadObject::parse(data)?;

    let mut converter = Converter::default();
    converter.process_breakpad(&breakpad, |_| {});
    let mut symcache_buf = Vec::new();
    converter.serialize(&mut symcache_buf, |_| {})?;

    Ok(symcache_buf)
}

pub fn lookup_new_symcache(
    format: &crate::format::Format,
    addr: u64,
) -> Result<Vec<ResolvedFrame>, Box<dyn std::error::Error>> {
    let mut frames = format.lookup(addr);

    let mut result = vec![];

    while let Some(source_location) = frames.next()? {
        let function = source_location.function()?;
        let file = source_location.file()?;
        let line = source_location.line();

        let function = if let Some(function) = function {
            function.name()?.unwrap_or_default().to_owned()
        } else {
            String::new()
        };
        let file = if let Some(file) = file {
            file.full_path()?.unwrap_or_default()
        } else {
            String::new()
        };

        result.push(ResolvedFrame {
            function,
            file,
            line,
        });
    }

    Ok(result)
}
