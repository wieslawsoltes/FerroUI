use crate::{DesignerSession, PreviewSourceAnchor};
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
    pub anchor: Option<PreviewSourceAnchor>,
}

impl DesignerSession {
    pub(crate) fn scrub_animation_program(&mut self, id: NodeId, time: f64) -> Result<(), String> {
        let duration = self
            .animation_programs()
            .iter()
            .find(|program| program.id == id)
            .ok_or("Unknown animation program")?
            .timeline
            .as_ref()
            .map(|timeline| timeline.duration);
        if !time.is_finite() || time < 0.0 || duration.is_some_and(|duration| time > duration) {
            return Err("Choose a time within the animation duration".into());
        }
        if self.active_animation_program() != Some(id) {
            self.select_animation_program(id)?;
        }
        if time > self.timeline().duration {
            return Err("Choose a time within the animation duration".into());
        }
        self.pause();
        self.seek(time)
    }

    pub fn timeline_preview_pose(&self) -> Option<TimelinePreviewPose> {
        let id = self.active_animation_program()?;
        let program = self
            .animation_programs()
            .iter()
            .find(|program| program.id == id)?;
        let timeline = program.timeline.as_ref()?;
        let source_key = program.source_key.clone();
        let anchor = if source_key.is_none() {
            PreviewSourceAnchor::capture(self.document(), id).ok()
        } else {
            None
        };
        Some(TimelinePreviewPose {
            program: id,
            time: self.time().min(timeline.duration),
            source_key,
            anchor,
        })
    }

    /// Sample an authored pose without source changes. Missing or ambiguous anchors
    /// leave the new frame in its base state.
    pub fn restore_timeline_preview_pose(
        &mut self,
        pose: &TimelinePreviewPose,
    ) -> Result<Option<TimelinePreviewPose>, String> {
        let program = if let Some(key) = &pose.source_key {
            let mut matches = self
                .animation_programs()
                .iter()
                .filter(|program| program.source_key.as_ref() == Some(key));
            let candidate = matches.next();
            if matches.next().is_some() {
                None
            } else {
                candidate
            }
        } else {
            let id = pose
                .anchor
                .as_ref()
                .and_then(|anchor| anchor.resolve(self.document()).ok());
            self.animation_programs()
                .iter()
                .find(|program| Some(program.id) == id)
        };
        let Some((id, duration)) = program.and_then(|program| {
            program
                .timeline
                .as_ref()
                .map(|timeline| (program.id, timeline.duration))
        }) else {
            return Ok(None);
        };
        if !pose.time.is_finite() || pose.time < 0.0 {
            return Err("Animation time must be finite and nonnegative".into());
        }
        self.scrub_animation_program(id, pose.time.min(duration))?;
        Ok(self.timeline_preview_pose())
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
        if nodes.is_empty()
            || value.starts_with('{')
            || nodes
                .iter()
                .any(|&node| !self.can_record_property(node, name))
        {
            return Err("Record a literal on one or more animatable preview properties".into());
        }
        let active_program = self
            .animation_programs()
            .iter()
            .find(|entry| entry.id == program && entry.timeline.is_some())
            .ok_or("Choose an authored animation program with typed keyframes")?;
        let records_on_active_program = nodes
            .iter()
            .any(|node| active_program.targets.contains(node));
        for &node in nodes {
            let animation_value = self.animation_value_for_target(node, name, value)?;
            self.preview
                .prepare_animation_value(node, name, &animation_value)?;
        }
        self.scrub_animation_program(program, time)?;
        let was_recording = self.recording();
        self.set_recording(true);
        let result = self.set_property(nodes, name, value);
        self.set_recording(was_recording);
        result?;
        let active = self
            .active_animation_program()
            .filter(|_| records_on_active_program);
        if let Some(target) = self
            .animation_programs()
            .iter()
            .find(|entry| Some(entry.id) == active)
            .map(|entry| entry.primary_target())
        {
            let mut key = ferroui_designer_animation::KeyframeLocation {
                target,
                property: name.into(),
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
