use crate::DesignerSession;
use ferroui_designer_animation::{AnimationValue, Easing};
use ferroui_designer_core::NodeId;

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
    pub(crate) fn edit_program_keyframe(
        &mut self,
        program: NodeId,
        target: NodeId,
        property: &str,
        edit: KeyframeEdit<'_>,
    ) -> Result<(), String> {
        let program = self
            .animation_programs()
            .iter()
            .find(|candidate| candidate.id == program)
            .ok_or("Unknown animation program")?;
        let id = program.id;
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
        self.update_animation_program(id, timeline)
    }
}
