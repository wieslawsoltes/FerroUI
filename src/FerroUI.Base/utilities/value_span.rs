/// Pairing of value and positions sharing that value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueSpan<T> {
    start: i32,
    length: i32,
    value: T,
}

impl<T> ValueSpan<T> {
    pub const fn new(start: i32, length: i32, value: T) -> Self {
        Self { start, length, value }
    }

    /// Gets the start of the span.
    #[inline]
    pub const fn start(&self) -> i32 {
        self.start
    }

    /// Gets the length of the span.
    #[inline]
    pub const fn length(&self) -> i32 {
        self.length
    }

    /// Gets the value of the span.
    #[inline]
    pub const fn value(&self) -> &T {
        &self.value
    }
}
