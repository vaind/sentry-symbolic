use std::ops::Range;

use object::{Object, ObjectSection};
use symbolic_symcache::SymCache;

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
        if let (Some(fun), Some(loc)) = (frame.function, frame.location) {
            let name = fun.raw_name()?.into();
            let file = loc.file.unwrap_or_default().to_string();
            let line = loc.line.unwrap_or_default();

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
        let file = frame.path();
        let line = frame.line();

        result.push(LookupFrame { name, file, line });
    }

    Ok(LookupResult { frames: result })
}
