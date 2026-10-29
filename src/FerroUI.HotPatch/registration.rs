use std::collections::BTreeSet;

/// A stable application declaration, supplied when its patch supervisor starts.
/// Changing registrations is a build-input change and requires process replacement.
#[derive(Clone, Debug)]
pub struct FunctionRegistration {
    pub(crate) source: String,
    pub(crate) function: String,
    pub(crate) views: Option<BTreeSet<String>>,
}
impl FunctionRegistration {
    /// A handler or command whose updated body takes effect on its next invocation.
    /// Register the function here and invoke it through a patch-aware callback helper.
    pub fn callback(source: impl Into<String>, function: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            function: function.into(),
            views: None,
        }
    }

    /// A synchronous builder which explicitly requests reapplication of these views.
    /// View identifiers belong to the application; the view transaction must resolve
    /// them to reversible adapters. At least one nonempty identifier is required.
    pub fn view_builder(
        source: impl Into<String>,
        function: impl Into<String>,
        views: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            source: source.into(),
            function: function.into(),
            views: Some(views.into_iter().collect()),
        }
    }
}
