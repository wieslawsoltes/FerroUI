#[path = "keyframe_selection.rs"]
mod keyframe_selection;

use super::{send, Connection, Relay, FRAME_TIMEOUT};
use crate::{hosted_animation, hosted_animation_restore::Presentation};
use ferroui_designer_animation::{CurveChannel, CurvePoint};
use ferroui_remote_protocol::designer::{
    DesignAnimationAuthoringRequest, DesignAnimationClipBinding, DesignAnimationCurveRequest, DesignAnimationRequestMessage,
    DesignAnimationResultMessage,
};
use std::sync::Arc;

impl Relay {
    pub(super) fn restore_animation(
        &mut self,
        connection: &Connection,
        revision: i64,
        presentation: &mut Presentation,
    ) -> Result<(), String> {
        let Some(position) = &mut presentation.position else {
            return Ok(());
        };
        // The base frame stays in the relay. Only the sample from the
        // rebuilt artboard is delivered to the editor.
        let clips = position.clips.as_ref().map(|clips| clips.iter().map(|&program| {
            DesignAnimationClipBinding { program, targets: presentation.overrides.get(&program).cloned() }
        }).collect());
        let program = clips.is_none().then_some(position.program);
        let outcome = self.request_animation(
            connection,
            DesignAnimationRequestMessage {
                revision,
                program,
                targets: program.and_then(|id| presentation.overrides.get(&id).cloned()),
                clips,
                elapsed: position.elapsed,
                restart: true,
                ..Default::default()
            },
        )?;
        position.elapsed = outcome.elapsed;
        if position.preview_range.is_none() {
            position.completed = outcome.completed;
            position.playing &= !outcome.completed;
        }
        Ok(())
    }

    fn animation_connection(&self, revision: i64, sequence: i64) -> Result<Connection, String> {
        if self.sent != Some((revision, sequence)) {
            return Err("The application preview frame changed".into());
        }
        let connection = self
            .connection
            .clone()
            .ok_or("Open the application preview first")?;
        let state = self.events.0.lock().unwrap();
        if !state.animation_supported
            || state.requested != Some(revision)
            || !matches!(state.outcome, Some(Ok(())))
            || state.disconnected
        {
            return Err("Open a current application preview with native animation support".into());
        }
        Ok(connection)
    }

    fn next_animation_request(&mut self) -> Result<i64, String> {
        self.next_animation = self
            .next_animation
            .checked_add(1)
            .ok_or("The animation session retired")?;
        Ok(self.next_animation)
    }

    pub(crate) fn animate(
        &mut self,
        command: hosted_animation::Command,
    ) -> Result<hosted_animation::Sample, String> {
        let connection = self.animation_connection(command.revision, command.sequence)?;
        if command.clips.is_some() && !self.animation_sources.supports_shared_playback() {
            return Err("The application preview does not expose shared clip playback".into());
        }
        let result = self.request_animation(
            &connection,
            DesignAnimationRequestMessage {
                revision: command.revision,
                program: command.program,
                targets: command.targets,
                clips: command.clips.map(|clips| clips.into_iter().map(Into::into).collect()),
                elapsed: command.elapsed,
                restart: command.restart,
                ..Default::default()
            },
        )?;
        let frame = self
            .frame()?
            .ok_or("The application animation frame retired")?;
        Ok(hosted_animation::Sample {
            frame,
            elapsed: result.elapsed,
            completed: result.completed,
        })
    }

    fn request_animation(
        &mut self,
        connection: &Connection,
        mut command: DesignAnimationRequestMessage,
    ) -> Result<DesignAnimationResultMessage, String> {
        let request = self.next_animation_request()?;
        command.request = request;
        {
            let mut state = self.events.0.lock().unwrap();
            state.animation_request = Some(request);
            state.animation_result = None;
        }
        send(connection, Arc::new(command))?;
        let (mut state, _) = self
            .events
            .1
            .wait_timeout_while(self.events.0.lock().unwrap(), FRAME_TIMEOUT, |state| {
                !state.disconnected
                    && state.animation_result.as_ref().is_none_or(|result| {
                        result.error.is_none()
                            && state.latest.as_ref().is_none_or(|frame| {
                                frame.frame.sequence_id <= result.after_sequence
                            })
                    })
            })
            .unwrap();
        state.animation_request = None;
        let result = state
            .animation_result
            .take()
            .ok_or("The application did not acknowledge the animation sample")?;
        if let Some(error) = result.error {
            return Err(error);
        }
        if state
            .latest
            .as_ref()
            .is_none_or(|frame| frame.frame.sequence_id <= result.after_sequence)
        {
            return Err("The application did not render the requested animation sample".into());
        }
        Ok(result)
    }

    pub(crate) fn plan_keyframe(
        &mut self,
        draft: hosted_animation::KeyframeDraft,
    ) -> Result<serde_json::Value, String> {
        let connection = self.animation_connection(draft.revision, draft.sequence)?;
        let authored_patch = self.authored_rsx.as_ref().map(|source| source.hosted_patch(&draft)).transpose()?;
        let request = self.next_animation_request()?;
        {
            let mut state = self.events.0.lock().unwrap();
            if !state.authorable.contains(&draft.program) {
                return Err(
                    "Choose an editable animation in the current application preview".into(),
                );
            }
            state.animation_request = Some(request);
            state.keyframe_result = None;
        }
        send(
            &connection,
            Arc::new(DesignAnimationAuthoringRequest {
                revision: draft.revision,
                request,
                program: draft.program,
                target: draft.target,
                property: draft.property,
                previous: draft.previous,
                time: draft.time,
                value: draft.value,
                easing: draft.easing,
            }),
        )?;
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
        if let Some(patch) = authored_patch {
            result.patch = patch;
        }
        hosted_animation::keyframe(result)
    }

    pub(crate) fn plan_state(&self, draft: hosted_animation::StateDraft) -> Result<serde_json::Value, String> {
        self.animation_connection(draft.revision, draft.sequence)?;
        self.animation_sources.plan_state_route(&draft, self.authored_rsx.as_ref())
    }

    pub(crate) fn plan_state_group(&self, draft: hosted_animation::StateGroupDraft) -> Result<serde_json::Value, String> {
        self.animation_connection(draft.revision, draft.sequence)?;
        self.animation_sources.plan_state_group(&draft, self.authored_rsx.as_ref())
    }

    pub(crate) fn plan_clip_placement(&self, draft: hosted_animation::ClipDraft) -> Result<serde_json::Value, String> {
        self.animation_connection(draft.revision, draft.sequence)?;
        self.animation_sources.plan_clip_placement(&draft, self.authored_rsx.as_ref())
    }

    pub(crate) fn plan_clip_edit(&self, draft: hosted_animation::ClipEditDraft) -> Result<serde_json::Value, String> {
        self.animation_connection(draft.revision, draft.sequence)?;
        self.animation_sources.plan_clip_edit(&draft, self.authored_rsx.as_ref())
    }

    pub(crate) fn curve(
        &mut self,
        query: hosted_animation::CurveRequest,
    ) -> Result<Vec<CurvePoint>, String> {
        let channel = if query.channel == CurveChannel::Value {
            None
        } else {
            serde_json::to_value(query.channel).map_err(|error| error.to_string())?.as_str().map(str::to_owned)
        };
        let connection = self.animation_connection(query.revision, query.sequence)?;
        let request = self.next_animation_request()?;
        {
            let mut state = self.events.0.lock().unwrap();
            if !state.curve_programs.contains(&query.program) {
                return Err("Choose a program with app-hosted animation curves".into());
            }
            state.animation_request = Some(request);
            state.curve_result = None;
        }
        send(&connection, Arc::new(DesignAnimationCurveRequest {
            revision: query.revision,
            request,
            program: query.program,
            target: query.target,
            property: query.property,
            channel,
            start: query.window.start,
            end: query.window.end,
        }))?;
        let (mut state, _) = self.events.1
            .wait_timeout_while(self.events.0.lock().unwrap(), FRAME_TIMEOUT,
                |state| !state.disconnected && state.curve_result.is_none())
            .unwrap();
        state.animation_request = None;
        let result = state.curve_result.take().ok_or("The application did not return curve samples")?;
        if let Some(error) = result.error {
            return Err(error);
        }
        let points = result.points.into_iter().map(|point| CurvePoint {
            break_before: point.break_before.unwrap_or(false),
            ..CurvePoint::new(point.time, point.value)
        }).collect();
        query.metric.measure(points).map_err(|error| error.to_string())
    }
}
