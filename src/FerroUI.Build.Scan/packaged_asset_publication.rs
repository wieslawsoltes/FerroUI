//! Build inputs that determine where an asset will be published after a move.

use super::{field, json_string, JsonValue};
use std::{panic::Location, path::PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPublication {
    pub directories: Vec<AssetDeclaration>,
    /// A closed export plan selects individual paths. An open plan exports every embedded asset.
    pub registrations: Option<Vec<AssetDeclaration>>,
}

/// An argument passed to `embed_assets` or `ExportPlan::asset` in the selected build.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetDeclaration {
    pub value: String,
    pub rust_file: PathBuf,
    pub line: u32,
    pub column: u32,
}

impl AssetDeclaration {
    pub fn at(value: &str, caller: &Location<'_>) -> Self {
        Self {
            value: value.into(),
            rust_file: caller.file().into(),
            line: caller.line(),
            column: caller.column(),
        }
    }

    fn parse(json: &JsonValue) -> Result<Self, String> {
        let value = json.object("asset declaration")?;
        Ok(Self {
            value: field(value, "value")?.string("value")?.into(),
            rust_file: field(value, "rust_file")?.string("rust_file")?.into(),
            line: field(value, "line")?.integer("line")?,
            column: field(value, "column")?.integer("column")?,
        })
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"value\":{},\"rust_file\":{},\"line\":{},\"column\":{}}}",
            json_string(&self.value),
            json_string(&self.rust_file.to_string_lossy()),
            self.line,
            self.column
        )
    }
}

impl AssetPublication {
    pub(super) fn parse(json: &JsonValue) -> Result<Self, String> {
        let publication = json.object("asset publication")?;
        let declarations = |value: &JsonValue| {
            value
                .array("asset declarations")?
                .iter()
                .map(AssetDeclaration::parse)
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(Self {
            directories: declarations(field(publication, "directories")?)?,
            registrations: match field(publication, "registrations")? {
                JsonValue::Null => None,
                entries => Some(declarations(entries)?),
            },
        })
    }

    pub(super) fn to_json(&self) -> String {
        let declarations = |entries: &[AssetDeclaration]| {
            format!(
                "[{}]",
                entries
                    .iter()
                    .map(AssetDeclaration::to_json)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let directories = declarations(&self.directories);
        let registrations = self
            .registrations
            .as_deref()
            .map_or_else(|| "null".into(), declarations);
        format!("{{\"directories\":{directories},\"registrations\":{registrations}}}")
    }
}
