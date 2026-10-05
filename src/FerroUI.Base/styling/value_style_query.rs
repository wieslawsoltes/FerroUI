use super::StyleQuery;

/// The state shared by queries that compare a value with an argument.
pub(crate) struct ValueStyleQuery<T> {
    previous: Option<StyleQuery>,
    argument: T,
}

impl<T> ValueStyleQuery<T> {
    pub fn new(previous: Option<StyleQuery>, argument: T) -> Self {
        Self { previous, argument }
    }

    pub fn argument(&self) -> &T {
        &self.argument
    }

    pub fn previous(&self) -> Option<&StyleQuery> {
        self.previous.as_ref()
    }
}
