//! `TransformAsyncOptions` (`Cache/TransformAsyncOptions.cs`).

/// The options of `TransformAsync` and `TransformSafeAsync` (`record struct
/// TransformAsyncOptions(int? MaximumConcurrency, bool TransformOnRefresh)`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TransformAsyncOptions {
    /// The greatest number of transforms in flight at once; `None` for no limit
    /// (`MaximumConcurrency`).
    pub maximum_concurrency: Option<usize>,
    /// Whether a refresh transforms the item again (`TransformOnRefresh`).
    pub transform_on_refresh: bool,
}

impl TransformAsyncOptions {
    /// No limit of concurrency, and a refresh is forwarded without a new transform (`Default`).
    pub const DEFAULT: TransformAsyncOptions = TransformAsyncOptions { maximum_concurrency: None, transform_on_refresh: false };

    /// The options of `maximum_concurrency` and `transform_on_refresh`
    /// (`TransformAsyncOptions(int?, bool)`).
    pub const fn new(maximum_concurrency: Option<usize>, transform_on_refresh: bool) -> Self {
        Self { maximum_concurrency, transform_on_refresh }
    }
}
