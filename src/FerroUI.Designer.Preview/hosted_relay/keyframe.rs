use super::{hosted_animation, send, Relay, FRAME_TIMEOUT};
use ferroui_designer_animation::KeyframeEdit;
use ferroui_remote_protocol::designer::DesignAnimationAuthoringRequest;
use std::sync::Arc;

impl Relay {
    pub(crate) fn plan_keyframe(
        &mut self,
        draft: hosted_animation::KeyframeDraft,
    ) -> Result<serde_json::Value, String> {
        let connection = self.animation_connection(draft.revision, draft.sequence)?;
        let recording = draft.sample == Some(true);
        let (value, easing) = match (recording, draft.value, draft.easing) {
            (true, None, None) if draft.previous.is_none() => (String::new(), String::new()),
            (false, Some(value), Some(easing)) if draft.preview_target.is_none() => (value, easing),
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
        });
        {
            let mut state = self.events.0.lock().unwrap();
            if !state.authorable.contains(&command.program) {
                return Err(
                    "Choose an editable animation in the current application preview".into(),
                );
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
        let (time, value, easing) = if recording {
            let sampled = result
                .sampled_key
                .as_ref()
                .ok_or("The application did not return the sampled keyframe")?;
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
