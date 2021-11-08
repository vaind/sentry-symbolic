use std::error::Error;

pub trait ErrorSink<E: Error> {
    fn raise_error(&mut self, error: E);
}

impl<E: Error, F: FnMut(E)> ErrorSink<E> for F {
    fn raise_error(&mut self, error: E) {
        self(error)
    }
}
