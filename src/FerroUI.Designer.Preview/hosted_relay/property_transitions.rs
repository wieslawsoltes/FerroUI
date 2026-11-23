//! Property transition queries and checked patches for the accepted app frame.
use super::{send, Relay, FRAME_TIMEOUT};
use crate::hosted_animation::{checked_source_patch, source_patch};
use crate::hosted_frame::FrameIdentity;
use crate::hosted_relay::authored::Request;
use ferroui_designer::mcp::{authored_property_transition_report, mapped_property_transition_plan};
use ferroui_designer_animation::native::TransitionPropertyChoice;
use ferroui_designer_animation::{PropertyTransitionCommand, PROPERTY_TRANSITION_TYPES};
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_remote_protocol::designer::{
    DesignPropertyTransitionsRequest, DesignPropertyTransitionsResult,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransitionAction {
    owner: NodeId,
    target_type: Option<String>,
    command: Option<PropertyTransitionCommand>,
}

impl Relay {
    pub(crate) fn property_transitions(
        &mut self,
        request: Request<TransitionAction>,
    ) -> Result<Value, String> {
        let Request { frame, action } = request;
        let TransitionAction {
            owner,
            target_type,
            command,
        } = action;
        let is_rust = self.authored_rsx.is_some();
        if is_rust && target_type.is_some() {
            return Err("The mapped Rust control supplies the transition target type".into());
        }
        let remote_command = if is_rust {
            None
        } else {
            command
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| error.to_string())?
        };
        let response =
            self.request_property_transitions(frame, owner, target_type, remote_command)?;
        let options = response
            .properties
            .into_iter()
            .filter_map(|property| {
                let type_name = PROPERTY_TRANSITION_TYPES
                    .iter()
                    .copied()
                    .find(|name| *name == property.type_name)?;
                Some(TransitionPropertyChoice {
                    property: property.property,
                    label: property.label,
                    type_name,
                })
            })
            .collect::<Vec<_>>();
        let document = self.animation_sources.authoring_document(frame)?;
        if let Some(rust) = &self.authored_rsx {
            let projection = rust.projection()?;
            if let Some(command) = command {
                let patches = mapped_property_transition_plan(
                    document,
                    owner,
                    &projection,
                    &options,
                    &command,
                )?;
                let updated = TextPatch::apply_all(rust.source(), patches)
                    .map_err(|error| error.to_string())?;
                let patch = checked_source_patch(rust.source(), &updated)?;
                return Ok(
                    json!({"patches": patch.into_iter().map(source_patch).collect::<Vec<_>>() }),
                );
            }
            return authored_property_transition_report(
                document,
                rust.source(),
                owner,
                Some(&projection),
                options,
            );
        }
        if command.is_some() {
            return Ok(
                json!({"patches": response.patch.into_iter().map(source_patch).collect::<Vec<_>>() }),
            );
        }
        authored_property_transition_report(document, document.valid_source(), owner, None, options)
    }

    fn request_property_transitions(
        &mut self,
        frame: FrameIdentity,
        owner: NodeId,
        target_type: Option<String>,
        command: Option<String>,
    ) -> Result<DesignPropertyTransitionsResult, String> {
        let connection = self.animation_connection(frame.revision, frame.sequence)?;
        let owner = i64::try_from(owner).map_err(|_| "Choose an authored transition owner")?;
        let request = self.next_animation_request()?;
        {
            let mut state = self.events.0.lock().unwrap();
            if !state.property_transitions {
                return Err(
                    "Use an application preview with property transition authoring support".into(),
                );
            }
            state.animation_request = Some(request);
            state.property_transition_result = None;
        }
        send(
            &connection,
            Arc::new(DesignPropertyTransitionsRequest {
                revision: frame.revision,
                request,
                owner,
                target_type,
                command,
            }),
        )?;
        let (mut state, _) = self
            .events
            .1
            .wait_timeout_while(self.events.0.lock().unwrap(), FRAME_TIMEOUT, |state| {
                !state.disconnected && state.property_transition_result.is_none()
            })
            .unwrap();
        state.animation_request = None;
        let result = state
            .property_transition_result
            .take()
            .ok_or("The application did not return the transition collection")?;
        match result.error {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }
}
