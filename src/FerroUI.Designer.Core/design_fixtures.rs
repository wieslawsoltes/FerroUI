//! Authored preview data shared by the language server and native design hosts.
use crate::{Attribute, Document};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub const DESIGN_NAMESPACE: &str = "http://schemas.microsoft.com/expression/blend/2008";
pub const FIXTURES_ATTRIBUTE: &str = "Fixtures";
const MAX_BYTES: usize = 128 * 1024;
const MAX_FIXTURES: usize = 32;
const MAX_VALUES: usize = 4096;
const MAX_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignFixture {
    pub name: String,
    pub data: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignFixtures {
    pub default: String,
    pub fixtures: Vec<DesignFixture>,
}

impl DesignFixtures {
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.len() > MAX_BYTES {
            return Err("Design fixtures cannot exceed 128 KiB.".into());
        }
        let manifest: Self = serde_json::from_str(value)
            .map_err(|error| format!("Invalid design fixtures JSON: {error}"))?;
        if manifest.fixtures.is_empty() || manifest.fixtures.len() > MAX_FIXTURES {
            return Err("Provide between 1 and 32 named design fixtures.".into());
        }
        let mut names = BTreeSet::new();
        let mut remaining = MAX_VALUES;
        for fixture in &manifest.fixtures {
            validate_name(&fixture.name)?;
            if !names.insert(&fixture.name) {
                return Err(format!(
                    "Design fixture '{}' is declared twice.",
                    fixture.name
                ));
            }
            validate_fields(&fixture.data, 1, &mut remaining)?;
        }
        if manifest.get(&manifest.default).is_none() {
            return Err("The default design fixture must name an existing fixture.".into());
        }
        Ok(manifest)
    }

    pub fn from_document(document: &Document) -> Result<Option<Self>, String> {
        document
            .ast()
            .node(document.ast().root())
            .into_iter()
            .flat_map(|root| &root.attributes)
            .find(|attribute| is_fixtures(attribute))
            .map(|attribute| Self::parse(&attribute.value))
            .transpose()
    }

    pub fn get(&self, name: &str) -> Option<&DesignFixture> {
        self.fixtures.iter().find(|fixture| fixture.name == name)
    }
}

pub fn is_fixtures(attribute: &Attribute) -> bool {
    attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
        && attribute.name.rsplit(':').next() == Some(FIXTURES_ATTRIBUTE)
}

pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 128
        || name.trim() != name
        || name.chars().any(char::is_control)
    {
        Err("Use a fixture name of 1 to 128 UTF-8 bytes without surrounding whitespace or control characters.".into())
    } else {
        Ok(())
    }
}

fn validate_fields(
    fields: &Map<String, Value>,
    depth: usize,
    remaining: &mut usize,
) -> Result<(), String> {
    for (name, value) in fields {
        let mut characters = name.chars();
        if name.len() > 128
            || !characters
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
            || !characters.all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err(format!(
                "'{name}' is not an identifier that can be used in a binding path."
            ));
        }
        validate_value(value, depth, remaining)?;
    }
    Ok(())
}

fn validate_value(value: &Value, depth: usize, remaining: &mut usize) -> Result<(), String> {
    if depth > MAX_DEPTH || *remaining == 0 {
        return Err("Design fixtures are limited to 16 levels and 4096 values.".into());
    }
    *remaining -= 1;
    match value {
        Value::Object(fields) => validate_fields(fields, depth + 1, remaining),
        Value::Array(items) => items
            .iter()
            .try_for_each(|item| validate_value(item, depth + 1, remaining)),
        Value::Number(number) if number.as_f64().is_none() => {
            Err("Use a finite JSON number in design fixtures.".into())
        }
        _ => Ok(()),
    }
}
