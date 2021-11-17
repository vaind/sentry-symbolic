//! Defines the [`ErrorSink`] trait that is used for lenient parsing.

/// The [`ErrorSink`] is used to raise errors that happen during processing.
///
/// The processing steps themselves are infallible, however errors that happen during processing
/// will be pushed out to the [`ErrorSink`], and it is the responsibility of the user to decide what
/// to do with those errors.
///
/// The idea behind this is to not fail *all* of a file, just because a single reference may be
/// invalid, due to compiler or linker bugs. The assumption is that a debug information file might
/// still contain usable data even if it contains some invalid data.
pub trait ErrorSink<E> {
    /// Raises an intermediate processing error with the [`ErrorSink`].
    fn raise_error(&mut self, error: E);
}

impl<E, F: FnMut(E)> ErrorSink<E> for F {
    fn raise_error(&mut self, error: E) {
        self(error)
    }
}
