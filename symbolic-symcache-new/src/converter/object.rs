use std::collections::btree_map;

use symbolic_debuginfo::{DebugSession, Function, ObjectLike};
use symbolic_symcache::{SymCacheError, SymCacheErrorKind};

use super::*;

impl Converter {
    ///
    pub fn process_object<'d, 'o, O>(&mut self, object: &'o O) -> Result<(), SymCacheError>
    where
        O: ObjectLike<'d, 'o>,
        O::Error: std::error::Error + Send + Sync + 'static,
    {
        let session = object
            .debug_session()
            .map_err(|e| SymCacheError::new(SymCacheErrorKind::BadDebugFile, e))?;

        for function in session.functions() {
            let function =
                function.map_err(|e| SymCacheError::new(SymCacheErrorKind::BadDebugFile, e))?;

            self.process_symbolic_function(&function);
        }

        for symbol in object.symbols() {
            let name = match symbol.name {
                Some(ref name) => name.as_ref(),
                None => continue,
            };

            let name_idx = self.insert_string(name);

            match self.ranges.entry(symbol.address as u32) {
                btree_map::Entry::Vacant(entry) => {
                    let function = raw::Function {
                        name_idx,
                        entry_pc: symbol.address as u32,
                        lang: u8::MAX,
                    };
                    let function_idx = self.functions.insert_full(function).0 as u32;

                    entry.insert(raw::SourceLocation {
                        file_idx: u32::MAX,
                        line: 0,
                        function_idx,
                        inlined_into_idx: u32::MAX,
                    });
                }
                btree_map::Entry::Occupied(entry) => {
                    // ASSUMPTION:
                    // the `functions` iterator has already filled in this addr via debug session.
                    // we could trace the caller hierarchy up to the root, and assert that it is
                    // indeed the same function, and maybe update its `entry_pc`, but we don’t do
                    // that for now.
                    let _function_idx = entry.get().function_idx as usize;
                }
            }
        }

        Ok(())
    }

    fn process_symbolic_function(&mut self, function: &Function<'_>) {
        let comp_dir = std::str::from_utf8(function.compilation_dir).ok();

        let entry_pc = if function.inline {
            u32::MAX
        } else {
            function.address as u32
        };
        let function_idx =
            self.insert_function(function.name.as_str(), entry_pc, function.name.language());

        for line in &function.lines {
            let path_name = line.file.name_str();
            let file_idx = self.insert_file(&path_name, Some(&line.file.dir_str()), comp_dir);

            let source_location = raw::SourceLocation {
                file_idx,
                line: line.line as u32,
                function_idx,
                inlined_into_idx: u32::MAX,
            };

            match self.ranges.entry(line.address as u32) {
                btree_map::Entry::Vacant(entry) => {
                    if function.inline {
                        // BUG:
                        // the abstraction should have defined this line record inside the caller
                        // function already!
                    }
                    entry.insert(source_location);
                }
                btree_map::Entry::Occupied(mut entry) => {
                    if function.inline {
                        let caller_source_location = entry.get().clone();

                        let mut callee_source_location = source_location;
                        let (inlined_into_idx, _) =
                            self.source_locations.insert_full(caller_source_location);

                        callee_source_location.inlined_into_idx = inlined_into_idx as u32;
                        entry.insert(callee_source_location);
                    } else {
                        // BUG:
                        // the abstraction yields multiple top-level functions for the same
                        // instruction addr
                        entry.insert(source_location);
                    }
                }
            }
        }

        for inlinee in &function.inlinees {
            self.process_symbolic_function(inlinee);
        }
    }
}
