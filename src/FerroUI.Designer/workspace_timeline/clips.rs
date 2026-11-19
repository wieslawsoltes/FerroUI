//! The clip comparison view shares Studio's ruler, clock and source transaction.
#[path = "clips/placement.rs"]
mod placement;

use super::*;
use ferroui_designer_animation::ClipSpan;

struct ClipRow {
    id: NodeId,
    name: String,
    span: ClipSpan,
    editable: bool,
    length: f64,
}

impl DesignerWorkspace {
    pub(in crate::workspace) fn comparing_animation_clips(&self) -> bool {
        self.timeline_ui.borrow().compare_clips
    }

    pub(in crate::workspace) fn timeline_view_duration(&self) -> f64 {
        let ui = self.timeline_ui.borrow();
        if ui.compare_clips {
            ui.clip_duration
        } else {
            drop(ui);
            let session = self.session.borrow();
            session.timeline().duration.max(self.preview_range_for_view(&session).map_or(0.0, |range| range.end))
        }
    }

    pub(in crate::workspace) fn toggle_clip_comparison(self: &Rc<Self>) {
        let mut ui = self.timeline_ui.borrow_mut();
        ui.compare_clips = !ui.compare_clips;
        ui.view = None;
        let comparing = ui.compare_clips;
        drop(ui);
        let mut session = self.session.borrow_mut();
        session.pause();
        if comparing {
            session.set_recording(false);
        }
        if !comparing && session.animation.play_all {
            session.stop_animation_playback();
        }
        drop(session);
        let toggle = self.root.get_control::<Button>("TimelineClips");
        toggle.classes().set("active", comparing);
        let label = if comparing {
            "Return to animation keyframes"
        } else {
            "Compare animation clips"
        };
        AutomationProperties::set_name(&toggle, Some(label));
        ToolTip::set_tip(&toggle, Some(Rc::new(label.to_owned())));
        self.refresh(true);
    }

    pub(in crate::workspace) fn seek_workspace_timeline(&self, time: f64) -> Result<(), String> {
        if self.comparing_animation_clips() {
            let limit = self.timeline_view_duration();
            if !time.is_finite() || !(0.0..=limit).contains(&time) {
                return Err("Choose a time within the clip ruler".into());
            }
            self.session.borrow_mut().scrub_animation_clips(None, time)
        } else {
            let mut session = self.session.borrow_mut();
            if session.animation.preview_range.is_some() {
                session.pause();
            }
            session.seek(time)
        }
    }

    pub(super) fn rebuild_clip_tracks(self: &Rc<Self>, restore_ruler_focus: bool) {
        let session = self.session.borrow();
        let clips = session
            .animation_programs()
            .iter()
            .filter_map(|program| {
                let (clock, editable) = session.animation_clip_clock(program)?;
                Some(ClipRow {
                    id: program.id,
                    name: program.name.clone(),
                    span: clock.clip_span(),
                    editable,
                    length: clock.duration,
                })
            })
            .collect::<Vec<_>>();
        let scope = (
            session.document_uri().to_owned(),
            session.rust_map().map(|map| map.invocation.start),
        );
        let context = TimelineContext::new(&session);
        let playhead = if session.animation.play_all {
            session.time()
        } else {
            0.0
        };
        let range = self.preview_range_for_view(&session);
        drop(session);
        let duration = clips
            .iter()
            .map(|clip| clip.span.end.unwrap_or(clip.span.start + clip.length))
            .fold(range.map_or(0.05, |range| range.end.max(0.05)), f64::max);
        let mut ui = self.timeline_ui.borrow_mut();
        if ui.clip_scope.as_ref() != Some(&scope) {
            ui.clip_scope = Some(scope);
            ui.clip_selection.clear();
            ui.view = None;
        }
        ui.clip_duration = duration;
        ui.view = ui
            .view
            .filter(|(start, end)| *start >= 0.0 && *end > *start && *end <= duration);
        ui.clip_selection
            .retain(|id| clips.iter().any(|clip| clip.id == *id));
        let view = ui.view.unwrap_or((0.0, duration));
        let focus = ui.clip_focus.take();
        let selected = ui.clip_selection.clone();
        drop(ui);

        self.duration.set_text(Some(&format!("{duration:.3}")));
        let ruler = TimelineLane::ruler();
        ruler.configure(duration, vec![]);
        ruler.set_view(view.0, view.1);
        ruler.set_playhead(playhead);
        ruler.set_preview_range(range);
        AutomationProperties::set_name(&ruler, Some("Shared animation clip ruler"));
        self.bind_clip_lane(&ruler, &context);
        self.connect_preview_range(&ruler, &context);
        self.connect_preview_markers(&ruler, &context, true);
        let header = StackPanel::new();
        header.set_orientation(Orientation::Horizontal);
        header.set_spacing(4.0);
        for (path, title, factor) in [
            (crate::icons::REMOVE, "Zoom out on clips", 0.8),
            (crate::icons::FIT, "Fit clips", 0.0),
            (crate::icons::ADD, "Zoom in on clips", 1.25),
        ] {
            let tool = clip_tool(path, title);
            let weak = Rc::downgrade(self);
            tool.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let time = this.session.borrow().time();
                    if factor == 0.0 {
                        this.set_timeline_view(None);
                    } else {
                        this.zoom_timeline(factor, time);
                    }
                }
            });
            header.children().add(tool.upcast::<Control>());
        }
        let row = clip_row(header.upcast::<Control>(), &ruler);
        self.root
            .get_control::<ContentControl>("TimelineRulerHost")
            .set_content(Some(Control::boxed(&row)));
        if restore_ruler_focus {
            ruler.focus();
        }

        for clip in &clips {
            let order = selected.iter().position(|id| *id == clip.id);
            let label = button(&order.map_or_else(
                || clip.name.clone(),
                |index| format!("{}  {}", index + 1, clip.name),
            ));
            label.set_name(Some(format!("Clip{}", clip.id)));
            label.classes().add("nav");
            label.classes().set("active", order.is_some());
            label.set_min_height(26.0);
            let end = clip
                .span
                .end
                .map_or_else(|| "∞".into(), |end| format!("{end:.3} s"));
            let title = format!(
                "{} · {:.3} to {end}{}",
                clip.name,
                clip.span.start,
                if clip.editable {
                    ""
                } else {
                    " · Read-only timing"
                }
            );
            AutomationProperties::set_name(&label, Some(&title));
            ToolTip::set_tip(&label, Some(Rc::new(title.clone())));
            let weak = Rc::downgrade(self);
            let id = clip.id;
            label.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let mut ui = this.timeline_ui.borrow_mut();
                    if let Some(index) = ui
                        .clip_selection
                        .iter()
                        .position(|&selected| selected == id)
                    {
                        ui.clip_selection.remove(index);
                    } else {
                        ui.clip_selection.push(id);
                    }
                    ui.clip_focus = Some(id);
                    drop(ui);
                    this.rebuild_tracks();
                }
            });
            let lane = TimelineLane::new();
            lane.set_clip(clip.span, duration, clip.editable);
            lane.set_view(view.0, view.1);
            lane.set_playhead(playhead);
            lane.set_highlighted(order.is_some());
            AutomationProperties::set_name(&lane, Some(&title));
            ToolTip::set_tip(&lane, Some(Rc::new(title)));
            self.bind_clip_lane(&lane, &context);
            if clip.editable {
                let weak = Rc::downgrade(self);
                let context = context.clone();
                lane.on_move(move |_, start| {
                    if let Some(this) = weak.upgrade() {
                        this.apply_clip_placement(&context, &[id], Some(start), 0.0);
                    }
                });
            }
            self.tracks
                .children()
                .add(clip_row(label.clone().upcast::<Control>(), &lane).upcast::<Control>());
            if focus == Some(id) {
                label.focus();
            }
        }
        let edit = selected
            .iter()
            .map(|id| clips.iter().find(|clip| clip.id == *id && clip.editable))
            .collect::<Option<Vec<_>>>();
        if let Some(edit) = edit.filter(|clips| !clips.is_empty()) {
            self.clip_placement_fields(&context, selected, edit[0].span.start);
        }
    }

    fn bind_clip_lane(self: &Rc<Self>, lane: &Ref<TimelineLane>, context: &TimelineContext) {
        let weak = Rc::downgrade(self);
        let context = context.clone();
        lane.on_activate(move |time| {
            if let Some(this) = weak.upgrade() {
                let result = context
                    .check(&this)
                    .and_then(|()| this.seek_workspace_timeline(time));
                this.refresh(false);
                this.report(result);
            }
        });
        self.connect_timeline_zoom(lane);
        self.timeline_lanes.borrow_mut().push(lane.downgrade());
    }
}

fn clip_row(label: Ref<Control>, lane: &TimelineLane) -> Ref<Grid> {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse(&format!("{TRACK_HEADER},*")).unwrap());
    row.children().add(label);
    Grid::set_column(lane, 1);
    row.children().add(lane.to_ref().upcast::<Control>());
    row
}

pub(super) fn clip_tool(path: &str, title: &str) -> Ref<Button> {
    let tool = button("");
    tool.set_content(Some(Control::boxed(crate::icons::vector_icon(path, 12.0))));
    tool.classes().add("icon");
    tool.classes().add("designer-timeline-tool");
    AutomationProperties::set_name(&tool, Some(title));
    ToolTip::set_tip(&tool, Some(Rc::new(title.to_owned())));
    tool
}
