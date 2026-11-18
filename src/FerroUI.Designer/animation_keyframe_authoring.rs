use crate::DesignerSession;
use ferroui_designer_animation::{AnimationValue, Easing, KeyframeGroupEdit, KeyframeLocation, Timeline};
use ferroui_designer_core::NodeId;

#[derive(Clone, Copy)]
pub(crate) enum KeyframeEdit<'a> {
    Upsert {
        previous: Option<f64>,
        time: f64,
        value: &'a str,
        easing: &'a str,
    },
    Remove(f64),
}

impl DesignerSession {
    pub(crate) fn edit_program_keyframes(
        &mut self,
        id: NodeId,
        keys: &[KeyframeLocation],
        edit: KeyframeGroupEdit,
    ) -> Result<Vec<KeyframeLocation>, String> {
        let program = self.animation_programs().iter().find(|program| program.id == id)
            .ok_or("Unknown animation program")?;
        let authored = program.timeline.as_ref().ok_or("The program has an opaque payload")?;
        let (timeline, mut changed) = authored.with_keyframe_edit(keys, edit).map_err(|error| error.to_string())?;
        if &timeline != authored {
            self.update_animation_program(id, timeline)?;
        }
        if let Some(accepted) = self.animation_programs().iter().find(|program| program.id == id)
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
        let mut timeline = program
            .timeline
            .clone()
            .ok_or("The program has an opaque payload")?;
        let track = timeline.track(target, property);
        match edit {
            KeyframeEdit::Upsert {
                previous,
                time,
                value,
                easing,
            } => {
                if !time.is_finite() || !(0.0..=timeline.duration).contains(&time) {
                    return Err("Keyframe is outside the timeline".into());
                }
                let authored = track.and_then(|track| track.keyframes.first());
                if authored.is_none() && target != program.primary_target() {
                    return Err("Create tracks on the program's primary target".into());
                }
                if previous.is_none()
                    && track.is_some_and(|track| track.keyframes.iter().any(|key| key.time == time))
                {
                    return Err("Another keyframe occupies the destination time".into());
                }
                let value = match authored {
                    Some(key) => AnimationValue::parse(value, key.value.kind())
                        .map_err(|error| error.to_string())?,
                    None => self.animation_value_for_target(target, property, value)?,
                };
                if self.can_record_property(target, property) {
                    self.preview.prepare_animation_value(target, property, &value)?;
                }
                let easing = Easing::parse(easing).map_err(|error| error.to_string())?;
                if let Some(previous) = previous {
                    timeline
                        .track_mut(target, property)
                        .ok_or("Animation track is no longer present")?
                        .move_keyframe(previous, time)
                        .map_err(|error| error.to_string())?;
                }
                timeline
                    .record(target, property, time, value)
                    .map_err(|error| error.to_string())?;
                timeline
                    .track_mut(target, property)
                    .expect("recorded track")
                    .set_easing(time, easing)
                    .map_err(|error| error.to_string())?;
            }
            KeyframeEdit::Remove(time) => {
                let track = timeline
                    .track_mut(target, property)
                    .ok_or("Animation track is no longer present")?;
                track
                    .remove_keyframe(time)
                    .map_err(|error| error.to_string())?;
                timeline.tracks.retain(|track| !track.keyframes.is_empty());
            }
        }
        Ok(timeline)
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn preview_program_keyframe(
        &self,
        program: NodeId,
        target: NodeId,
        property: &str,
        draft: KeyframeEdit<'_>,
    ) -> Result<serde_json::Value, String> {
        let KeyframeEdit::Upsert { time, .. } = draft else { return Err("Supply a keyframe draft".into()); };
        let timeline = self.plan_program_keyframe(program, target, property, draft)?;
        self.program_edits(program, &timeline)?;
        let key = timeline.track(target, property).and_then(|track| track.keyframes.iter().find(|key| key.time == time))
            .ok_or("The draft did not produce a keyframe")?;
        let samples = key.easing.preview_samples().map_err(|error| error.to_string())?;
        Ok(serde_json::json!({"valueKind": key.value.kind(), "samples": samples}))
    }
}
