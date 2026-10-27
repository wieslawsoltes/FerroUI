//! Compact object/property lanes and a single contextual keyframe inspector.
use super::*;
use crate::timeline_lane::TimelineLane;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{ColumnDefinitions, ToolTip};
use ferroui_designer_animation::{Keyframe, Track};

const TRACK_HEADER: f64 = 210.0;
#[derive(Clone, Debug)]
pub(super) struct SelectedKey {
    target: NodeId,
    property: String,
    time: f64,
}
#[derive(Default)]
pub(super) struct TimelineUi {
    document: String,
    program: Option<NodeId>,
    selected: Option<SelectedKey>,
    view: Option<(f64, f64)>,
}
#[derive(Clone)]
struct TimelineContext {
    document: String,
    revision: u64,
    program: Option<NodeId>,
    syntax: std::sync::Arc<ferroui_designer_core::Ast>,
}
impl TimelineContext {
    fn new(session: &DesignerSession) -> Self {
        Self {
            document: session.document_uri().to_owned(),
            revision: session.document().revision(),
            program: session.active_animation_program(),
            syntax: session.document().ast_snapshot(),
        }
    }
    fn check(&self, workspace: &DesignerWorkspace) -> Result<(), String> {
        let session = workspace.session.borrow();
        if session.document_uri() == self.document
            && session.document().revision() == self.revision
            && session.active_animation_program() == self.program
            && std::ptr::eq(session.document().ast(), self.syntax.as_ref())
        {
            Ok(())
        } else {
            Err("The timeline changed while editing; retry with its current keyframes".into())
        }
    }
}
impl DesignerWorkspace {
    pub(super) fn rebuild_tracks(self: &Rc<Self>) {
        let focus_was_ruler = self
            .timeline_lanes
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
            .any(|lane| lane.is_focused() && lane.is_ruler());
        let focus_was_track = self
            .timeline_lanes
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
            .any(|lane| lane.is_focused() && !lane.is_ruler());
        self.tracks.children().clear();
        self.timeline_lanes.borrow_mut().clear();
        let session = self.session.borrow();
        let tracks = session.timeline().tracks.clone();
        let warnings = session.animation_warnings().to_vec();
        let duration = session.timeline().duration;
        let context = TimelineContext::new(&session);
        let playhead = session.time();
        let selection = session.selection().to_vec();
        let mut ui = self.timeline_ui.borrow_mut();
        if ui.document != session.document_uri() || ui.program != session.active_animation_program()
        {
            ui.document = session.document_uri().to_owned();
            ui.program = session.active_animation_program();
            ui.selected = None;
            ui.view = None;
        }
        if !ui.selected.as_ref().is_some_and(|selected| {
            tracks.iter().any(|track| {
                track.target == selected.target
                    && track.property == selected.property
                    && track.keyframes.iter().any(|key| key.time == selected.time)
            })
        }) {
            let preferred = tracks
                .iter()
                .find(|track| selection.contains(&track.target))
                .or(tracks.first());
            ui.selected = preferred.and_then(|track| nearest_key(track, playhead));
        }
        ui.view = ui
            .view
            .filter(|(start, end)| *start >= 0.0 && *end > *start && *end <= duration);
        let view = ui.view.unwrap_or((0.0, duration));
        let selected = ui.selected.clone();
        drop(ui);
        drop(session);

        let ruler_row = Grid::new();
        ruler_row.set_column_definitions(
            ColumnDefinitions::parse(&format!("{TRACK_HEADER},*")).unwrap(),
        );
        let header = StackPanel::new();
        header.set_orientation(Orientation::Horizontal);
        header.set_spacing(4.0);
        let caption = TextBlock::new();
        caption.set_text(Some("OBJECT / PROPERTY"));
        caption.set_font_size(10.0);
        caption.set_opacity(0.65);
        caption.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
        caption.set_margin(Thickness::new(6.0, 0.0, 6.0, 0.0));
        header.children().add(caption.upcast::<Control>());
        for (label, help, factor) in [
            ("−", "Zoom out timeline", 0.8),
            ("+", "Zoom in timeline", 1.25),
            ("Fit", "Fit complete animation", 0.0),
        ] {
            let control = button(label);
            control.set_padding(Thickness::symmetric(5.0, 2.0));
            control.set_min_width(22.0);
            control.set_font_size(10.0);
            AutomationProperties::set_name(&control, Some(help));
            ToolTip::set_tip(&control, Some(Rc::new(help.to_owned())));
            let weak = Rc::downgrade(self);
            control.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    if factor == 0.0 {
                        this.set_timeline_view(None);
                    } else {
                        let pivot = this.session.borrow().time();
                        this.zoom_timeline(factor, pivot);
                    }
                }
            });
            header.children().add(control.upcast::<Control>());
        }
        ruler_row.children().add(header.upcast::<Control>());
        let ruler = TimelineLane::ruler();
        ruler.configure(duration, Vec::new());
        ruler.set_view(view.0, view.1);
        ruler.set_playhead(playhead);
        let weak = Rc::downgrade(self);
        ruler.on_activate(move |time| {
            if let Some(this) = weak.upgrade() {
                let result = this.session.borrow_mut().seek(time);
                this.refresh(false);
                this.report(result);
            }
        });
        self.connect_timeline_zoom(&ruler);
        self.timeline_lanes.borrow_mut().push(ruler.downgrade());
        Grid::set_column(&ruler, 1);
        ruler_row.children().add(ruler.clone().upcast::<Control>());
        self.tracks.children().add(ruler_row.upcast::<Control>());
        if focus_was_ruler {
            ruler.focus();
        }

        for warning in warnings {
            let label = TextBlock::new();
            label.set_text(Some(&warning));
            label.set_font_size(10.0);
            label.set_foreground(Some(SolidColorBrush::from_uint32(0xfff5c77d).into()));
            label.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
            self.tracks.children().add(label.upcast::<Control>());
        }
        for track in &tracks {
            let target_name = self
                .session
                .borrow()
                .document()
                .ast()
                .node(track.target)
                .map(|node| {
                    node.xaml_name()
                        .unwrap_or_else(|| node.local_name())
                        .to_owned()
                })
                .unwrap_or_else(|| "Missing target".into());
            let is_selected = selected.as_ref().is_some_and(|selected| {
                selected.target == track.target && selected.property == track.property
            });
            let plot = Grid::new();
            plot.set_column_definitions(
                ColumnDefinitions::parse(&format!("{TRACK_HEADER},*")).unwrap(),
            );
            let title = button(&format!("{}  ·  {}", target_name, track.property));
            title.set_font_size(11.0);
            title.set_height(34.0);
            title.set_horizontal_alignment(HorizontalAlignment::Stretch);
            title.set_horizontal_content_alignment(HorizontalAlignment::Left);
            title.set_padding(Thickness::symmetric(8.0, 3.0));
            title.set_background(Some(
                SolidColorBrush::from_uint32(if is_selected { 0xff263943 } else { 0xff262626 })
                    .into(),
            ));
            ToolTip::set_tip(
                &title,
                Some(Rc::new(format!(
                    "{} / {}\n{} keyframes",
                    target_name,
                    track.property,
                    track.keyframes.len()
                ))),
            );
            let weak = Rc::downgrade(self);
            let target = track.target;
            let property = track.property.clone();
            title.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let time = this.session.borrow().time();
                    this.activate_timeline_key(target, &property, time);
                }
            });
            plot.children().add(title.upcast::<Control>());
            let lane = TimelineLane::new();
            lane.configure(
                duration,
                track.keyframes.iter().map(|key| key.time).collect(),
            );
            lane.set_view(view.0, view.1);
            lane.set_playhead(playhead);
            lane.set_highlighted(is_selected);
            if is_selected {
                lane.set_selected(selected.as_ref().map(|selected| selected.time));
            }
            AutomationProperties::set_name(
                &lane,
                Some(&format!("{} {} keyframes", target_name, track.property)),
            );
            let weak = Rc::downgrade(self);
            let property = track.property.clone();
            lane.on_activate(move |time| {
                if let Some(this) = weak.upgrade() {
                    this.activate_timeline_key(target, &property, time);
                }
            });
            let weak = Rc::downgrade(self);
            let property = track.property.clone();
            let move_context = context.clone();
            lane.on_move(move |from, to| {
                if let Some(this) = weak.upgrade() {
                    let result = move_context.check(&this).and_then(|_| {
                        this.session
                            .borrow_mut()
                            .move_keyframe(target, &property, from, to)
                    });
                    if result.is_ok() {
                        this.timeline_ui.borrow_mut().selected = Some(SelectedKey {
                            target,
                            property: property.clone(),
                            time: to,
                        });
                    }
                    this.refresh(true);
                    this.report(result);
                }
            });
            let weak = Rc::downgrade(self);
            let property = track.property.clone();
            let remove_context = context.clone();
            lane.on_remove(move |time| {
                if let Some(this) = weak.upgrade() {
                    let result = remove_context.check(&this).and_then(|_| {
                        this.session
                            .borrow_mut()
                            .remove_keyframe(target, &property, time)
                    });
                    this.refresh(true);
                    this.report(result);
                }
            });
            let weak = Rc::downgrade(self);
            let property = track.property.clone();
            let insert_context = context.clone();
            lane.on_insert(move |time| {
                if let Some(this) = weak.upgrade() {
                    let result = insert_context
                        .check(&this)
                        .and_then(|_| this.record_timeline_property(target, &property, time));
                    if result.is_ok() {
                        this.timeline_ui.borrow_mut().selected = Some(SelectedKey {
                            target,
                            property: property.clone(),
                            time,
                        });
                    }
                    this.refresh(true);
                    this.report(result);
                }
            });
            self.connect_timeline_zoom(&lane);
            self.timeline_lanes.borrow_mut().push(lane.downgrade());
            Grid::set_column(&lane, 1);
            plot.children().add(lane.clone().upcast::<Control>());
            self.tracks.children().add(plot.upcast::<Control>());
            if is_selected && focus_was_track {
                lane.focus();
            }
        }
        if let Some(selected) = selected {
            if let Some(key) = tracks
                .iter()
                .find(|track| {
                    track.target == selected.target && track.property == selected.property
                })
                .and_then(|track| track.keyframes.iter().find(|key| key.time == selected.time))
            {
                self.build_keyframe_inspector(&selected, key, context);
            }
        }
        if tracks.is_empty() {
            let hint = TextBlock::new();
            hint.set_text(Some("Build motion with keyframes\nSelect an object, enable Record, then edit a property at different times."));
            hint.set_opacity(0.65);
            hint.set_font_size(12.0);
            hint.set_margin(Thickness::symmetric(12.0, 18.0));
            hint.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
            self.tracks.children().add(hint.upcast::<Control>());
        }
    }
    fn connect_timeline_zoom(self: &Rc<Self>, lane: &TimelineLane) {
        let weak = Rc::downgrade(self);
        lane.on_zoom(move |factor, pivot| {
            if let Some(this) = weak.upgrade() {
                this.zoom_timeline(factor, pivot);
            }
        });
        let weak = Rc::downgrade(self);
        lane.on_pan(move |seconds| {
            if let Some(this) = weak.upgrade() {
                if !seconds.is_finite() {
                    return;
                }
                let duration = this.session.borrow().timeline().duration;
                let (start, end) = this
                    .timeline_ui
                    .borrow()
                    .view
                    .filter(|(start, end)| *start >= 0.0 && *end > *start && *end <= duration)
                    .unwrap_or((0.0, duration));
                let span = end - start;
                let start = (start + seconds).clamp(0.0, duration - span);
                this.set_timeline_view(Some((start, start + span)));
            }
        });
    }
    fn zoom_timeline(self: &Rc<Self>, factor: f64, pivot: f64) {
        if !factor.is_finite() || factor <= 0.0 || !pivot.is_finite() {
            return;
        }
        let duration = self.session.borrow().timeline().duration;
        let (start, end) = self
            .timeline_ui
            .borrow()
            .view
            .filter(|(start, end)| *start >= 0.0 && *end > *start && *end <= duration)
            .unwrap_or((0.0, duration));
        let span = ((end - start) / factor).clamp(duration.min(0.05), duration);
        let anchor = ((pivot - start) / (end - start)).clamp(0.0, 1.0);
        let start = (pivot - span * anchor).clamp(0.0, duration - span);
        self.set_timeline_view(Some((start, start + span)));
    }
    fn set_timeline_view(&self, view: Option<(f64, f64)>) {
        let duration = self.session.borrow().timeline().duration;
        let view = view.filter(|(start, end)| *start >= 0.0 && *end > *start && *end <= duration);
        self.timeline_ui.borrow_mut().view = view;
        let (start, end) = view.unwrap_or((0.0, duration));
        // Viewport changes preserve native focus and any unfinished keyframe
        // input, and avoid rebuilding the catalog of keys on every wheel event.
        for lane in self
            .timeline_lanes
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
        {
            lane.set_view(start, end);
        }
    }
    fn activate_timeline_key(self: &Rc<Self>, target: NodeId, property: &str, time: f64) {
        let selection = self
            .session
            .borrow()
            .timeline()
            .track(target, property)
            .and_then(|track| nearest_key(track, time));
        self.timeline_ui.borrow_mut().selected = selection;
        self.session.borrow_mut().select([target]);
        let result = self.session.borrow_mut().seek(time);
        self.refresh(true);
        self.report(result);
    }
    /// Selects an authored keyframe, seeks its time, and displays its contextual editor.
    pub fn select_keyframe(
        self: &Rc<Self>,
        target: NodeId,
        property: &str,
        time: f64,
    ) -> Result<(), String> {
        let exists = self
            .session
            .borrow()
            .timeline()
            .track(target, property)
            .is_some_and(|track| track.keyframes.iter().any(|key| key.time == time));
        if !exists {
            return Err("The requested keyframe does not exist".into());
        }
        self.session.borrow_mut().seek(time)?;
        self.session.borrow_mut().select([target]);
        self.remember_keyframe(SelectedKey {
            target,
            property: property.to_owned(),
            time,
        });
        self.refresh(true);
        Ok(())
    }
    pub(super) fn record_selected_keyframe(&self) -> Result<(), String> {
        let session = self.session.borrow();
        let target = session
            .selection()
            .first()
            .copied()
            .ok_or("Select an object to add a keyframe")?;
        let time = session.time();
        let ui = self.timeline_ui.borrow();
        let property = ui
            .selected
            .as_ref()
            .filter(|selected| {
                selected.target == target
                    && ui.document == session.document_uri()
                    && ui.program == session.active_animation_program()
                    && session
                        .timeline()
                        .track(target, &selected.property)
                        .is_some()
            })
            .map(|selected| selected.property.clone())
            .unwrap_or_else(|| "Opacity".to_owned());
        drop(ui);
        drop(session);
        self.record_timeline_property(target, &property, time)?;
        self.remember_keyframe(SelectedKey {
            target,
            property,
            time,
        });
        Ok(())
    }
    fn remember_keyframe(&self, selected: SelectedKey) {
        let session = self.session.borrow();
        let mut ui = self.timeline_ui.borrow_mut();
        if ui.document != session.document_uri() || ui.program != session.active_animation_program()
        {
            ui.view = None;
        }
        ui.document = session.document_uri().to_owned();
        ui.program = session.active_animation_program();
        ui.selected = Some(selected);
    }
    fn record_timeline_property(
        &self,
        target: NodeId,
        property: &str,
        time: f64,
    ) -> Result<(), String> {
        let mut session = self.session.borrow_mut();
        session.seek(time)?;
        let visual = session
            .preview
            .visual(target)
            .ok_or("The animation target has no accepted preview")?;
        let info = Inspector::property(&visual, property).map_err(|error| error.to_string())?;
        let value = ferroui_inspect::display_value(Some(&visual.get_value_untyped(info)));
        let value = session.animation_value_for_target(target, property, &value)?;
        session.record_keyframe(target, property, value)
    }
    fn build_keyframe_inspector(
        self: &Rc<Self>,
        selected: &SelectedKey,
        key: &Keyframe,
        context: TimelineContext,
    ) {
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(7.0);
        row.set_margin(Thickness::new(6.0, 8.0, 0.0, 2.0));
        let caption = TextBlock::new();
        caption.set_text(Some(&format!("◆ {}", selected.property)));
        caption.set_font_size(11.0);
        caption.set_width(TRACK_HEADER - 13.0);
        caption.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
        row.children().add(caption.upcast::<Control>());
        let time = key_field(&row, "Time", "KeyframeTime", &key.time.to_string(), 65.0);
        let value = key_field(&row, "Value", "KeyframeValue", &key.value.to_xaml(), 140.0);
        let curve = key_field(&row, "Curve", "KeyframeCurve", &key.easing.to_xaml(), 160.0);
        ToolTip::set_tip(
            &curve,
            Some(Rc::new(
                "LinearEasing or cubic Bézier x1,y1,x2,y2".to_owned(),
            )),
        );
        let apply = button("Apply keyframe");
        apply.set_name(Some("ApplyKeyframe".into()));
        apply.set_font_size(11.0);
        let weak = Rc::downgrade(self);
        let selected = selected.clone();
        let kind = key.value.kind();
        let property = selected.property.clone();
        let target = selected.target;
        let old_time = selected.time;
        let weak_time = time.downgrade();
        let weak_value = value.downgrade();
        let weak_curve = curve.downgrade();
        let commit_context = context.clone();
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            if let (Some(this), Some(time), Some(value), Some(curve)) = (
                weak.upgrade(),
                weak_time.upgrade(),
                weak_value.upgrade(),
                weak_curve.upgrade(),
            ) {
                let result = (|| {
                    commit_context.check(&this)?;
                    let time: f64 = time
                        .text()
                        .unwrap_or_default()
                        .parse()
                        .map_err(|_| "Invalid keyframe time")?;
                    let value = AnimationValue::parse(&value.text().unwrap_or_default(), kind)
                        .map_err(|error| error.to_string())?;
                    let easing = Easing::parse(&curve.text().unwrap_or_default())
                        .map_err(|error| error.to_string())?;
                    let mut session = this.session.borrow_mut();
                    let id = session
                        .active_animation_program()
                        .ok_or("Choose an animation")?;
                    let mut timeline = session.timeline().clone();
                    if !time.is_finite() || !(0.0..=timeline.duration).contains(&time) {
                        return Err("Keyframe is outside the timeline".into());
                    }
                    let track = timeline
                        .track_mut(target, &property)
                        .ok_or("Animation track is no longer present")?;
                    if time != old_time {
                        track
                            .move_keyframe(old_time, time)
                            .map_err(|error| error.to_string())?;
                    }
                    track
                        .record(time, value)
                        .map_err(|error| error.to_string())?;
                    track
                        .set_easing(time, easing)
                        .map_err(|error| error.to_string())?;
                    session.update_animation_program(id, timeline)?;
                    this.timeline_ui.borrow_mut().selected = Some(SelectedKey {
                        target,
                        property: property.clone(),
                        time,
                    });
                    Ok(())
                })();
                // Keep invalid input available for correction. Accepted changes
                // rebuild the editor from the single committed source transaction.
                if result.is_ok() {
                    this.refresh(true);
                }
                this.report(result);
            }
        });
        let click = commit.clone();
        apply.click(move |_, _| click());
        for input in [&time, &value, &curve] {
            let commit = commit.clone();
            input.add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Enter {
                    event.set_handled(true);
                    commit();
                }
            });
        }
        row.children().add(apply.upcast::<Control>());
        let remove = button("Remove");
        remove.set_font_size(11.0);
        AutomationProperties::set_name(&remove, Some("Remove selected keyframe"));
        let weak = Rc::downgrade(self);
        remove.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                let result = context.check(&this).and_then(|_| {
                    this.session.borrow_mut().remove_keyframe(
                        selected.target,
                        &selected.property,
                        selected.time,
                    )
                });
                this.refresh(true);
                this.report(result);
            }
        });
        row.children().add(remove.upcast::<Control>());
        self.tracks.children().add(row.upcast::<Control>());
    }
}
fn nearest_key(track: &Track, time: f64) -> Option<SelectedKey> {
    track
        .keyframes
        .iter()
        .min_by(|a, b| (a.time - time).abs().total_cmp(&(b.time - time).abs()))
        .map(|key| SelectedKey {
            target: track.target,
            property: track.property.clone(),
            time: key.time,
        })
}
fn key_field(
    panel: &StackPanel,
    caption: &str,
    name: &str,
    value: &str,
    width: f64,
) -> Ref<TextBox> {
    let label = TextBlock::new();
    label.set_text(Some(caption));
    label.set_font_size(10.0);
    label.set_opacity(0.65);
    label.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    panel.children().add(label.upcast::<Control>());
    let input = TextBox::new();
    input.set_name(Some(name.into()));
    input.set_text(Some(value));
    input.set_width(width);
    input.set_font_size(11.0);
    AutomationProperties::set_name(&input, Some(&format!("Keyframe {caption}")));
    panel.children().add(input.clone().upcast::<Control>());
    input
}
