//! `TestFixtureBuilderExtensionTests` (`ReactiveUI.Testing.Tests/TestFixtureBuilderExtensionTests.cs`).
//!
//! The rows of upstream's data sources and `[Arguments]` are loops over the rows.

use crate::mocks::{TestFixture, TestFixtureBuilder};
use std::collections::HashMap;

/// The value the rows share.
const TESTING_VALUE: &str = "testing";

/// The lists of three tests (`Data`).
fn data() -> [(&'static str, &'static str, &'static str); 4] {
    [(TESTING_VALUE, "", ""), (TESTING_VALUE, TESTING_VALUE, ""), (TESTING_VALUE, TESTING_VALUE, "one"), (TESTING_VALUE, "one", "two")]
}

/// The keys and values (`KeyValues`).
fn key_values() -> [(&'static str, &'static str); 4] {
    [(TESTING_VALUE, ""), (TESTING_VALUE, "one"), (TESTING_VALUE, "two"), (TESTING_VALUE, "one two")]
}

/// The key-value pairs (`KeyValuePairs`).
fn key_value_pairs() -> [(String, String); 4] {
    [
        ("latch".to_string(), "key".to_string()),
        ("skeleton".to_string(), "key".to_string()),
        ("electronic".to_string(), "key".to_string()),
        ("rsa".to_string(), "key".to_string()),
    ]
}

#[test]
fn should_add_dictionary() {
    // Given, When
    let dictionary: HashMap<String, String> = [("check".to_string(), "one".to_string()), (TESTING_VALUE.to_string(), "two".to_string())].into_iter().collect();

    let builder: TestFixture = TestFixtureBuilder::new().with_dictionary(&dictionary).into();

    // Then
    assert_eq!(builder.variables.as_ref(), Some(&dictionary));
}

#[test]
fn should_add_key_value() {
    for (key, value) in key_values() {
        // Given, When
        let builder: TestFixture = TestFixtureBuilder::new().with_key_value(key, value).into();

        // Then
        let variables = builder.variables.expect("the variables are set");
        assert!(variables.contains_key(key));
        assert_eq!(variables[key], value);
    }
}

#[test]
fn should_add_key_value_pair() {
    for key_value_pair in key_value_pairs() {
        // Given, When
        let builder: TestFixture = TestFixtureBuilder::new().with_key_value_pair(key_value_pair.clone()).into();

        // Then
        let variables = builder.variables.expect("the variables are set");
        assert!(variables.contains_key(&key_value_pair.0));
        assert_eq!(variables[&key_value_pair.0], key_value_pair.1);
    }
}

#[test]
fn should_add_range_to_list() {
    for (test1, test2, test3) in data() {
        // Given, When
        let builder: TestFixture = TestFixtureBuilder::new().with_tests([test1, test2, test3]).into();

        // Then
        assert_eq!(builder.tests, Some(vec![test1.to_string(), test2.to_string(), test3.to_string()]));
    }
}

#[test]
fn should_add_value_to_list() {
    // Given, When
    let builder: TestFixture = TestFixtureBuilder::new().with_test(TESTING_VALUE).into();

    // Then
    assert_eq!(builder.tests, Some(vec![TESTING_VALUE.to_string()]));
}

#[test]
fn should_return_count() {
    for count in [1, 100, 1_000, 10_000, 100_000] {
        // Given, When
        let builder: TestFixture = TestFixtureBuilder::new().with_count(count).into();

        // Then
        assert_eq!(builder.count, count);
    }
}

#[test]
fn should_return_name() {
    for name in ["ReactiveUI", "Splat", "Sextant", "Akavache"] {
        // Given, When
        let builder: TestFixture = TestFixtureBuilder::new().with_name(name).into();

        // Then
        assert_eq!(builder.name.as_deref(), Some(name));
    }
}
