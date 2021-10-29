# Example

```
0x0001 - 0x002f
 - `trigger_crash` in file b.c line 12
   -> inlined into `main` in file a.c line 10
0x002f - 0x004a
 - `trigger_crash` in file b.c line 13
   -> inlined into `main` in file a.c line 10

ranges: [{
    start: 0x0001
    end: 0x002f
    source_location: Some(1)
}, {
    start: 0x002f
    end: 0x004a
    source_location: Some(2)
}, {
    start: 0x004a
    end: 0x0084
    source_location: None <- this range has no mapping
}]

source_locations: [{
    file: "a.c"
    line_no: 10
    function: "main"
    inlined_into: None
}, {
    file: "b.c"
    line_no: 12
    function: "trigger_crash"
    inlined_into: Some(0) <- reference to "main"
}, {
    file: "b.c"
    line_no: 13
    function: "trigger_crash"
    inlined_into: Some(0) <- reference to "main"
}]
```

## Napkin math for space usage

```
// sizeof() = 16 byte
struct SourceLocation {
    file: u32, // <- index into array of all files
    line_no: u32,
    function: u32, // <- index into array of all functions
    inlined_into: Option<u32>, // <- index into array of all source_locations
}
// sizeof() = 12 bytes
struct Range {
    start: u32,
    end: u32,
    source_location: u32, // index into array of all source_locations
}
```

Worst case, we have slightly more source_locations (because of inlining) than ranges.

For `electron`, with ~12M of ranges, that means:
(the executable itself is only 105M, DWARF size: 1.5G)

- `(12 + 16 = 28) * 12M = 336M`
- `(8 + 16 = 24) * 12M = 288M`

Observations:

- 99% of line numbers fit in u16
- number of files (probably) fit in u16, but I don’t do cross-CU deduplication for those
- how about function? _how many unique functions exist_ <-
- `inlined_into` could fit in u16, probably if we sort by refcount, so the _referenced_ source locations
  have a low index

-> `(8 + 8 = 16) * 12M = 192M`

## Idea: only save end of range

We assume there are no gaps between ranges.
_If_ there are gaps, the `source_location` should be `None`.

```
start = 1
ranges: [{
    end: 0x002f
}, {
    end: 0x004a
}]
```

## Idea: prefix-sum (offset compression) for instruction ranges:

(probably not worth it?)

```
start = 1
ranges: [{
    offset: 0x2e <- range is: [start .. start + offset] = 0x0001 - 0x002f
}, {
    offset: 0x1b <- range is: 0x002f - [0x002f + 0x1b = 0x004a]
}]
```

# Range

- start
- end
- source_location -> SourceLocation

# SourceLocation

- file -> File
- line_no
- function -> Function
- inlined_into -> SourceLocation

# Function

- name
- start
- ...

# File

- name
- directory

# Lines

Vec<{
instruction addr,
line number,
file index,
}>

range_offsets: Vec<u8>
info_for_offset: Vec<u32>

# Info for a range offset:

0xxxxxx xxxxxxxx yyyyyyyy yyyyyyyy
^ flag for "compressed" record
| x = line number
| y = file index

## "Big Records"

1xxxxxx xxxxxxxx xxxxxxxx xxxxxxxx
^ flag for "big" record
| x = index into "big records"

big_records = Vec<(u32, u32)>
