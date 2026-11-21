//! Compatible stage manifests and per-stage design fixture extensions.

use super::{
    PreviewPrototypeManifest, PreviewStageConfiguration, PreviewStageInfo, MAX_MANIFEST_BYTES,
    MAX_PREVIEW_STAGES,
};
use serde::{Deserialize, Serialize};

/// A stage configuration with an optional design-time fixture override.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageSpec {
    #[serde(flatten)]
    pub configuration: PreviewStageConfiguration,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design_fixture: Option<String>,
}

impl From<PreviewStageConfiguration> for PreviewStageSpec {
    fn from(configuration: PreviewStageConfiguration) -> Self {
        Self {
            configuration,
            design_fixture: None,
        }
    }
}

impl PreviewStageSpec {
    pub fn validate(&self) -> Result<(), String> {
        self.configuration.validate()?;
        if let Some(name) = &self.design_fixture {
            ferroui_designer_core::design_fixtures::validate_name(name)?;
        }
        Ok(())
    }

    fn text_size(&self) -> usize {
        self.configuration.text_size() + self.design_fixture.as_ref().map_or(0, String::len)
    }

    pub(super) fn same_rendering(&self, other: &Self) -> bool {
        self.design_fixture == other.design_fixture
            && self.configuration.same_rendering(&other.configuration)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageManifest {
    pub version: u32,
    pub stages: Vec<PreviewStageConfiguration>,
    pub active: Option<usize>,
    #[serde(default, skip_serializing_if = "PreviewPrototypeManifest::is_empty")]
    pub prototype: PreviewPrototypeManifest,
}

pub type PreviewStageSnapshot = PreviewStageManifest;
pub type PreviewStageDetails = PreviewStageInfo<PreviewStageSpec>;

/// A saved stage set including the design fixture selected on each card.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageSet {
    pub version: u32,
    pub stages: Vec<PreviewStageSpec>,
    pub active: Option<usize>,
    #[serde(default, skip_serializing_if = "PreviewPrototypeManifest::is_empty")]
    pub prototype: PreviewPrototypeManifest,
}

impl From<PreviewStageManifest> for PreviewStageSet {
    fn from(manifest: PreviewStageManifest) -> Self {
        Self {
            version: manifest.version,
            stages: manifest
                .stages
                .into_iter()
                .map(PreviewStageSpec::from)
                .collect(),
            active: manifest.active,
            prototype: manifest.prototype,
        }
    }
}

impl PreviewStageManifest {
    pub fn validate(&self) -> Result<(), String> {
        validate_stages(
            self.version,
            self.stages.len(),
            self.active,
            &self.prototype,
            self.stages.iter().map(|config| {
                config.validate()?;
                Ok(config.text_size())
            }),
        )
    }

    pub fn from_json(source: &str) -> Result<Self, String> {
        let manifest = parse_manifest::<Self>(source)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        encode_manifest(self)
    }
}

impl PreviewStageSet {
    pub(super) fn into_manifest(self) -> PreviewStageManifest {
        PreviewStageManifest {
            version: self.version,
            stages: self
                .stages
                .into_iter()
                .map(|spec| spec.configuration)
                .collect(),
            active: self.active,
            prototype: self.prototype,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_stages(
            self.version,
            self.stages.len(),
            self.active,
            &self.prototype,
            self.stages.iter().map(|spec| {
                spec.validate()?;
                Ok(spec.text_size())
            }),
        )
    }

    pub fn from_json_with_fixtures(source: &str) -> Result<Self, String> {
        let manifest = parse_manifest::<Self>(source)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        encode_manifest(self)
    }
}

fn validate_stages(
    version: u32,
    stage_count: usize,
    active: Option<usize>,
    prototype: &PreviewPrototypeManifest,
    stages: impl Iterator<Item = Result<usize, String>>,
) -> Result<(), String> {
    if !matches!(version, 1..=3) {
        return Err("Unsupported preview-stage manifest version".into());
    }
    if version < 3 && prototype.has_overlay_actions() {
        return Err("Prototype overlays require preview-stage manifest version 3".into());
    }
    if version == 1 && !prototype.is_empty() {
        return Err("Prototype interactions require preview-stage manifest version 2".into());
    }
    if stage_count > MAX_PREVIEW_STAGES {
        return Err(format!(
            "A preview board supports at most {MAX_PREVIEW_STAGES} stages"
        ));
    }
    if active.is_some_and(|index| index >= stage_count) {
        return Err("The active preview-stage index is outside the manifest".into());
    }
    let text_size = stages.sum::<Result<usize, _>>()?;
    prototype.validate(stage_count)?;
    if text_size + prototype.text_size() > MAX_MANIFEST_BYTES {
        return Err("Preview-stage manifest text exceeds 1 MiB".into());
    }
    Ok(())
}

fn parse_manifest<T: serde::de::DeserializeOwned>(source: &str) -> Result<T, String> {
    if source.len() > MAX_MANIFEST_BYTES {
        return Err("Preview-stage manifest exceeds 1 MiB".into());
    }
    serde_json::from_str(source).map_err(|error| error.to_string())
}

fn encode_manifest(manifest: &impl Serialize) -> Result<String, String> {
    let source = serde_json::to_string_pretty(manifest).map_err(|error| error.to_string())?;
    if source.len() > MAX_MANIFEST_BYTES {
        return Err("Preview-stage manifest exceeds 1 MiB".into());
    }
    Ok(source)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_stage_set_retains_the_original_manifest_shape() {
        let original = PreviewStageManifest {
            version: 1,
            stages: vec![PreviewStageConfiguration::new(
                "Accounts",
                "file:///accounts.xaml",
                Default::default(),
            )],
            active: Some(0),
            prototype: Default::default(),
        };
        let mut extended =
            PreviewStageSet::from_json_with_fixtures(&original.to_json().unwrap()).unwrap();
        assert_eq!(extended.clone().into_manifest(), original);
        extended.stages[0].design_fixture = Some("Loading".into());
        let data = extended.to_json().unwrap();
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["stages"][0]["design_fixture"], "Loading");
        assert_eq!(json["stages"][0]["source_uri"], "file:///accounts.xaml");
        assert_eq!(
            PreviewStageSet::from_json_with_fixtures(&data).unwrap(),
            extended
        );
        assert!(PreviewStageSet::from_json_with_fixtures(
            &data.replace("design_fixture", "unknown")
        )
        .is_err());
        assert_eq!(extended.into_manifest(), original);
    }
}
