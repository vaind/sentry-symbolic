use std::collections::{hash_map::Entry, HashMap};

#[derive(Debug, Default)]
struct NewSymCache {
    files: Vec<File>,
    functions: Vec<Function>,
    ranges: Vec<InstrRange>,
    source_locations: Vec<InternalSourceLocation>,

    // internal lookup tables
    file_lookup: HashMap<String, u32>,
    function_lookup: HashMap<String, u32>,
}

#[derive(Debug)]
struct File {
    name: String,
}

#[derive(Debug)]
struct Function {
    name: String,
}

#[derive(Debug)]
struct InstrRange {
    start: u32,
    //end: u32,
    source_location: u32,
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
        range: std::ops::Range<usize>,
    },
    InlinedSubroutine {
        function: String,
        range: std::ops::Range<usize>,
        call_file: String,
        call_line: u32,
    },
}

impl NewSymCache {
    pub fn construct(line_program: &[LineProgramRow], dwarf_dies: &[DwarfDie]) -> Self {
        let mut slf = Self::default();

        // first, map all line_program rows into our range structure
        for line_program_row in line_program {
            let file_idx = slf.add_file(line_program_row.file.clone());

            let source_location_idx = slf.source_locations.len() as u32;
            slf.source_locations.push(InternalSourceLocation {
                file: file_idx,
                line: line_program_row.line,
                function: 0,
                inlined_into: None,
            });
            slf.ranges.push(InstrRange {
                start: line_program_row.addr,
                source_location: source_location_idx,
            });
        }

        // next, walk all (inline) functions, and mutate our range structure accordingly
        for fun in dwarf_dies {
            match fun {
                DwarfDie::Subprogram { function, range } => {
                    let fun_idx = slf.add_function(function.clone());
                    for range in slf.ranges.get_mut(range.clone()).unwrap() {
                        let source_location =
                            &mut slf.source_locations[range.source_location as usize];
                        source_location.function = fun_idx;
                    }
                }
                DwarfDie::InlinedSubroutine {
                    function,
                    range,
                    call_file,
                    call_line,
                } => {
                    let fun_idx = slf.add_function(function.clone());
                    let file_idx = slf.add_file(call_file.clone());

                    for range in slf.ranges.get_mut(range.clone()).unwrap() {
                        let caller_source_location =
                            &mut slf.source_locations[range.source_location as usize];
                        let mut own_source_location = caller_source_location.clone();
                        own_source_location.function = fun_idx;

                        caller_source_location.file = file_idx;
                        caller_source_location.line = *call_line;

                        own_source_location.inlined_into = Some(range.source_location);

                        let own_source_location_idx = slf.source_locations.len() as u32;
                        slf.source_locations.push(own_source_location);

                        range.source_location = own_source_location_idx;
                    }
                }
            }
        }

        slf
    }

    fn add_file(&mut self, file: String) -> u32 {
        match self.file_lookup.entry(file.clone()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let idx = self.files.len() as u32;
                self.files.push(File { name: file });
                e.insert(idx);
                idx
            }
        }
    }

    fn add_function(&mut self, function: String) -> u32 {
        match self.function_lookup.entry(function.clone()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let idx = self.functions.len() as u32;
                self.functions.push(Function {
                    name: function.clone(),
                });
                e.insert(idx);
                idx
            }
        }
    }

    pub fn lookup(&self, addr: u32) -> SourceLocationIter<'_> {
        let range_idx = self
            .ranges
            .binary_search_by_key(&addr, |range| range.start)
            .unwrap_or_else(|i| i);

        let range = self.ranges.get(range_idx);

        SourceLocationIter {
            symcache: self,
            source_location_idx: range.map(|range| range.source_location),
        }
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
