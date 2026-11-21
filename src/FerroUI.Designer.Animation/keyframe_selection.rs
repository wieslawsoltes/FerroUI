use crate::{
    AnimationError, KeyframeClipboard, KeyframeGroupEdit, KeyframeLocation, KeyframePaste, Result,
    Timeline, require,
};
use serde::{Deserialize, Serialize};

/// A source-neutral action on the authored keys of one animation program.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum KeyframeSelection {
    CopyKeyframes {
        keyframes: Vec<KeyframeLocation>,
    },
    MoveKeyframes {
        keyframes: Vec<KeyframeLocation>,
        offset: f64,
    },
    DuplicateKeyframes {
        keyframes: Vec<KeyframeLocation>,
        offset: f64,
    },
    ScaleKeyframes {
        keyframes: Vec<KeyframeLocation>,
        pivot: f64,
        factor: f64,
    },
    RemoveKeyframes {
        keyframes: Vec<KeyframeLocation>,
    },
    MirrorKeyframes {
        keyframes: Vec<KeyframeLocation>,
        pivot: f64,
    },
    PasteKeyframes {
        paste: KeyframePaste,
    },
}

impl KeyframeSelection {
    pub fn from_json(text: &str) -> Result<Self> {
        require(
            text.len() <= 1024 * 1024,
            "The keyframe request exceeds 1 MiB",
        )?;
        serde_json::from_str(text).map_err(|error| AnimationError(error.to_string()))
    }

    pub fn keyframes(&self) -> &[KeyframeLocation] {
        match self {
            Self::CopyKeyframes { keyframes }
            | Self::MoveKeyframes { keyframes, .. }
            | Self::DuplicateKeyframes { keyframes, .. }
            | Self::ScaleKeyframes { keyframes, .. }
            | Self::RemoveKeyframes { keyframes }
            | Self::MirrorKeyframes { keyframes, .. } => keyframes,
            Self::PasteKeyframes { .. } => &[],
        }
    }

    pub fn group_edit(&self) -> Option<KeyframeGroupEdit> {
        match self {
            Self::MoveKeyframes { offset, .. } => Some(KeyframeGroupEdit::Move(*offset)),
            Self::DuplicateKeyframes { offset, .. } => Some(KeyframeGroupEdit::Duplicate(*offset)),
            Self::ScaleKeyframes { pivot, factor, .. } => Some(KeyframeGroupEdit::Scale {
                pivot: *pivot,
                factor: *factor,
            }),
            Self::RemoveKeyframes { .. } => Some(KeyframeGroupEdit::Remove),
            _ => None,
        }
    }

    /// Use an accepted timeline to resolve input precision and plan an atomic edit.
    pub fn plan(&self, timeline: &Timeline) -> Result<KeyframeSelectionPlan> {
        require(
            self.keyframes().len() <= 4096,
            "Select up to 4096 keyframes",
        )?;
        let mut locations = self.keyframes().to_vec();
        for location in &mut locations {
            location.reconcile_time(timeline);
        }
        let (timeline, keys) = match self {
            Self::CopyKeyframes { .. } => {
                return Ok(KeyframeSelectionPlan::Copy(
                    timeline.copy_keyframes(&locations)?,
                ));
            }
            Self::MirrorKeyframes { pivot, .. } => {
                timeline.with_mirrored_keyframes(&locations, *pivot)?
            }
            Self::PasteKeyframes { paste } => timeline.with_pasted_keyframes(paste)?,
            _ => timeline.with_keyframe_edit(&locations, self.group_edit().expect("group edit"))?,
        };
        Ok(KeyframeSelectionPlan::Edit { timeline, keys })
    }
}

pub enum KeyframeSelectionPlan {
    Copy(KeyframeClipboard),
    Edit {
        timeline: Timeline,
        keys: Vec<KeyframeLocation>,
    },
}
