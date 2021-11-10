use std::vec;
use std::{borrow, ops::Range};

use object::{Object, ObjectSection};
use symbolic_symcache::SymCache;

use crate::converter::Converter;

const SHF_EXECINSTR: u64 = 0x4;

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
pub struct LookupResult {
    pub frames: Vec<LookupFrame>,
}

#[derive(Debug, PartialEq)]
pub struct LookupFrame {
    pub name: String,
    pub file: String,
    pub line: u32,
}

pub fn create_addr2line(
    data: &[u8],
) -> Result<addr2line::ObjectContext, Box<dyn std::error::Error>> {
    let object = object::File::parse(data)?;
    Ok(addr2line::Context::new(&object)?)
}

pub fn lookup_addr2line<R: gimli::Reader>(
    ctx: &addr2line::Context<R>,
    addr: u64,
) -> Result<LookupResult, gimli::Error> {
    let mut frames = ctx.find_frames(addr)?;

    let mut result = vec![];

    while let Some(frame) = frames.next()? {
        // TODO: return just the name with an empty file/line if there is no location
        if let (Some(fun), loc) = (frame.function, frame.location.as_ref()) {
            let name = fun.raw_name()?.into();
            // strip leading `./` to be in-line with symcache output
            let file = loc
                .and_then(|loc| loc.file)
                .map(|f| f.strip_prefix("./").unwrap_or(f))
                .unwrap_or_default()
                .to_string();
            let line = loc.and_then(|loc| loc.line).unwrap_or_default();

            result.push(LookupFrame { name, file, line });
        }
    }

    Ok(LookupResult { frames: result })
}

pub fn create_symcache(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let object = symbolic_debuginfo::elf::ElfObject::parse(data)?;
    let mut symcache_buf = vec![];
    symbolic_symcache::SymCacheWriter::write_object(
        &object,
        std::io::Cursor::new(&mut symcache_buf),
    )?;

    Ok(symcache_buf)
}

pub fn lookup_symcache(
    symcache: &SymCache,
    addr: u64,
) -> Result<LookupResult, Box<dyn std::error::Error>> {
    let frames = symcache.lookup(addr)?;

    let mut result = vec![];

    for frame in frames {
        let frame = frame?;

        let name = frame.function_name().into_string();
        let file = frame.abs_path();
        let line = frame.line();

        result.push(LookupFrame { name, file, line });
    }

    Ok(LookupResult { frames: result })
}

pub fn create_new_symcache(data: &[u8]) -> Result<Converter, Box<dyn std::error::Error>> {
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

    let mut converter = Converter::new();
    converter.process_dwarf(&dwarf, |_| ());

    Ok(converter)
}

pub fn lookup_new_symcache(
    converter: &Converter,
    addr: u64,
) -> Result<LookupResult, Box<dyn std::error::Error>> {
    let frames = vec![];
    // let frames = converter
    //     .lookup(addr as u32)
    //     .map(|source_location| {
    //         let name = source_location.function_name().into();
    //         let file = symbolic_common::join_path(
    //             source_location.directory().unwrap_or(""),
    //             source_location.path_name(),
    //         );
    //         let line = source_location.line();
    //         LookupFrame { name, file, line }
    //     })
    //     .collect();

    Ok(LookupResult { frames })
}
