use super::{FRAME_TIMEOUT, Relay, hosted_animation, send};
use ferroui_designer_animation::{KeyframeLocation, Timeline};
use ferroui_remote_protocol::designer::DesignKeyframeSelectionRequest;
use serde_json::{Value, json};
use std::sync::Arc;

impl Relay {
    pub(crate) fn plan_keyframe_selection(
        &mut self,
        draft: hosted_animation::KeyframeSelectionDraft,
    ) -> Result<Value, String> {
        let connection = self.animation_connection(draft.revision, draft.sequence)?;
        let command = serde_json::to_string(&draft.command).map_err(|error| error.to_string())?;
        let request = self.next_animation_request()?;
        {
            let mut state = self.events.0.lock().unwrap();
            if !state.keyframe_selection || !state.authorable.contains(&draft.program) {
                return Err(
                    "Choose an editable animation in an app with keyframe selection support".into(),
                );
            }
            state.animation_request = Some(request);
            state.keyframe_selection_result = None;
        }
        send(
            &connection,
            Arc::new(DesignKeyframeSelectionRequest {
                revision: draft.revision,
                request,
                program: draft.program,
                command,
                projected: self.authored_rsx.is_some(),
            }),
        )?;
        let (mut state, _) = self
            .events
            .1
            .wait_timeout_while(self.events.0.lock().unwrap(), FRAME_TIMEOUT, |state| {
                !state.disconnected && state.keyframe_selection_result.is_none()
            })
            .unwrap();
        state.animation_request = None;
        let result = state
            .keyframe_selection_result
            .take()
            .ok_or("The application did not return the keyframe selection plan")?;
        drop(state);
        if let Some(error) = result.error {
            return Err(error);
        }
        let mut keys = result
            .keys
            .as_deref()
            .map(serde_json::from_str::<Vec<KeyframeLocation>>)
            .transpose()
            .map_err(|error| error.to_string())?;
        let mut patch = result.patch;
        if let Some(authored) = &self.authored_rsx {
            let timeline: Timeline = serde_json::from_str(
                result
                    .timeline
                    .as_deref()
                    .ok_or("The application did not return the accepted animation")?,
            )
            .map_err(|error| error.to_string())?;
            (patch, keys) =
                authored.hosted_keyframe_selection(draft.program, &draft.command, &timeline)?;
        }
        Ok(json!({
            "patch": patch.map(hosted_animation::source_patch),
            "keys": keys,
            "clipboard": result.clipboard,
        }))
    }
}
