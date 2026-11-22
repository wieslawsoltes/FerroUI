//! Reviewable conversion between DTCG tokens and a standalone XAML resource dictionary.
//! Both directions return a candidate; a host chooses when and where to write it.
mod bezier;
mod export;
mod gradient;
mod gradient_geometry;
mod import;
mod references;
mod scalars;
mod shadow;

pub use export::plan_export;
pub use import::plan_import;

use crate::{Result, ToolError};
use ferroui_designer_core::Node;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const EXTENSION: &str = "com.github.ferroui";
const GRADIENT_PROPERTIES: &str = "gradientProperties";

type ValueResult<T> = std::result::Result<T, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub direction: Direction,
    pub source: String,
    #[serde(default)]
    pub options: Options,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Import,
    Export,
}

impl Request {
    pub fn plan(&self) -> Result<Plan> {
        match self.direction {
            Direction::Import => plan_import(&self.source, &self.options),
            Direction::Export => plan_export(&self.source, &self.options),
        }
    }
}

/// Overrides are keyed by DTCG token path during import and XAML key during export.
#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Options {
    pub mappings: BTreeMap<String, Mapping>,
    pub pixels_per_rem: Option<f64>,
}

#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Mapping {
    pub key: Option<String>,
    pub token: Option<String>,
    pub kind: Option<String>,
    pub xaml_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub source: String,
    pub mappings: Vec<Conversion>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversion {
    pub token: String,
    pub key: String,
    pub kind: String,
    pub xaml_type: String,
    pub value: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Issue {
    pub path: String,
    pub message: String,
}

fn size(source: &str) -> Result<()> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(ToolError("The token handoff source exceeds 1 MiB".into()));
    }
    Ok(())
}

fn valid_token_name(name: &str) -> bool {
    !name.trim().is_empty() && !name.starts_with('$') && !name.contains(['.', '{', '}'])
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn simple_key(token: &str) -> String {
    token
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn native_type(node: &Node, namespace: &str) -> bool {
    node.namespace_uri.as_deref().is_some_and(|uri| {
        uri == crate::components::FRAMEWORK
            || uri.strip_prefix("using:") == Some(namespace)
            || uri
                .strip_prefix("clr-namespace:")
                .and_then(|uri| uri.split_once(";assembly="))
                == Some((namespace, "FerroUI.Base"))
    })
}

#[cfg(test)]
mod compound_tests;
#[cfg(test)]
mod tests;
