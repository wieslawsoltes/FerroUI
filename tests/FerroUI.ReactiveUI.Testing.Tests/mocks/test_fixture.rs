//! `TestFixture` (`ReactiveUI.Testing.Tests/TestFixture.cs`).

use std::collections::HashMap;

/// The test data that [`TestFixtureBuilder`](crate::mocks::TestFixtureBuilder) builds
/// (`TestFixture`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TestFixture {
    /// `Count`.
    pub count: i32,
    /// `Name`.
    pub name: Option<String>,
    /// `Tests`.
    pub tests: Option<Vec<String>>,
    /// `Variables`.
    pub variables: Option<HashMap<String, String>>,
}
