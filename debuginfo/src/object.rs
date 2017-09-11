use std::path::Path;
use std::borrow::Cow;
use std::io::Cursor;

use uuid;
use goblin;

use dwarf::{DwarfSection, DwarfSectionData};
use symbolic_common::{Arch, ByteView, ErrorKind, Result};

enum FatObjectKind<'a> {
    Elf(goblin::elf::Elf<'a>),
    MachO(goblin::mach::Mach<'a>),
}

enum ObjectTarget<'a> {
    Elf(&'a goblin::elf::Elf<'a>),
    MachOSingle(&'a goblin::mach::MachO<'a>),
    MachOFat(goblin::mach::fat::FatArch, goblin::mach::MachO<'a>),
}

pub struct Object<'a> {
    fat_object: &'a FatObject<'a>,
    arch: Arch,
    target: ObjectTarget<'a>,
}

impl<'a> Object<'a> {
    /// Returns the UUID of the object
    pub fn uuid(&self) -> Option<&uuid::Uuid> {
        None
    }

    /// Returns the architecture of the object
    pub fn arch(&self) -> Arch {
        self.arch
    }

    /// Returns the object name of the object
    pub fn object_name(&self) -> Option<&str> {
        None
    }

    /// Returns the content of the object as bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self.target {
            ObjectTarget::Elf(..) => self.fat_object.as_bytes(),
            ObjectTarget::MachOSingle(macho) => self.fat_object.as_bytes(),
            ObjectTarget::MachOFat(ref arch, ref macho) => {
                let bytes = self.fat_object.as_bytes();
                &bytes[arch.offset as usize..(arch.offset + arch.size) as usize]
            }
        }
    }

    /// Loads a specific dwarf section if its in the file.
    pub fn get_dwarf_section(&self, sect: DwarfSection) -> Option<DwarfSectionData> {
        match self.target {
            // XXX: implement me
            ObjectTarget::Elf(..) => None,
            ObjectTarget::MachOSingle(..) => None,
            ObjectTarget::MachOFat(..) => None,
        }
    }
}

/// Represents an object file.
pub struct FatObject<'a> {
    byteview: &'a ByteView<'a>,
    kind: FatObjectKind<'a>,
}

impl<'a> FatObject<'a> {
    /// Provides a view to an object file from a byteview.
    pub fn new(byteview: &'a ByteView<'a>) -> Result<FatObject<'a>> {
        let kind = {
            let buf = &byteview;
            let mut cur = Cursor::new(buf);
            match goblin::peek(&mut cur)? {
                goblin::Hint::Elf(_) => FatObjectKind::Elf(goblin::elf::Elf::parse(buf)?),
                goblin::Hint::Mach(_) => FatObjectKind::MachO(goblin::mach::Mach::parse(buf)?),
                goblin::Hint::MachFat(_) => FatObjectKind::MachO(goblin::mach::Mach::parse(buf)?),
                _ => {
                    return Err(ErrorKind::UnsupportedObjectFile.into());
                }
            }
        };
        Ok(FatObject {
            byteview: byteview,
            kind: kind,
        })
    }

    /// Returns the contents as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.byteview
    }

    /// Returns a list of variants.
    pub fn objects(&'a self) -> Result<Vec<Object<'a>>> {
        let mut rv = vec![];
        match self.kind {
            FatObjectKind::Elf(ref elf) => {
                rv.push(Object {
                    fat_object: self,
                    arch: Arch::X86_64, // TODO(ja): fix me
                    target: ObjectTarget::Elf(elf),
                });
            }
            FatObjectKind::MachO(ref mach) => match *mach {
                goblin::mach::Mach::Fat(ref fat) => {
                    for (idx, arch) in fat.iter_arches().enumerate() {
                        let arch = arch?;
                        rv.push(Object {
                            fat_object: self,
                            arch: Arch::from_mach(arch.cputype as u32, arch.cpusubtype as u32)?,
                            target: ObjectTarget::MachOFat(arch, fat.get(idx)?),
                        });
                    }
                }
                goblin::mach::Mach::Binary(ref macho) => {
                    rv.push(Object {
                        fat_object: self,
                        arch: Arch::from_mach(
                            macho.header.cputype as u32,
                            macho.header.cpusubtype as u32,
                        )?,
                        target: ObjectTarget::MachOSingle(macho),
                    });
                }
            },
        }
        Ok(rv)
    }
}
