/// Validates the items added to a [`FerroList`](super::FerroList).
///
/// Internal to the framework in the managed original; public here so that
/// the classes of other crates can validate the items of their lists.
pub trait IFerroListItemValidator<T> {
    /// Validates an item before it is added; rejects it by panicking.
    fn validate(&self, item: &T);
}
