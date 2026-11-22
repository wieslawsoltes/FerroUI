//! Playback capabilities and preview-only target assignments for authored programs.
use crate::{DesignerSession, PreviewSourceAnchor};
use ferroui_designer_animation::{AnimationProgram, AnimationProgramLocator as ProgramLocator, ValueKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TimelineTargetOverrides {
    programs: Vec<PreviewTargets>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewTargets {
    source_key: Option<String>,
    anchor: Option<ProgramLocator>,
    targets: Vec<PreviewSourceAnchor>,
}

#[cfg(feature = "mcp")]
#[derive(Serialize)]
pub(crate) struct ProgramPreview {
    pub(crate) native: bool,
    pub(crate) duration: f64,
    pub(crate) total_duration: Option<f64>,
    pub(crate) available: bool,
    explicit_targets: bool,
}

impl DesignerSession {
    pub(crate) fn seeks_animation_elapsed(&self, program: &AnimationProgram) -> bool {
        program.timeline.is_none()
            || self.animation.native.uses_factory_target(program.id)
    }

    pub(crate) fn requires_native_animation(&self, program: &AnimationProgram) -> bool {
        self.seeks_animation_elapsed(program)
            || self.animation.native.uses_custom_animators(program.id)
            || self.animation.native.uses_transform_components(program.id)
            || program.timeline.as_ref().is_some_and(|timeline| timeline.tracks.iter().any(|track| {
                track.keyframes.first().is_some_and(|key| matches!(key.value.kind(), ValueKind::Effect | ValueKind::BoxShadows))
            }))
            || (program.has_project_codec() && self.animation.native.duration(program.id).is_some())
    }

    pub(crate) fn has_animation_targets(&self, program: &AnimationProgram) -> bool {
        !program.targets.is_empty() || self.animation.native.uses_factory_target(program.id)
    }

    pub(crate) fn animation_preview_available(&self, program: &AnimationProgram) -> bool {
        self.has_animation_targets(program)
            && (!self.requires_native_animation(program)
                || self.animation.native.timing(program.id).is_some())
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn program_preview(&self, program: &AnimationProgram) -> Option<ProgramPreview> {
        let native = self.seeks_animation_elapsed(program);
        let timing;
        let clock = if native {
            timing = self.animation.native.timing(program.id)?;
            &timing
        } else {
            program.timeline.as_ref()?
        };
        Some(ProgramPreview {
            native,
            duration: clock.duration,
            total_duration: clock.total_duration(),
            available: self.animation_preview_available(program),
            explicit_targets: self.animation.preview_targets.contains_key(&program.id),
        })
    }

    pub fn timeline_target_overrides(&self) -> TimelineTargetOverrides {
        let programs = self
            .animation_programs()
            .iter()
            .filter_map(|program| {
                let targets = self.animation.preview_targets.get(&program.id)?;
                let anchor = ProgramLocator::capture(self.document(), program);
                if program.source_key.is_none() && anchor.is_none() {
                    return None;
                }
                let targets = targets
                    .iter()
                    .filter_map(|&id| PreviewSourceAnchor::capture(self.document(), id).ok())
                    .collect();
                Some(PreviewTargets {
                    source_key: program.source_key.clone(),
                    anchor,
                    targets,
                })
            })
            .collect();
        TimelineTargetOverrides { programs }
    }

    pub fn restore_timeline_targets(&mut self, saved: &TimelineTargetOverrides) {
        let mut overrides = BTreeMap::new();
        for entry in &saved.programs {
            let Some(program) =
                self.resolve_preview_program(entry.source_key.as_deref(), entry.anchor.as_ref())
            else {
                continue;
            };
            let mut targets = Vec::new();
            for target in &entry.targets {
                if let Some(id) = target
                    .resolve(self.document())
                    .ok()
                    .filter(|id| self.preview.visual(*id).is_some() && !targets.contains(id))
                {
                    targets.push(id);
                }
            }
            overrides.insert(program.id, targets);
        }
        if self.animation.preview_targets != overrides {
            self.animation.preview_targets = overrides;
            self.import_animations();
        }
    }
}
