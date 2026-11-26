//! The agent uses the native track header and component picker actions.
use super::*;
use serde::Deserialize;
use serde_json::{json, Value};

pub(super) struct CurvePicker {
    pub target: NodeId,
    pub property: String,
    pub context: TimelineContext,
    pub choices: Vec<TrackCurveChannel>,
    pub picker: WeakRef<ComboBox>,
}

#[derive(Deserialize)]
pub(crate) struct TimelineTrackCommand {
    program: Option<NodeId>,
    target: NodeId,
    property: String,
    #[serde(flatten)]
    action: TrackAction,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum TrackAction {
    Select,
    Channel { channel: TrackCurveChannel },
}

impl DesignerWorkspace {
    pub(crate) fn timeline_track_view(&self) -> Option<Value> {
        if !self.timeline_visible() || self.comparing_animation_clips() {
            return None;
        }
        let session = self.session.borrow();
        let timeline = session.timeline();
        if timeline.tracks.is_empty() {
            return None;
        }
        let ui = self.timeline_ui.borrow();
        let tracks: Vec<_> = timeline
            .tracks
            .iter()
            .map(|track| {
                let mut view = json!({
                    "target": track.target, "property": track.property, "visible": true,
                    "name": timeline_target_name(session.document().ast(), track.target)
                });
                if ui.curves.is_some() {
                    let picker = ui.curve_pickers.iter().find(|picker| {
                        picker.target == track.target && picker.property == track.property
                    });
                    let choices = picker.map_or_else(
                        || TrackCurveChannel::for_track(track, timeline.duration, timeline.fill),
                        |picker| picker.choices.clone(),
                    );
                    let control = picker.and_then(|picker| picker.picker.upgrade());
                    let index = control
                        .as_ref()
                        .map_or(0, |picker| picker.selected_index().max(0) as usize);
                    if let Some(channel) = choices.get(index) {
                        view["channel"] = json!(channel);
                        view["channels"] = json!(choices);
                        view["canChooseChannel"] =
                            json!(control.is_some_and(|picker| picker.is_enabled()));
                    }
                }
                view
            })
            .collect();
        let mut view = json!({ "tracks": tracks });
        if let Some(program) = session.active_animation_program() {
            view["program"] = json!(program);
        }
        if let Some(key) = &ui.selected {
            view["selected"] = json!({ "target": key.target, "property": key.property });
        }
        Some(view)
    }

    pub(crate) fn change_timeline_tracks(
        self: &Rc<Self>,
        command: TimelineTrackCommand,
    ) -> Result<(), String> {
        let time = {
            let session = self.session.borrow();
            if self.comparing_animation_clips()
                || session.active_animation_program() != command.program
            {
                return Err(
                    "Read the visible Studio Timeline tracks and current program again.".into(),
                );
            }
            if session
                .timeline()
                .track(command.target, &command.property)
                .is_none()
            {
                return Err("Choose a property and target from the current Timeline.".into());
            }
            session.time()
        };
        if let TrackAction::Channel { channel } = command.action {
            let (picker, index) = {
                let ui = self.timeline_ui.borrow();
                let entry = ui
                    .curve_pickers
                    .iter()
                    .find(|entry| {
                        entry.target == command.target && entry.property == command.property
                    })
                    .ok_or("Show the track and its curve component picker first.")?;
                entry.context.check(self)?;
                let picker = entry
                    .picker
                    .upgrade()
                    .filter(|picker| picker.is_enabled())
                    .ok_or("Show the track and its curve component picker first.")?;
                let index = entry
                    .choices
                    .iter()
                    .position(|choice| *choice == channel)
                    .ok_or("Choose a component from the visible curve.")?;
                (picker, index)
            };
            self.set_timeline_visible(true);
            picker.set_selected_index(index as i32);
            Ok(())
        } else {
            self.set_timeline_visible(true);
            self.activate_timeline_key(command.target, &command.property, time)
        }
    }
}
