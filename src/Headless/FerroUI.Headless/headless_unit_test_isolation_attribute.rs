//! Port of `HeadlessUnitTestIsolationAttribute.cs`: the isolation level of
//! headless unit tests and the attribute that states it.
//!
//! The attribute of the original is an assembly attribute. There are no
//! assembly attributes: the value is a field of the
//! [`FerroTestAssembly`](crate::FerroTestAssembly) that a test crate hands
//! to
//! [`HeadlessUnitTestSession::get_or_start_for_assembly`](crate::HeadlessUnitTestSession::get_or_start_for_assembly).

/// Defines the isolation level for headless unit tests, controlling how the
/// application and its associated dispatcher are managed between test runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FerroTestIsolationLevel {
    /// Reuses a single application and dispatcher instance across all tests
    /// within the assembly.
    ///
    /// Tests must not rely on any global or persistent state that could
    /// leak between runs. Headless framework won't dispose any resources
    /// after tests when using this mode.
    PerAssembly,

    /// Recreates the application and dispatcher for each individual test
    /// method.
    ///
    /// This mode ensures complete test isolation, and should be used for
    /// tests that modify global application state or rely on a clean
    /// dispatcher environment. This is the default isolation level if none
    /// is specified.
    PerTest,
}

/// Specifies how headless unit tests should be isolated from each other,
/// defining when the test runtime should recreate the application and
/// dispatcher instances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FerroTestIsolationAttribute {
    isolation_level: FerroTestIsolationLevel,
}

impl FerroTestIsolationAttribute {
    pub const fn new(isolation_level: FerroTestIsolationLevel) -> Self {
        Self { isolation_level }
    }

    /// Gets the isolation level for headless tests.
    pub fn isolation_level(&self) -> FerroTestIsolationLevel {
        self.isolation_level
    }
}
