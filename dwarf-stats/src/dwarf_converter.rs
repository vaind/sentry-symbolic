use indexmap::set::IndexSet;
use std::collections::BTreeMap;
use std::ops::{Bound, Range};

#[derive(Debug, Default)]
struct NewSymCache {
    files: IndexSet<File>,
    functions: IndexSet<Function>,
    ranges: BTreeMap<u32, u32>,
    source_locations: Vec<InternalSourceLocation>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct File {
    name: String,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Function {
    name: String,
}

#[derive(Debug, Clone)]
struct InternalSourceLocation {
    file: u32,
    line: u32,
    function: u32,
    inlined_into: Option<u32>,
}

#[derive(Debug)]
struct LineProgramRow {
    addr: u32,
    file: String,
    line: u32,
}

enum DwarfDie {
    Subprogram {
        function: String,
        range: std::ops::Range<u32>,
    },
    InlinedSubroutine {
        function: String,
        range: std::ops::Range<u32>,
        call_file: String,
        call_line: u32,
    },
}

impl NewSymCache {
    pub fn construct(line_program: &[LineProgramRow], dwarf_dies: &[DwarfDie]) -> Self {
        let mut slf = Self::default();

        // first, map all line_program rows into our range structure
        for line_program_row in line_program {
            let (file_idx, _) = slf.files.insert_full(File {
                name: line_program_row.file.clone(),
            });

            let source_location_idx = slf.source_locations.len() as u32;
            slf.source_locations.push(InternalSourceLocation {
                file: file_idx as u32,
                line: line_program_row.line,
                function: 0,
                inlined_into: None,
            });
            slf.ranges
                .insert(line_program_row.addr, source_location_idx);
        }

        // next, walk all (inline) functions, and mutate our range structure accordingly
        for fun in dwarf_dies {
            match fun {
                DwarfDie::Subprogram { function, range } => {
                    let (fun_idx, _) = slf.functions.insert_full(Function {
                        name: function.clone(),
                    });
                    for source_location_idx in Self::sub_ranges(&mut slf.ranges, range) {
                        let source_location =
                            &mut slf.source_locations[*source_location_idx as usize];
                        source_location.function = fun_idx as u32;
                    }
                }
                DwarfDie::InlinedSubroutine {
                    function,
                    range,
                    call_file,
                    call_line,
                } => {
                    let (fun_idx, _) = slf.functions.insert_full(Function {
                        name: function.clone(),
                    });
                    let (file_idx, _) = slf.files.insert_full(File {
                        name: call_file.clone(),
                    });

                    for source_location_idx in Self::sub_ranges(&mut slf.ranges, range) {
                        let caller_source_location =
                            &mut slf.source_locations[*source_location_idx as usize];
                        let mut own_source_location = caller_source_location.clone();
                        own_source_location.function = fun_idx as u32;

                        caller_source_location.file = file_idx as u32;
                        caller_source_location.line = *call_line;

                        own_source_location.inlined_into = Some(*source_location_idx);

                        let own_source_location_idx = slf.source_locations.len() as u32;
                        slf.source_locations.push(own_source_location);

                        *source_location_idx = own_source_location_idx;
                    }
                }
            }
        }

        slf
    }

    pub fn lookup(&self, addr: u32) -> SourceLocationIter<'_> {
        let source_location_idx = self
            .ranges
            .range((Bound::Unbounded, Bound::Included(addr)))
            .next_back()
            .map(|(_, idx)| *idx);

        SourceLocationIter {
            symcache: self,
            source_location_idx,
        }
    }

    fn sub_ranges<'a, 'b>(
        ranges: &'a mut BTreeMap<u32, u32>,
        range: &'b Range<u32>,
    ) -> impl Iterator<Item = &'a mut u32> + 'a {
        let first_after = ranges.range(range.end..).next();
        let upper_bound = if let Some((first_after_start, _)) = first_after {
            Bound::Excluded(*first_after_start)
        } else {
            Bound::Unbounded
        };
        let lower_bound = Bound::Included(range.start);
        ranges.range_mut((lower_bound, upper_bound)).map(|(_, v)| v)
    }
}

struct SourceLocationIter<'symcache> {
    symcache: &'symcache NewSymCache,
    source_location_idx: Option<u32>,
}

struct SourceLocationReference<'symcache> {
    symcache: &'symcache NewSymCache,
    source_location_idx: usize,
}

impl<'symcache> std::fmt::Debug for SourceLocationReference<'symcache> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source_location = &self.symcache.source_locations[self.source_location_idx];
        let file = &self.symcache.files[source_location.file as usize];
        let function = &self.symcache.functions[source_location.function as usize];
        let line = source_location.line;

        f.debug_struct("SourceLocationReference")
            .field("function", function)
            .field("file", file)
            .field("line", &line)
            .finish()
    }
}

impl<'symcache> Iterator for SourceLocationIter<'symcache> {
    type Item = SourceLocationReference<'symcache>;

    fn next(&mut self) -> Option<Self::Item> {
        let source_location_idx = self.source_location_idx? as usize;
        let source_location = self.symcache.source_locations.get(source_location_idx)?;

        self.source_location_idx = source_location.inlined_into;
        Some(SourceLocationReference {
            symcache: self.symcache,
            source_location_idx,
        })
    }
}

#[test]
fn how_symcache_should_work() {
    let line_program = &[
        LineProgramRow {
            addr: 0,
            file: "main.c".into(),
            line: 10,
        },
        LineProgramRow {
            addr: 1,
            file: "a.c".into(),
            line: 12,
        },
        LineProgramRow {
            addr: 2,
            file: "b.c".into(),
            line: 14,
        },
    ];
    let dwarf_dies = &[
        DwarfDie::Subprogram {
            function: "main".into(),
            range: 0..3,
        },
        DwarfDie::InlinedSubroutine {
            function: "call_a".into(),
            range: 1..3,
            call_file: "main.c".into(),
            call_line: 11,
        },
        DwarfDie::InlinedSubroutine {
            function: "call_b".into(),
            range: 2..3,
            call_file: "a.c".into(),
            call_line: 13,
        },
    ];

    let symcache = NewSymCache::construct(line_program, dwarf_dies);
    dbg!(&symcache);

    for addr in 0..3 {
        let source_locations: Vec<_> = symcache.lookup(addr).collect();
        dbg!((addr, source_locations));
    }
}
