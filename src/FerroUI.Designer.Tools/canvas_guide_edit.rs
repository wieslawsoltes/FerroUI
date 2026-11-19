use crate::{CanvasGuide, CanvasGuideAxis, CanvasGuideSettings};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasGuideDraft {
    pub id: Option<u64>,
    pub axis: CanvasGuideAxis,
    pub position: f64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanvasGuideEdit {
    Visibility {
        rulers: Option<bool>,
        guides: Option<bool>,
    },
    Add {
        axis: CanvasGuideAxis,
        position: f64,
    },
    Move {
        id: u64,
        position: f64,
    },
    Duplicate {
        id: u64,
        position: f64,
    },
    Remove {
        id: u64,
    },
    Clear,
    Apply {
        rulers: bool,
        guides: bool,
        lines: Vec<CanvasGuideDraft>,
    },
}

#[derive(Debug, Serialize)]
pub struct CanvasGuideUpdate {
    pub settings: CanvasGuideSettings,
    pub selected: Option<u64>,
}

impl CanvasGuideEdit {
    /// Build an accepted view update without changing the original settings.
    pub fn plan(self, current: &CanvasGuideSettings) -> Result<CanvasGuideUpdate, String> {
        current.validate()?;
        let mut settings = current.clone();
        let selected = match self {
            Self::Visibility { rulers, guides } => {
                settings.rulers_visible = rulers.unwrap_or(settings.rulers_visible);
                settings.guides_visible = guides.unwrap_or(settings.guides_visible);
                None
            }
            Self::Add { axis, position } => Some(settings.add(axis, position)?),
            Self::Move { id, position } => {
                settings
                    .guides
                    .iter_mut()
                    .find(|guide| guide.id == id)
                    .ok_or("Unknown canvas guide")?
                    .position = position;
                Some(id)
            }
            Self::Duplicate { id, position } => {
                let axis = current
                    .guides
                    .iter()
                    .find(|guide| guide.id == id)
                    .ok_or("Unknown canvas guide")?
                    .axis;
                Some(settings.add(axis, position)?)
            }
            Self::Remove { id } => {
                let index = current
                    .guides
                    .iter()
                    .position(|guide| guide.id == id)
                    .ok_or("Unknown canvas guide")?;
                settings.guides.remove(index);
                None
            }
            Self::Clear => {
                settings.guides.clear();
                None
            }
            Self::Apply {
                rulers,
                guides,
                lines,
            } => {
                if lines.len() > CanvasGuideSettings::MAX_GUIDES {
                    return Err(format!(
                        "Use at most {} guides in one canvas scope",
                        CanvasGuideSettings::MAX_GUIDES
                    ));
                }
                settings.rulers_visible = rulers;
                settings.guides_visible = guides;
                settings.guides.clear();
                for (id, line) in lines.iter().filter_map(|line| line.id.map(|id| (id, line))) {
                    if !current
                        .guides
                        .iter()
                        .any(|guide| guide.id == id && guide.axis == line.axis)
                    {
                        return Err("Choose an existing guide in the current canvas scope".into());
                    }
                    settings.guides.push(CanvasGuide {
                        id,
                        axis: line.axis,
                        position: line.position,
                    });
                }
                for line in lines.into_iter().filter(|line| line.id.is_none()) {
                    settings.add(line.axis, line.position)?;
                }
                None
            }
        };
        settings.validate()?;
        Ok(CanvasGuideUpdate { settings, selected })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CanvasGuideAxis::{Horizontal, Vertical};

    #[test]
    fn checked_edits_retire_unknown_ids_and_keep_fractional_guides() {
        let mut base = CanvasGuideSettings::default();
        let id = base.add(Vertical, 30.25).unwrap();
        let changed = CanvasGuideEdit::Duplicate {
            id,
            position: -18.5,
        }
        .plan(&base)
        .unwrap();
        assert_eq!(changed.selected, Some(2));
        assert_eq!(changed.settings.guides[1].axis, Vertical);
        assert!(CanvasGuideEdit::Move {
            id: 8,
            position: 5.0
        }
        .plan(&base)
        .is_err());
        assert!(CanvasGuideEdit::Move {
            id,
            position: f64::NAN
        }
        .plan(&base)
        .is_err());

        let valid = CanvasGuideEdit::Apply {
            rulers: true,
            guides: false,
            lines: vec![
                CanvasGuideDraft {
                    id: Some(id),
                    axis: Vertical,
                    position: 44.5,
                },
                CanvasGuideDraft {
                    id: None,
                    axis: Horizontal,
                    position: -1.75,
                },
            ],
        }
        .plan(&base)
        .unwrap();
        assert!(valid.settings.rulers_visible && !valid.settings.guides_visible);
        assert_eq!(
            valid
                .settings
                .guides
                .iter()
                .map(|guide| (guide.id, guide.position))
                .collect::<Vec<_>>(),
            [(id, 44.5), (2, -1.75)]
        );
        assert_eq!(base.guides[0].position, 30.25);
        let duplicate = CanvasGuideDraft {
            id: Some(id),
            axis: Vertical,
            position: 0.0,
        };
        assert!(CanvasGuideEdit::Apply {
            rulers: true,
            guides: true,
            lines: vec![duplicate; 2]
        }
        .plan(&base)
        .is_err());
        assert!(CanvasGuideEdit::Apply {
            rulers: true,
            guides: true,
            lines: vec![CanvasGuideDraft {
                axis: Horizontal,
                ..duplicate
            }]
        }
        .plan(&base)
        .is_err());
    }
}
