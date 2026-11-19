//! The clip comparison view shares Studio's ruler, clock and source transaction.
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
            self.session.borrow().timeline().duration
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
            self.session.borrow_mut().seek(time)
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
        drop(session);
        let duration = clips
            .iter()
            .map(|clip| clip.span.end.unwrap_or(clip.span.start + clip.length))
            .fold(0.05, f64::max);
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
        AutomationProperties::set_name(&ruler, Some("Shared animation clip ruler"));
        self.bind_clip_lane(&ruler, &context);
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

    fn apply_clip_placement(
        self: &Rc<Self>,
        context: &TimelineContext,
        clips: &[NodeId],
        start: Option<f64>,
        gap: f64,
    ) {
        let result = context.check(self).and_then(|()| {
            let mut session = self.session.borrow_mut();
            if session.is_playing() {
                return Err("Pause the clips before placing them".into());
            }
            session.place_animation_clips(clips, start, gap)
        });
        if result.is_ok() {
            self.refresh(true);
        }
        self.report(result);
    }

    fn clip_placement_fields(
        self: &Rc<Self>,
        context: &TimelineContext,
        ids: Vec<NodeId>,
        initial: f64,
    ) {
        let fields = StackPanel::new();
        fields.set_orientation(Orientation::Horizontal);
        fields.set_spacing(5.0);
        let start = clip_field(&fields, "Start (s)", "ClipStart", initial);
        let gap = (ids.len() > 1).then(|| clip_field(&fields, "Gap / overlap (s)", "ClipGap", 0.0));
        let title = if ids.len() == 1 {
            "Set selected clip start"
        } else {
            "Sequence selected clips"
        };
        let apply = clip_tool("F1 M1 8L3 6L6 9L13 2L15 4L6 13Z", title);
        apply.set_name(Some("ApplyClipPlacement".into()));
        let weak = Rc::downgrade(self);
        let context = context.clone();
        apply.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                let parse =
                    |field: &TextBox| field.text().unwrap_or_default().trim().parse::<f64>();
                match (
                    parse(&start),
                    gap.as_ref().map(|field| parse(field)).transpose(),
                ) {
                    (Ok(start), Ok(gap)) => this.apply_clip_placement(
                        &context,
                        &ids,
                        (start != initial).then_some(start),
                        gap.unwrap_or(0.0),
                    ),
                    _ => this.report(Err("Enter clip timing in seconds".into())),
                }
            }
        });
        fields.children().add(apply.upcast::<Control>());
        self.root
            .get_control::<ContentControl>("TimelineInspectorHost")
            .set_content(Some(Control::boxed(&fields)));
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

fn clip_tool(path: &str, title: &str) -> Ref<Button> {
    let tool = button("");
    tool.set_content(Some(Control::boxed(crate::icons::vector_icon(path, 12.0))));
    tool.classes().add("icon");
    tool.classes().add("designer-timeline-tool");
    AutomationProperties::set_name(&tool, Some(title));
    ToolTip::set_tip(&tool, Some(Rc::new(title.to_owned())));
    tool
}

fn clip_field(fields: &StackPanel, label: &str, name: &str, initial: f64) -> Ref<TextBox> {
    let caption = TextBlock::new();
    caption.set_text(Some(label));
    caption.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    fields.children().add(caption.upcast::<Control>());
    let input = TextBox::new();
    input.set_name(Some(name.into()));
    input.set_text(Some(&initial.to_string()));
    input.set_width(80.0);
    input.classes().add("designer-field");
    AutomationProperties::set_name(&input, Some(label));
    fields.children().add(input.clone().upcast::<Control>());
    input
}
