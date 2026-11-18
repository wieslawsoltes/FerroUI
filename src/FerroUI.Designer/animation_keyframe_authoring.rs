use crate::DesignerSession;
use ferroui_designer_animation::{
    AnimationError, AnimationValue, KeyframeEdit, KeyframeGroupEdit, KeyframeLocation, Timeline,
};
use ferroui_designer_core::NodeId;

impl DesignerSession {
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
        let (timeline, mut changed) = authored
            .with_keyframe_edit(keys, edit)
            .map_err(|error| error.to_string())?;
        if &timeline != authored {
            self.update_animation_program(id, timeline)?;
        }
        if let Some(accepted) = self
            .animation_programs()
            .iter()
            .find(|program| program.id == id)
            .and_then(|program| program.timeline.as_ref())
        {
            for key in &mut changed {
                key.reconcile_time(accepted);
            }
        }
        Ok(changed)
    }

    pub(crate) fn edit_program_keyframe(
        &mut self,
        program: NodeId,
        target: NodeId,
        property: &str,
        edit: KeyframeEdit<'_>,
    ) -> Result<(), String> {
        let timeline = self.plan_program_keyframe(program, target, property, edit)?;
        self.update_animation_program(program, timeline)
    }

    fn plan_program_keyframe(
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
                    Some(kind) => AnimationValue::parse(value, kind)?,
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
        self.program_edits(program, &timeline)?;
        serde_json::to_value(
            timeline
                .inspect_keyframe(target, property, time)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }
}
