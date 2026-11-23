//! Studio work-area and editing-grid state for the agent and toolbar.
use super::*;
use ferroui_designer_animation::PreviewRange;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum WorkAreaScope {
    Program,
    Clips,
}

#[derive(Deserialize)]
pub(crate) struct WorkAreaCommand {
    scope: WorkAreaScope,
    program: Option<NodeId>,
    invocation: Option<usize>,
    #[serde(flatten)]
    edit: WorkAreaEdit,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum WorkAreaEdit {
    Set { range: PreviewRange },
    Enable,
    Disable,
    SetIn,
    SetOut,
}

#[derive(Deserialize)]
pub(crate) struct SnapCommand {
    program: Option<NodeId>,
    invocation: Option<usize>,
    frame_rate: u32,
}

fn invocation(session: &DesignerSession) -> Option<usize> {
    session.rust_map().map(|map| map.invocation.start)
}

impl DesignerWorkspace {
    fn work_area_subject(&self, session: &DesignerSession) -> (WorkAreaScope, Option<NodeId>) {
        if self.comparing_animation_clips() {
            (WorkAreaScope::Clips, None)
        } else {
            (WorkAreaScope::Program, session.active_animation_program())
        }
    }

    pub(crate) fn timeline_work_area(&self) -> Option<Value> {
        if !self.timeline_visible() {
            return None;
        }
        let session = self.session.borrow();
        let (scope, program) = self.work_area_subject(&session);
        let all = scope == WorkAreaScope::Clips;
        let limit = session.preview_range_limit(program, all).ok()?;
        let current = self.preview_range_for_view(&session);
        let range = current.unwrap_or_else(|| self.default_preview_range(limit));
        range.validate(limit).ok()?;
        let clock = if all || session.uses_native_timeline_clock() {
            "elapsed"
        } else {
            "authored"
        };
        let mut report = json!({"scope":scope, "clock":clock, "range":range, "limit":limit, "playhead":session.time(),
            "enabled":current.is_some(), "editable":!session.is_playing()});
        if let Some(program) = program {
            report["program"] = json!(program);
        }
        if let Some(invocation) = invocation(&session) {
            report["invocation"] = json!(invocation);
        }
        Some(report)
    }

    pub(crate) fn change_timeline_work_area(
        self: &Rc<Self>,
        command: WorkAreaCommand,
    ) -> Result<(), String> {
        {
            let session = self.session.borrow();
            if self.work_area_subject(&session) != (command.scope, command.program)
                || invocation(&session) != command.invocation
            {
                return Err("Read the work area for the visible Timeline clock again.".into());
            }
            session.preview_range_limit(command.program, command.scope == WorkAreaScope::Clips)?;
        }
        match command.edit {
            WorkAreaEdit::Set { range } => self.set_preview_range(Some(range))?,
            WorkAreaEdit::Disable => self.set_preview_range(None)?,
            WorkAreaEdit::SetIn => self.set_preview_range_boundary(true)?,
            WorkAreaEdit::SetOut => self.set_preview_range_boundary(false)?,
            WorkAreaEdit::Enable => {
                let range = {
                    let session = self.session.borrow();
                    let limit = session.preview_range_limit(
                        command.program,
                        command.scope == WorkAreaScope::Clips,
                    )?;
                    self.preview_range_for_view(&session)
                        .unwrap_or_else(|| self.default_preview_range(limit))
                };
                self.set_preview_range(Some(range))?;
            }
        }
        self.set_timeline_visible(true);
        self.refresh(true);
        Ok(())
    }

    pub(crate) fn timeline_snap(&self) -> Option<Value> {
        if !self.timeline_visible() || self.comparing_animation_clips() {
            return None;
        }
        let (rates, editable) = self.timeline_snap_options()?;
        let session = self.session.borrow();
        let rate = self.timeline_frame_rate().unwrap_or(0);
        let mut report = json!({"frame_rate":rate, "rates":rates, "editable":editable});
        if let Some(program) = session.active_animation_program() {
            report["program"] = json!(program);
        }
        if let Some(invocation) = invocation(&session) {
            report["invocation"] = json!(invocation);
        }
        Some(report)
    }

    fn timeline_snap_options(&self) -> Option<(Vec<u32>, bool)> {
        let picker = self.timeline_ui.borrow().snap_picker.as_ref()?.upgrade()?;
        let items = picker.items();
        let rates = (0..items.count())
            .filter_map(|index| {
                items.get_at(index).and_then(|item| {
                    item.downcast_ref::<String>()
                        .and_then(|label| timeline_snap_rate(label))
                })
            })
            .collect();
        Some((rates, picker.is_enabled()))
    }

    pub(crate) fn change_timeline_snap(
        self: &Rc<Self>,
        command: SnapCommand,
    ) -> Result<(), String> {
        {
            let session = self.session.borrow();
            if self.comparing_animation_clips()
                || session.active_animation_program() != command.program
                || invocation(&session) != command.invocation
            {
                return Err("Read the current Studio keyframe snap settings.".into());
            }
        }
        let (rates, _) = self
            .timeline_snap_options()
            .filter(|(_, editable)| *editable)
            .ok_or("Open the Studio Timeline snap control.")?;
        if !rates.contains(&command.frame_rate) {
            return Err("Choose a frame rate listed in the Timeline view.".into());
        }
        self.set_timeline_visible(true);
        self.set_timeline_frame_rate((command.frame_rate > 0).then_some(command.frame_rate));
        Ok(())
    }
}
