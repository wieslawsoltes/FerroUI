use crate::{DesignerSession, PreviewSourceAnchor};
use crate::animation_playback_pose::PlaybackPose;
use ferroui_designer_animation::AnimationProgram;
use ferroui_designer_core::Document;
use ferroui_designer_core::NodeId;
use serde::{Deserialize, Serialize};

/// The authored position of a Timeline preview. The program ID is valid in the
/// frame that returned this pose. A source anchor identifies it in a new frame.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelinePreviewPose {
    pub program: NodeId,
    pub time: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    anchor: Option<ProgramLocator>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) playback: Option<PlaybackPose>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProgramLocator {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<PreviewSourceAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<PreviewSourceAnchor>,
    name: Option<String>,
    selector: String,
}

impl ProgramLocator {
    pub(crate) fn capture(document: &Document, program: &AnimationProgram) -> Option<Self> {
        let source = PreviewSourceAnchor::capture(document, program.id).ok();
        let owner = PreviewSourceAnchor::capture(document, program.owner).ok();
        (source.is_some() || owner.is_some()).then(|| Self {
            source,
            owner,
            name: stable_name(program).map(str::to_owned),
            selector: program.selector.clone(),
        })
    }
}

fn stable_name(program: &AnimationProgram) -> Option<&str> {
    let generated = program.name.strip_prefix("Animation ").and_then(|id| id.parse().ok()) == Some(program.id);
    (!generated).then_some(program.name.as_str())
}

impl DesignerSession {
    pub(crate) fn scrub_animation_program(&mut self, id: NodeId, time: f64) -> Result<(), String> {
        let program = self.animation_programs().iter().find(|program| program.id == id).ok_or("Unknown animation program")?;
        if !self.has_animation_targets(program) {
            return Err("Choose live preview targets for this animation program".into());
        }
        let duration = self.preview_seek_limit(program)?;
        if !time.is_finite() || time < 0.0 || duration.is_some_and(|duration| time > duration) {
            return Err("Choose a time within the animation duration".into());
        }
        if self.active_animation_program() != Some(id) {
            self.select_animation_program(id)?;
        }
        self.pause();
        self.seek(time)
    }

    pub fn timeline_preview_pose(&self) -> Option<TimelinePreviewPose> {
        self.timeline_pose_at(self.time())
    }

    pub(crate) fn timeline_pose_at(&self, time: f64) -> Option<TimelinePreviewPose> {
        let id = self.active_animation_program()?;
        let program = self
            .animation_programs()
            .iter()
            .find(|program| program.id == id)?;
        let anchor = ProgramLocator::capture(self.document(), program);
        Some(TimelinePreviewPose {
            program: id,
            time,
            source_key: program.source_key.clone(),
            anchor,
            playback: None,
        })
    }

    /// Sample an authored pose without source changes. Missing or ambiguous anchors
    /// leave the new frame in its base state.
    pub fn restore_timeline_preview_pose(
        &mut self,
        pose: &TimelinePreviewPose,
    ) -> Result<Option<TimelinePreviewPose>, String> {
        let program = self.resolve_preview_program(pose.source_key.as_deref(), pose.anchor.as_ref());
        if let Some(playback) = &pose.playback {
            // Play All belongs to the current document even when its reference clip is removed.
            let program = program.map(|program| program.id)
                .or_else(|| playback.all.then_some(self.active_animation_program()).flatten());
            return match program {
                Some(program) => self.restore_playback_pose(program, playback),
                None => Ok(None),
            };
        }
        let Some(program) = program.filter(|program| self.has_animation_targets(program)) else {
            return Ok(None);
        };
        if !pose.time.is_finite() || pose.time < 0.0 {
            return Err("Animation time must be finite and nonnegative".into());
        }
        let limit = self.preview_seek_limit(program)?;
        self.scrub_animation_program(program.id, limit.map_or(pose.time, |limit| pose.time.min(limit)))?;
        Ok(self.timeline_preview_pose())
    }

    pub(crate) fn uses_native_timeline_clock(&self) -> bool {
        self.animation_programs().iter().any(|program| Some(program.id) == self.active_animation_program()
            && self.seeks_animation_elapsed(program))
    }

    fn preview_seek_limit(&self, program: &AnimationProgram) -> Result<Option<f64>, String> {
        if self.seeks_animation_elapsed(program) {
            return self.animation.native.timing(program.id).map(|clock| clock.total_duration())
                .ok_or_else(|| "The authored animation is unavailable in the current native preview".into());
        }
        Ok(program.timeline.as_ref().map(|timeline| timeline.duration))
    }

    pub(crate) fn resolve_preview_program(&self, key: Option<&str>, anchor: Option<&ProgramLocator>) -> Option<&AnimationProgram> {
        let programs = self.animation_programs();
        let mut matching_keys = programs
            .iter()
            .filter(|program| key.is_some() && program.source_key.as_deref() == key);
        match (matching_keys.next(), matching_keys.next()) {
            (Some(program), None) => Some(program),
            // A first authoring edit can add a source key that Undo later removes.
            (None, None) => {
                let anchor = anchor?;
                let matches = |program: &&AnimationProgram| {
                    (key.is_none() || program.source_key.is_none())
                        && stable_name(program) == anchor.name.as_deref()
                        && program.selector == anchor.selector
                };
                if let Some(id) = anchor.source.as_ref().and_then(|source| source.resolve(self.document()).ok()) {
                    return programs.iter().filter(matches).find(|program| program.id == id);
                }
                let owner = anchor.owner.as_ref()?.resolve(self.document()).ok()?;
                let mut candidates = programs.iter().filter(matches).filter(|program| program.owner == owner);
                let only = candidates.next()?;
                candidates.next().is_none().then_some(only)
            }
            _ => None,
        }
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn record_property_at(
        &mut self,
        program: NodeId,
        time: f64,
        nodes: &[NodeId],
        name: &str,
        value: &str,
    ) -> Result<(), String> {
        if nodes.is_empty() {
            return Err("Select one or more animation targets".into());
        }
        for &node in nodes {
            self.validate_recorded_value(node, name, value)?;
        }
        self.record_at(
            program,
            time,
            nodes.iter().map(|&node| (node, name)),
            |session| session.set_property(nodes, name, value),
        )
    }

    pub(crate) fn validate_recorded_value(
        &self,
        node: NodeId,
        property: &str,
        value: &str,
    ) -> Result<(), String> {
        if value.starts_with('{') || !self.can_record_property(node, property) {
            return Err(format!(
                "{property} requires a literal value on an animatable preview property"
            ));
        }
        let animation_value = self.animation_value_for_target(node, property, value)?;
        self.preview
            .prepare_animation_value(node, property, &animation_value)?;
        Ok(())
    }

    pub(crate) fn record_at<S: AsRef<str>>(
        &mut self,
        program: NodeId,
        time: f64,
        properties: impl IntoIterator<Item = (NodeId, S)>,
        apply: impl FnOnce(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        let active_program = self
            .animation_programs()
            .iter()
            .find(|entry| entry.id == program && entry.timeline.is_some())
            .ok_or("Choose an authored animation program with typed keyframes")?;
        let sample = properties.into_iter().find_map(|(node, property)| {
            active_program
                .targets
                .contains(&node)
                .then(|| property.as_ref().to_owned())
        });
        self.scrub_animation_program(program, time)?;
        let was_recording = self.recording();
        self.set_recording(true);
        let result = apply(self);
        self.set_recording(was_recording);
        result?;
        let active = self.active_animation_program();
        let target = self
            .animation_programs()
            .iter()
            .find(|entry| Some(entry.id) == active)
            .map(|entry| entry.primary_target());
        if let (Some(target), Some(property)) = (target, sample) {
            let mut key = ferroui_designer_animation::KeyframeLocation {
                target,
                property,
                time,
            };
            key.reconcile_time(self.timeline());
            if key.time != self.time() {
                self.seek(key.time)?;
            }
        }
        Ok(())
    }
}
