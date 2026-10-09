//! Port of the test application attribute of the upstream headless project.
//!
//! The original is an assembly attribute, read through reflection by the
//! session of a test assembly. There are no assembly attributes: the value
//! is a field of the [`FerroTestAssembly`](crate::FerroTestAssembly) that a
//! test crate hands to
//! [`HeadlessUnitTestSession::get_or_start_for_assembly`](crate::HeadlessUnitTestSession::get_or_start_for_assembly).

use ferroui_controls::AppBuilder;

/// Sets up global test framework using the application builder passed as a
/// parameter.
#[derive(Clone, Copy, Debug)]
pub struct FerroTestApplicationAttribute {
    app_builder_entry_point_type: fn() -> AppBuilder,
}

impl FerroTestApplicationAttribute {
    /// Creates instance of [`FerroTestApplicationAttribute`].
    ///
    /// `app_builder_entry_point_type` is the parameter from which
    /// [`AppBuilder`] should be created. The original takes a type that has
    /// a method which builds the application or that inherits the
    /// application class, and finds the method through reflection; here it
    /// is that method (`AppBuilder::configure::<TApp>` for a type that
    /// inherits the application class).
    pub const fn new(app_builder_entry_point_type: fn() -> AppBuilder) -> Self {
        Self { app_builder_entry_point_type }
    }

    pub fn app_builder_entry_point_type(&self) -> fn() -> AppBuilder {
        self.app_builder_entry_point_type
    }
}
