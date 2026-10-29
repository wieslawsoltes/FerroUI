//! PersistentState and runtime storage: versioned JSON, validated on load,
//! atomically replaced on save. The platform path can be overridden for tests.
use super::*;
use crate::view_models::CardGameViewModel;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SettingsState {
    pub draw_mode: DrawMode,
    pub difficulty: Difficulty,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GameStatistics {
    pub played: u32,
    pub won: u32,
    pub best_score: Option<i32>,
    pub best_seconds: Option<u64>,
    pub fewest_moves: Option<u32>,
    pub total_seconds: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistentState {
    pub version: u32,
    pub settings: SettingsState,
    pub statistics: [GameStatistics; 3],
    pub games: [Option<CardGameViewModel>; 3],
}
impl Default for PersistentState {
    fn default() -> Self {
        Self {
            version: 1,
            settings: SettingsState::default(),
            statistics: Default::default(),
            games: Default::default(),
        }
    }
}
impl PersistentState {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("Unsupported save version {}", self.version));
        }
        for kind in GameKind::ALL {
            if let Some(game) = &self.games[kind.index()] {
                if game.kind != kind {
                    return Err("Game stored in wrong slot".into());
                }
                game.validate()?;
            }
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => {
                let state: Self = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("Could not read save: {e}"))?;
                state.validate()?;
                Ok(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("Could not open save: {e}")),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        serde_json::to_writer(&mut file, self).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn default_path() -> PathBuf {
        std::env::var_os("FERRO_SOLITAIRE_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::data_local_dir().map(|p| p.join("FerroSolitaire")))
            .unwrap_or_else(|| PathBuf::from(".ferro-solitaire"))
            .join("state.json")
    }
}
