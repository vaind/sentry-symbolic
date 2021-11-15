use std::collections::HashMap;

use symbolic_debuginfo::breakpad::{BreakpadError, BreakpadObject};

use super::*;
use crate::ErrorSink;

impl Converter {
    /// Processes the given [`BreakpadObject`].
    ///
    /// This feeds any errors that were raised during processing into the given [`ErrorSink`].
    /// Currently, the first encountered error will cause processing to abort, but
    /// more fine grained errors may be raised in the future.
    pub fn process_breakpad<E: ErrorSink<BreakpadError>>(
        &mut self,
        breakpad: &BreakpadObject<'_>,
        mut error_sink: E,
    ) {
        let _ = self
            .process_breakpad_internal(breakpad)
            .map_err(|e| error_sink.raise_error(e));
    }

    fn process_breakpad_internal(
        &mut self,
        breakpad: &BreakpadObject<'_>,
    ) -> Result<(), BreakpadError> {
        let mut file_map = HashMap::new();
        // gather files
        for file in breakpad.file_records() {
            let file_record = file?;
            let file_idx = self.insert_file(file_record.name, None, None);
            file_map.insert(file_record.id, file_idx as u32);
        }

        // gather functions
        for function in breakpad.func_records() {
            let func_record = function?;
            let func_idx = self.insert_function(func_record.name);

            for line in func_record.lines() {
                let line_record = line?;
                let source_location = raw::SourceLocation {
                    file_idx: file_map[&line_record.file_id],
                    line: line_record.line as u32,
                    function_idx: func_idx as u32,
                    inlined_into_idx: u32::MAX,
                };

                self.ranges
                    .insert(line_record.address as u32, source_location);
            }
        }
        Ok(())
    }
}
