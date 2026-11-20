//! The explicit correspondence between a registered XAML type and its RSX builder.
mod literal;

use crate::{require, ProjectError, Result};
use ferroui_build_scan::{
    authored_symbols::AuthoredText, json_member_ranges, model_set::ModelSet, JsonMemberRange,
};
use ferroui_designer_core::TextPatch;
use serde::Deserialize;
use std::{collections::BTreeMap, ops::Range};
use syn::{ext::IdentExt, Path};

pub const PREFIX: &str = "FerroUI.Designer.Rsx.v1:";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RsxBuilderMapping {
    pub path: String,
    #[serde(default)]
    pub inherit: bool,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

impl RsxBuilderMapping {
    pub fn parse(text: &str) -> Result<Self> {
        serde_json::from_str(text)
            .map_err(|error| ProjectError(format!("Invalid RSX builder mapping: {error}")))
    }

    /// Change the last identifier only when its canonical Rust path is the
    /// selected declaration. An explicitly named re-export still uses its alias.
    pub fn renamed_builder(
        &self,
        models: &ModelSet,
        assembly_crate: &str,
        rust_type: &str,
        old: &str,
        new: &str,
    ) -> Option<String> {
        let path = declared_path(&self.path, assembly_crate)?;
        let last = &path.segments.last()?.ident;
        if last.unraw() != old {
            return None;
        }
        let tail = path
            .segments
            .iter()
            .skip(1)
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let absolute = format!("::{assembly_crate}::{tail}");
        if models.canonical_path(&absolute) != models.canonical_path(rust_type) {
            return None;
        }
        let mut new_path = self.path.clone();
        new_path.replace_range(last.span().byte_range(), new);
        Some(new_path)
    }

    pub fn rename_property(
        &self,
        source: &str,
        operand: &AuthoredText,
        old: &str,
        new: &str,
    ) -> Result<TextPatch> {
        require(
            self.properties.contains_key(old),
            "The RSX property mapping changed. Rebuild the project schema",
        )?;
        require(
            !self.properties.contains_key(new),
            format!("The RSX builder already maps {new}"),
        )?;
        let props = member(&operand.value, "properties", |member| member.value)?;
        let key = member(&operand.value[props.clone()], old, |member| member.key)?;
        let range = (props.start + key.start)..(props.start + key.end);
        literal::replace(source, operand, range, &encoded(new)?)
    }

    pub fn rename_path(source: &str, operand: &AuthoredText, new_path: &str) -> Result<TextPatch> {
        let range = member(&operand.value, "path", |member| member.value)?;
        literal::replace(source, operand, range, &encoded(new_path)?)
    }
}

/// Normalize a registered Rust path for generated source in the selected package.
pub fn assembly_path(source: &str, assembly_crate: &str, package: Option<&str>) -> Option<String> {
    let path = declared_path(source, assembly_crate)?;
    let mut parts = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string());
    let owner = parts.next()?;
    let tail = parts.collect::<Vec<_>>().join("::");
    if package == Some(assembly_crate) {
        Some(format!("crate::{tail}"))
    } else if owner == assembly_crate {
        Some(format!("::{owner}::{tail}"))
    } else {
        None
    }
}

pub fn rename_key(source: &str, operand: &AuthoredText, new_key: &str) -> Result<TextPatch> {
    let old = operand.value.rsplit(['.', ':']).next().unwrap_or_default();
    let start = operand.value.len() - old.len();
    let new = new_key
        .strip_prefix(&operand.value[..start])
        .ok_or_else(|| ProjectError("The RSX mapping namespace changed".into()))?;
    literal::replace(source, operand, start..operand.value.len(), new)
}

fn declared_path(source: &str, assembly_crate: &str) -> Option<Path> {
    let path = syn::parse_str::<Path>(source).ok()?;
    if path.segments.len() < 2
        || path
            .segments
            .iter()
            .any(|segment| !segment.arguments.is_empty())
    {
        return None;
    }
    let owner = &path.segments.first()?.ident;
    (owner == "crate" || owner == assembly_crate).then_some(path)
}

fn member(
    text: &str,
    name: &str,
    range: impl FnOnce(JsonMemberRange) -> Range<usize>,
) -> Result<Range<usize>> {
    let mut matches = json_member_ranges(text)
        .map_err(ProjectError)?
        .into_iter()
        .filter(|member| member.name == name);
    let member = matches
        .next()
        .ok_or_else(|| ProjectError(format!("The RSX mapping has no {name} entry")))?;
    require(
        matches.next().is_none(),
        format!("The RSX mapping contains more than one {name} entry"),
    )?;
    Ok(range(member))
}

fn encoded(value: &str) -> Result<String> {
    serde_json::to_string(value).map_err(|error| ProjectError(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_build_scan::model::{AssemblyModel, ExportModel};

    #[test]
    fn updates_the_renamed_reexport_and_preserves_named_aliases() {
        let original = "::example::controls::Card";
        let mut model = AssemblyModel::new("Example", "example");
        for path in ["::example::Card", "::example::PreviewCard"] {
            model.exports.push(ExportModel {
                path: path.into(),
                declared: original.into(),
            });
        }
        let models = ModelSet::new(vec![model]);
        let rename = |path: &str| {
            let mapping = RsxBuilderMapping {
                path: path.into(),
                inherit: false,
                properties: BTreeMap::new(),
            };
            mapping.renamed_builder(&models, "example", original, "Card", "Overview")
        };
        assert_eq!(rename("crate::Card").as_deref(), Some("crate::Overview"));
        assert_eq!(
            rename("::example::controls :: Card").as_deref(),
            Some("::example::controls :: Overview")
        );
        assert_eq!(rename("crate::PreviewCard"), None);
        assert_eq!(rename("crate::wrappers::Card"), None);
    }
}
