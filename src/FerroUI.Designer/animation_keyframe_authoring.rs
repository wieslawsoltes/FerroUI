use crate::DesignerSession;
use ferroui_designer_animation::{
    AnimationError, AnimationValue, KeyframeComponentEdit, KeyframeEdit, KeyframeGroupEdit,
    KeyframeLocation, Timeline,
};
use ferroui_designer_core::NodeId;

/// The value and easing sampled for a source edit at the timeline playhead.
pub struct SampledKeyframe {
    pub(crate) previous: Option<f64>,
    pub(crate) time: f64,
    pub(crate) value: String,
    pub(crate) easing: String,
}

impl SampledKeyframe {
    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn edit(&self) -> KeyframeEdit<'_> {
        KeyframeEdit::Upsert {
            previous: self.previous,
            time: self.time,
            value: &self.value,
            easing: &self.easing,
        }
    }
}

impl DesignerSession {
    pub fn program_keyframe_component(
        &self,
        program: NodeId,
        target: NodeId,
        property: &str,
        previous: f64,
        edit: &KeyframeComponentEdit,
    ) -> Result<String, String> {
        self.program_keyframe_components(program, target, property, previous, std::slice::from_ref(edit))
    }

    pub fn program_keyframe_components(
        &self,
        program: NodeId,
        target: NodeId,
        property: &str,
        previous: f64,
        edits: &[KeyframeComponentEdit],
    ) -> Result<String, String> {
        self.animation_programs().iter().find(|candidate| candidate.id == program)
            .ok_or("Unknown animation program")?
            .keyframe_components(target, property, previous, edits)
            .map(|value| value.to_xaml())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn edit_program_keyframes(
        &mut self,
        id: NodeId,
        keys: &[KeyframeLocation],
        edit: KeyframeGroupEdit,
    ) -> Result<Vec<KeyframeLocation>, String> {
        let program = self
            .animation_programs()
            .iter()
            .find(|program| program.id == id)
            .ok_or("Unknown animation program")?;
        let authored = program
            .timeline
            .as_ref()
            .ok_or("The program has an opaque payload")?;
        let changed = if let Some(rust) = &self.rust {
            let (patches, changed) = rust.keyframes(&self.document, id, authored, keys, edit)?;
            if !patches.is_empty() {
                self.commit_rust_patches("Edit Rust keyframes", patches)
                    .map_err(|error| error.to_string())?;
                self.changed(true);
            }
            changed
        } else {
            let (timeline, changed) = authored
                .with_keyframe_edit(keys, edit)
                .map_err(|error| error.to_string())?;
            if &timeline != authored {
                let retimed = if edit.retimes() {
                    keys.iter().cloned().zip(changed.iter().cloned()).collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                let edits = self.program_edits(id, &timeline, &retimed)?;
                self.apply_program_edits(id, edits)?;
            }
            changed
        };
        Ok(self.accepted_keyframe_locations(id, changed))
    }

    pub(crate) fn edit_program_keyframe(
        &mut self,
        program: NodeId,
        target: NodeId,
        property: &str,
        edit: KeyframeEdit<'_>,
    ) -> Result<(), String> {
        let timeline = self.plan_program_keyframe(program, target, property, edit)?;
        if let Some(rust) = &self.rust {
            let patches = rust.keyframe(&self.document, program, property, edit)?;
            let change = self
                .commit_rust_patches("Edit Rust keyframe", patches)
                .map_err(|error| error.to_string())?;
            if !change.is_empty() {
                self.changed(true);
            }
            return Ok(());
        }
        let retimed = edit.retiming(target, property);
        let edits = self.program_edits(program, &timeline, retimed.as_slice())?;
        self.apply_program_edits(program, edits)
    }

    /// Validate a draft against an accepted program and its loaded property types.
    pub fn plan_program_keyframe(
        &self,
        program: NodeId,
        target: NodeId,
        property: &str,
        edit: KeyframeEdit<'_>,
    ) -> Result<Timeline, String> {
        let program = self
            .animation_programs()
            .iter()
            .find(|candidate| candidate.id == program)
            .ok_or("Unknown animation program")?;
        program
            .plan_keyframe(target, property, edit, |value, kind| {
                let value = match kind {
                    Some(kind) => AnimationValue::parse_native_literal(value, kind)?,
                    None => self
                        .animation_value_for_target(target, property, value)
                        .map_err(AnimationError)?,
                };
                if !program.has_project_codec() && self.can_record_property(target, property) {
                    self.preview
                        .prepare_animation_value(target, property, &value)
                        .map_err(AnimationError)?;
                }
                Ok(value)
            })
            .map_err(|error| error.to_string())
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn preview_program_keyframe(
        &self,
        program: NodeId,
        target: NodeId,
        property: &str,
        draft: KeyframeEdit<'_>,
    ) -> Result<serde_json::Value, String> {
        let KeyframeEdit::Upsert { time, .. } = draft else {
            return Err("Supply a keyframe draft".into());
        };
        let timeline = self.plan_program_keyframe(program, target, property, draft)?;
        if let Some(rust) = &self.rust {
            if !matches!(self.preview_status(), crate::PreviewStatus::Ready { revision }
                if *revision == self.document.revision())
            {
                return Err("Wait for the current RSX preview".into());
            }
            rust.keyframe(&self.document, program, property, draft)?;
        } else {
            self.program_edits(program, &timeline, draft.retiming(target, property).as_slice())?;
        }
        serde_json::to_value(
            timeline
                .inspect_keyframe(target, property, time)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }
}
