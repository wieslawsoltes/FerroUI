use crate::{AiProvider, AiProviderConfig, StudioError, StudioStorage};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StudioSettings {
    pub auto_save_enabled: bool,
    pub auto_save_interval_seconds: u32,
    pub auto_backup_enabled: bool,
    pub max_backup_count: usize,
    pub default_export_format: String,
    pub last_session_path: String,
    pub ai_translation_context: String,
    pub ai_providers: Vec<AiProviderConfig>,
}
impl Default for StudioSettings {
    fn default() -> Self {
        Self {
            auto_save_enabled: true,
            auto_save_interval_seconds: 30,
            auto_backup_enabled: true,
            max_backup_count: 10,
            default_export_format: "Gettext PO/POT".into(),
            last_session_path: String::new(),
            ai_translation_context: String::new(),
            ai_providers: AiProvider::ALL
                .into_iter()
                .map(AiProviderConfig::for_provider)
                .collect(),
        }
    }
}
impl StudioSettings {
    pub fn validate(&self) -> Result<(), StudioError> {
        if !(10..=300).contains(&self.auto_save_interval_seconds) {
            return Err(StudioError::InvalidInput(
                "Autosave interval must be 10–300 seconds".into(),
            ));
        }
        if !(1..=50).contains(&self.max_backup_count) {
            return Err(StudioError::InvalidInput(
                "Backup count must be 1–50".into(),
            ));
        }
        let mut providers = BTreeSet::new();
        for provider in &self.ai_providers {
            provider.validate()?;
            if !providers.insert(provider.provider) {
                return Err(StudioError::InvalidInput(
                    "Duplicate AI provider configuration".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CredentialPersistence {
    #[default]
    Omit,
    Include,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsLoad {
    pub settings: StudioSettings,
    pub warning: Option<String>,
}

pub struct SettingsService {
    storage: Rc<dyn StudioStorage>,
    path: String,
}
impl SettingsService {
    pub fn new(storage: Rc<dyn StudioStorage>, path: impl Into<String>) -> Self {
        Self {
            storage,
            path: path.into(),
        }
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn load(&self) -> Result<Option<StudioSettings>, StudioError> {
        let Some(bytes) = self.storage.read(&self.path)? else {
            return Ok(None);
        };
        let settings: StudioSettings = serde_json::from_slice(&bytes)?;
        settings.validate()?;
        Ok(Some(settings))
    }
    /// Corrupt settings are preserved and surfaced alongside usable defaults.
    pub fn load_or_default(&self) -> SettingsLoad {
        match self.load() {
            Ok(Some(settings)) => SettingsLoad {
                settings,
                warning: None,
            },
            Ok(None) => SettingsLoad {
                settings: StudioSettings::default(),
                warning: None,
            },
            Err(error) => SettingsLoad {
                settings: StudioSettings::default(),
                warning: Some(error.to_string()),
            },
        }
    }
    pub fn save(&self, settings: &StudioSettings) -> Result<(), StudioError> {
        self.save_with_credentials(settings, CredentialPersistence::Omit)
    }
    pub fn save_with_credentials(
        &self,
        settings: &StudioSettings,
        credentials: CredentialPersistence,
    ) -> Result<(), StudioError> {
        settings.validate()?;
        let mut saved = settings.clone();
        if credentials == CredentialPersistence::Omit {
            for provider in &mut saved.ai_providers {
                provider.api_key.clear();
            }
        }
        self.storage
            .write_atomic(&self.path, &serde_json::to_vec_pretty(&saved)?)
    }
}

/// Editing settings is transactional: cancelling discards the draft, and a
/// failed save keeps it available without changing the original configuration.
pub struct SettingsEditor {
    original: StudioSettings,
    draft: StudioSettings,
}
impl SettingsEditor {
    pub fn new(settings: StudioSettings) -> Self {
        Self {
            draft: settings.clone(),
            original: settings,
        }
    }
    pub fn draft(&self) -> &StudioSettings {
        &self.draft
    }
    pub fn draft_mut(&mut self) -> &mut StudioSettings {
        &mut self.draft
    }
    pub fn cancel(self) -> StudioSettings {
        self.original
    }
    pub fn save(
        &mut self,
        service: &SettingsService,
        credentials: CredentialPersistence,
    ) -> Result<StudioSettings, StudioError> {
        service.save_with_credentials(&self.draft, credentials)?;
        self.original = self.draft.clone();
        Ok(self.draft.clone())
    }
}
