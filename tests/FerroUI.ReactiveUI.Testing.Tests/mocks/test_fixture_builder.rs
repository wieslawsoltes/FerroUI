//! `TestFixtureBuilder` (`ReactiveUI.Testing.Tests/TestFixtureBuilder.cs`).

use crate::mocks::TestFixture;
use ferroui_reactiveui_testing::{IBuilder, IBuilderExtensions};
use std::collections::HashMap;

/// A builder of [`TestFixture`]s through the setters of [`IBuilderExtensions`]
/// (`TestFixtureBuilder`).
#[derive(Clone, Debug, Default)]
pub struct TestFixtureBuilder {
    count: i32,
    name: Option<String>,
    tests: Vec<String>,
    variables: HashMap<String, String>,
}

impl IBuilder for TestFixtureBuilder {}

/// `implicit operator TestFixture(TestFixtureBuilder)`.
impl From<TestFixtureBuilder> for TestFixture {
    fn from(builder: TestFixtureBuilder) -> Self {
        TestFixtureBuilder::to_test_fixture(builder)
    }
}

// The setters call the methods of `IBuilderExtensions` by the trait, since some share their name
// (`with_dictionary`, `with_key_value`).
impl TestFixtureBuilder {
    /// A builder of a fixture with no data.
    pub fn new() -> Self {
        Self::default()
    }

    /// The fixture of `builder` (`ToTestFixture`).
    pub fn to_test_fixture(builder: TestFixtureBuilder) -> TestFixture {
        builder.build()
    }

    /// `WithCount`.
    pub fn with_count(self, count: i32) -> Self {
        IBuilderExtensions::with(self, |builder| &mut builder.count, count)
    }

    /// `WithDictionary`.
    pub fn with_dictionary(self, variables: &HashMap<String, String>) -> Self {
        IBuilderExtensions::with_dictionary(self, |builder| &mut builder.variables, variables)
    }

    /// `WithKeyValue(KeyValuePair<string, string>)`.
    pub fn with_key_value_pair(self, key_value_pair: (String, String)) -> Self {
        IBuilderExtensions::with_pair(self, |builder| &mut builder.variables, key_value_pair)
    }

    /// `WithKeyValue(string, string)`.
    pub fn with_key_value(self, key: &str, value: &str) -> Self {
        IBuilderExtensions::with_key_value(self, |builder| &mut builder.variables, key.to_string(), value.to_string())
    }

    /// `WithName`.
    pub fn with_name(self, name: &str) -> Self {
        IBuilderExtensions::with(self, |builder| &mut builder.name, Some(name.to_string()))
    }

    /// `WithTest`.
    pub fn with_test(self, test: &str) -> Self {
        IBuilderExtensions::with_value(self, |builder| &mut builder.tests, test.to_string())
    }

    /// `WithTests`.
    pub fn with_tests<'a>(self, tests: impl IntoIterator<Item = &'a str>) -> Self {
        IBuilderExtensions::with_values(self, |builder| &mut builder.tests, tests.into_iter().map(str::to_string))
    }

    /// `Build`.
    fn build(self) -> TestFixture {
        TestFixture { name: self.name, count: self.count, tests: Some(self.tests), variables: Some(self.variables) }
    }
}
