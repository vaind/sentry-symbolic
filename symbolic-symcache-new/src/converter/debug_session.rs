use symbolic_debuginfo::DebugSession;

use super::*;
use crate::ErrorSink;

impl Converter {
    pub fn process_debug_session<
        'session,
        S: 'session + DebugSession<'session>,
        E: ErrorSink<S::Error>,
    >(
        &mut self,
        session: S,
        mut error_sink: E,
    ) {
        for function in session.functions() {
            let function = match function {
                Ok(function) => function,
                Err(e) => {
                    error_sink.raise_error(e);
                    continue;
                }
            };

            let entry_pc = function.address;
        }
    }
}
