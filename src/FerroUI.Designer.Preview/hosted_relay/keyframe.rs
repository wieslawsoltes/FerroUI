use super::{hosted_animation, send, Relay, FRAME_TIMEOUT};
use ferroui_designer_animation::{KeyframeComponentEdit, KeyframeEdit};
use ferroui_remote_protocol::designer::{DesignAnimationAuthoringRequest, DesignKeyframeComponent};
use std::sync::Arc;

impl Relay {
    pub(crate) fn plan_keyframe(
        &mut self,
        draft: hosted_animation::KeyframeDraft,
    ) -> Result<serde_json::Value, String> {
        let connection = self.animation_connection(draft.revision, draft.sequence)?;
        let recording = draft.sample == Some(true);
        let has_components = draft.component.is_some() || draft.components.is_some();
        let (value, easing) = match (recording, draft.value, draft.easing, has_components) {
            (true, None, None, false) if draft.previous.is_none() => (String::new(), String::new()),
            (false, Some(value), Some(easing), false) if draft.preview_target.is_none() => (value, easing),
            (false, None, Some(easing), true) if draft.previous.is_some() && draft.preview_target.is_none()
                && !(draft.component.is_some() && draft.components.is_some()) => (String::new(), easing),
            _ => {
                return Err(
                    "Choose a new sampled keyframe or supply an explicit keyframe draft".into(),
                )
            }
        };
        let request = self.next_animation_request()?;
        let command = Arc::new(DesignAnimationAuthoringRequest {
            revision: draft.revision,
            request,
            program: draft.program,
            target: draft.target,
            property: draft.property,
            previous: draft.previous,
            time: draft.time,
            value,
            easing,
            sample: recording.then_some(true),
            sample_target: draft.preview_target,
            component: draft.component.map(component_request).transpose()?,
            components: draft.components.map(|edits| edits.into_iter().map(component_request).collect()).transpose()?,
        });
        {
            let mut state = self.events.0.lock().unwrap();
            if !state.authorable.contains(&command.program) {
                return Err(
                    "Choose an editable animation in the current application preview".into(),
                );
            }
            if command.component.is_some() && !state.component_keys {
                return Err("Use an application preview with component keyframe authoring".into());
            }
            if command.components.is_some() && !state.component_key_batch {
                return Err("Use an application preview with batch keyframe authoring".into());
            }
            if recording
                && !state.sampleable_keys.contains(&(
                    command.program,
                    command.target,
                    command.property.clone(),
                ))
            {
                return Err(
                    "Choose a sampleable authored track in the current application preview".into(),
                );
            }
            state.animation_request = Some(request);
            state.keyframe_result = None;
        }
        send(&connection, command.clone())?;
        let (mut state, _) = self
            .events
            .1
            .wait_timeout_while(self.events.0.lock().unwrap(), FRAME_TIMEOUT, |state| {
                !state.disconnected && state.keyframe_result.is_none()
            })
            .unwrap();
        state.animation_request = None;
        let mut result = state
            .keyframe_result
            .take()
            .ok_or("The application did not return an animation edit plan")?;
        drop(state);
        if let Some(error) = result.error.take() {
            return Err(error);
        }
        let (time, value, easing) = if recording || has_components {
            let sampled = if recording { result.sampled_key.as_ref() } else { result.resolved_key.as_ref() }
                .ok_or("The application did not return the planned keyframe value")?;
            (
                sampled.time,
                sampled.value.as_str(),
                sampled.easing.as_str(),
            )
        } else {
            (
                command.time,
                command.value.as_str(),
                command.easing.as_str(),
            )
        };
        if let Some(source) = &self.authored_rsx {
            result.patch = source.hosted_patch(
                command.program,
                &command.property,
                KeyframeEdit::Upsert {
                    previous: command.previous,
                    time,
                    value,
                    easing,
                },
            )?;
        }
        Ok(hosted_animation::keyframe_plan(result))
    }
}

fn component_request(edit: KeyframeComponentEdit) -> Result<DesignKeyframeComponent, String> {
    let channel = serde_json::to_value(edit.channel).and_then(serde_json::from_value)
        .map_err(|error| error.to_string())?;
    Ok(DesignKeyframeComponent { channel, value: edit.value, step: None })
}
