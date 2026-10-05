/// Represents a split result that can be either a single object or two objects.
#[derive(Clone, Debug)]
pub struct SplitResult<T> {
    /// The first part of the split result.
    pub first: Option<T>,
    /// The second part of the split result.
    pub second: Option<T>,
}

impl<T> SplitResult<T> {
    pub fn new(first: Option<T>, second: Option<T>) -> Self {
        Self { first, second }
    }

    /// Deconstructs the split result into its components.
    pub fn deconstruct(self) -> (Option<T>, Option<T>) {
        (self.first, self.second)
    }
}
