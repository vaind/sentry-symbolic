#[derive(Debug)]
#[repr(C)]
pub struct Header {
    // TODO: preamble, version
    /// Number of included [`String`]s.
    pub num_strings: u32,
    /// Number of included [`File`]s.
    pub num_files: u32,
    /// Number of included [`Function`]s.
    pub num_functions: u32,
    /// Number of included [`SourceLocation`]s.
    pub num_source_locations: u32,
    /// Number of included [`Range`]s.
    pub num_ranges: u32,
    /// Total number of bytes used for string data.
    pub string_bytes: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct Function {
    /// The functions name (reference to a [`String`]).
    pub name_idx: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct File {
    /// The optional compilation directory prefix (reference to a [`String`]).
    pub comp_dir_idx: u32,
    /// The optional directory prefix (reference to a [`String`]).
    pub directory_idx: u32,
    /// The file path (reference to a [`String`]).
    pub path_name_idx: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct SourceLocation {
    /// The optional source file (reference to a [`File`]).
    pub file_idx: u32,
    /// The line number.
    pub line: u32,
    /// The function (reference to a [`Function`]).
    pub function_idx: u32,
    /// The caller source location in case this location was inlined
    /// (reference to another [`SourceLocation`]).
    pub inlined_into_idx: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct String {
    /// The offset into the `string_bytes`.
    pub string_offset: u32,
    /// Length of the string.
    pub string_len: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct Range(pub u32);
