//! Interpret saved designer references against the same paths used at preview time.
use super::token;
use crate::{require, Result};
use ferroui_designer_core::TextPatch;
use ferroui_designer_project::{
    canonical_file_uri, file_path, file_uri, relative_asset_uri, relative_file_path, XamlFileMove,
};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
pub(super) struct StageSet<'a> {
    version: u32,
    #[serde(borrow)]
    stages: Vec<Stage<'a>>,
}

#[derive(Deserialize)]
struct Stage<'a> {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "environment")]
    _environment: StageEnvironment,
    #[serde(borrow)]
    source_uri: &'a RawValue,
}

#[derive(Deserialize)]
struct StageEnvironment {
    #[serde(rename = "width")]
    _width: f64,
    #[serde(rename = "height")]
    _height: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Scenario<'a> {
    version: u32,
    #[serde(borrow)]
    document: &'a RawValue,
    #[serde(borrow)]
    prototype: Option<Prototype<'a>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Prototype<'a> {
    stage_set: String,
    #[serde(borrow)]
    sources: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct Remap<'a> {
    #[serde(borrow)]
    path: &'a RawValue,
}

impl Scenario<'_> {
    pub(super) fn moved_stage_keys<'a>(
        &self,
        base: &Path,
        stages: &'a BTreeMap<&str, BTreeSet<String>>,
    ) -> Option<&'a BTreeSet<String>> {
        let uri = resolved_path(base, &self.prototype.as_ref()?.stage_set)?;
        stages.get(uri.as_str())
    }
}

pub(super) struct Move<'a> {
    movement: &'a XamlFileMove,
    destination: PathBuf,
}

impl<'a> Move<'a> {
    pub(super) fn new(movement: &'a XamlFileMove) -> Result<Self> {
        Ok(Self {
            movement,
            destination: file_path(&movement.to)?,
        })
    }

    pub(super) fn stage_patches(
        &self,
        source: &str,
        manifest: StageSet<'_>,
    ) -> Result<(Vec<TextPatch>, BTreeSet<String>)> {
        let mut keys = BTreeSet::new();
        let mut patches = Vec::new();
        for stage in manifest.stages {
            let Ok(uri) = token::string(stage.source_uri) else {
                continue;
            };
            if canonical_file_uri(&uri).is_ok_and(|resolved| resolved == self.movement.from) {
                require(
                    (1..=3).contains(&manifest.version),
                    "Update the saved stage set before moving its source",
                )?;
                patches.push(token::replace(source, stage.source_uri, &self.movement.to)?);
                keys.insert(uri);
            }
        }
        Ok((patches, keys))
    }

    pub(super) fn scenario_patches(
        &self,
        uri: &str,
        base: &Path,
        source: &str,
        scenario: Scenario<'_>,
        changed: Option<&BTreeSet<String>>,
    ) -> Result<Vec<TextPatch>> {
        let mut patches = Vec::new();
        self.patch_path(base, source, scenario.document, &mut patches)?;
        if let Some(Prototype {
            sources: Some(remaps),
            ..
        }) = scenario.prototype
        {
            let mut destinations = BTreeSet::new();
            let mut conflict = false;
            let mut rekeyed = false;
            for (key, value) in token::entries(remaps)? {
                let name = token::string(key).map_err(token::error)?;
                let remap = serde_json::from_str::<Remap<'_>>(value.get()).map_err(token::error)?;
                self.patch_path(base, source, remap.path, &mut patches)?;
                let destination = if changed.is_some_and(|keys| keys.contains(&name)) {
                    patches.push(token::replace(source, key, &self.movement.to)?);
                    rekeyed = true;
                    self.movement.to.clone()
                } else {
                    name
                };
                conflict |= !destinations.insert(destination);
            }
            require(
                !rekeyed || !conflict,
                format!(
                    "Resolve the duplicate prototype source remap before moving the file: {uri}"
                ),
            )?;
        }
        require(
            patches.is_empty() || scenario.version == 1,
            format!("Update the saved scenario before moving its source: {uri}"),
        )?;
        Ok(patches)
    }

    fn patch_path(
        &self,
        base: &Path,
        source: &str,
        value: &RawValue,
        patches: &mut Vec<TextPatch>,
    ) -> Result<()> {
        let path = token::string(value).map_err(token::error)?;
        if resolved_path(base, &path).as_deref() == Some(self.movement.from.as_str()) {
            let next = if Path::new(&path).is_absolute() {
                self.destination.to_string_lossy().into_owned()
            } else {
                let relative = relative_file_path(base, &self.destination)?;
                if path.starts_with("./") && !relative.starts_with("../") {
                    format!("./{relative}")
                } else {
                    relative
                }
            };
            patches.push(token::replace(source, value, &next)?);
        }
        Ok(())
    }

    pub(super) fn may_reference(&self, source: &str) -> bool {
        let Some(name) = file_path(&self.movement.from)
            .ok()
            .and_then(|path| path.file_name()?.to_str().map(str::to_owned))
        else {
            return false;
        };
        source.contains(&name)
            || relative_asset_uri(&name).is_some_and(|name| source.contains(&name))
    }
}

fn resolved_path(base: &Path, path: &str) -> Option<String> {
    file_uri(base.join(path).canonicalize().ok()?).ok()
}
