use std::path::Path;
use std::string::String;
use std::{borrow, fs};

use object::{Object, ObjectSection};

use symbolic_symcache_new::*;

#[test]
fn works_on_simple() {
    let buf = create_symcache("tests/fixtures/simple.debug").unwrap();
    let symcache = Format::parse(&buf).unwrap();

    assert_eq!(&resolve_lookup(&symcache, 0x10ef), &[]);

    assert_eq!(
        &resolve_lookup(&symcache, 0x10f2),
        &[ResolvedFrame {
            function: "simple_fn".into(),
            file: "/root-comp-dir/simple.rs".into(),
            line: 5
        }]
    );

    // TODO: assert that we can resolve non-DWARF symbols
}

fn create_symcache(file: impl AsRef<Path>) -> Result<Vec<u8>> {
    with_loaded_dwarf(file.as_ref(), |dwarf| {
        let mut converter = Converter::new();
        converter.process_dwarf(dwarf, |err| panic!("{}", err));

        let mut buf = vec![];
        converter.serialize(&mut buf, |err| panic!("{}", err))?;

        Ok(buf)
    })
}

pub fn resolve_lookup(symcache: &Format<'_>, addr: u64) -> Vec<ResolvedFrame> {
    let mut lookup = symcache.lookup(addr);
    let mut resolved = vec![];

    while let Some(frame) = lookup.next().unwrap() {
        resolved.push(ResolvedFrame::from(frame));
    }

    resolved
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

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

type Dwarf<'a> = gimli::Dwarf<gimli::EndianSlice<'a, gimli::RunTimeEndian>>;

fn with_loaded_dwarf<T, F: FnOnce(&Dwarf) -> Result<T>>(path: &Path, f: F) -> Result<T> {
    let file = fs::File::open(&path).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
    let object = object::File::parse(mmap.as_ref())?;

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
