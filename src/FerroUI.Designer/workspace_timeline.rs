//! Compact object/property lanes and a single contextual keyframe inspector.
#[path = "workspace_timeline/clipboard.rs"]
mod clipboard;
#[path = "workspace_timeline/clips.rs"]
mod clips;
#[path = "workspace_timeline/curves.rs"]
mod curves;
#[path = "workspace_timeline/easing.rs"]
mod easing;

use super::*;
use crate::timeline_lane::{EasingPreview, TimelineLane};
use ferroui_base::media::{FontWeight, TextTrimming};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{ColumnDefinitions, ComboBox, Flyout, PathIcon, ToolTip};
use ferroui_designer_animation::{
    keyframe_easing_presets, CurveChannel, CurveMetric, Keyframe, KeyframeGroupEdit as KeyframeEdit,
    KeyframeLocation as SelectedKey, Track,
};

const TRACK_HEADER: f64 = 240.0;
#[derive(Default)]
pub(super) struct TimelineUi {
    document: String,
    program: Option<NodeId>,
    selected: Option<SelectedKey>,
    selection: Vec<SelectedKey>,
    selection_explicit: bool,
    frame_rate: Option<u32>,
    snap_picker: Option<WeakRef<ComboBox>>,
    view: Option<(f64, f64)>,
    curves: Option<CurveMetric>,
    curve_channels: std::collections::BTreeMap<(NodeId, String), CurveChannel>,
    keyframe_paste: Option<Ref<Flyout>>,
    compare_clips: bool,
    clip_scope: Option<(String, Option<usize>)>,
    clip_selection: Vec<NodeId>,
    clip_focus: Option<NodeId>,
    clip_duration: f64,
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
    fn timeline_inspector(&self) -> Option<Ref<StackPanel>> {
        self.root
            .get_control::<ContentControl>("TimelineInspectorHost")
            .content()
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|control| control.cast::<StackPanel>())
    }
    pub(super) fn rebuild_tracks(self: &Rc<Self>) {
        self.close_timeline_paste();
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
        let inspector_focus = self
            .timeline_inspector()
            .into_iter()
            .flat_map(|row| row.children().to_vec())
            .find(|control| control.is_focused())
            .and_then(|control| {
                let name = control.name()?;
                let selection = control.cast::<TextBox>().map(|input| (input.selection_start(), input.selection_end()));
                Some((name, selection))
            });
        self.tracks.children().clear();
        self.root
            .get_control::<ContentControl>("TimelineInspectorHost")
            .set_content(None);
        self.timeline_lanes.borrow_mut().clear();
        if self.comparing_animation_clips() {
            self.rebuild_clip_tracks(focus_was_ruler);
            return;
        }
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
            ui.selection.clear();
            ui.selection_explicit = false;
            ui.view = None;
        }
        ui.selection.retain(|selected| {
            tracks.iter().any(|track| {
                track.target == selected.target
                    && track.property == selected.property
                    && track
                        .keyframes
                        .binary_search_by(|key| key.time.total_cmp(&selected.time))
                        .is_ok()
            })
        });
        if !ui.selected.as_ref().is_some_and(|selected| {
            tracks.iter().any(|track| {
                track.target == selected.target
                    && track.property == selected.property
                    && track
                        .keyframes
                        .binary_search_by(|key| key.time.total_cmp(&selected.time))
                        .is_ok()
            })
        }) {
            let preferred = tracks
                .iter()
                .find(|track| selection.contains(&track.target))
                .or(tracks.first());
            ui.selected = ui.selection.last().cloned().or_else(|| {
                (!ui.selection_explicit)
                    .then(|| preferred.and_then(|track| nearest_key(track, playhead)))
                    .flatten()
            });
        }
        if ui.selection.is_empty() {
            if let Some(selected) = ui.selected.clone() {
                ui.selection.push(selected);
            }
        }
        let selection_keys = ui.selection.clone();
        let frame_rate = ui.frame_rate;
        let curves = ui.curves;
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
        header.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
        let caption = TextBlock::new();
        caption.set_text(Some("Tracks"));
        caption.set_font_size(10.0);
        caption.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
        caption.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
        caption.set_margin(Thickness::new(4.0, 0.0, 4.0, 0.0));
        header.children().add(caption.upcast::<Control>());
        for (path, help, factor) in [
            (crate::icons::REMOVE, "Zoom out timeline", 0.8),
            (crate::icons::ADD, "Zoom in timeline", 1.25),
            (
                crate::icons::FIT,
                "Fit selected keys or complete animation",
                0.0,
            ),
        ] {
            let control = button("");
            control.set_content(Some(Control::boxed(crate::icons::vector_icon(path, 12.0))));
            control.classes().add("designer-timeline-tool");
            control.classes().add("nav");
            control.set_padding(Thickness::symmetric(5.0, 2.0));
            control.set_min_width(24.0);
            control.set_font_size(10.0);
            AutomationProperties::set_name(&control, Some(help));
            ToolTip::set_tip(&control, Some(Rc::new(help.to_owned())));
            let weak = Rc::downgrade(self);
            control.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    if factor == 0.0 {
                        this.fit_timeline_selection();
                    } else {
                        let pivot = this.session.borrow().time();
                        this.zoom_timeline(factor, pivot);
                    }
                }
            });
            header.children().add(control.upcast::<Control>());
        }
        let snap = ComboBox::new();
        snap.set_name(Some("TimelineSnap".into()));
        snap.set_width(86.0);
        snap.classes().add("designer-field");
        snap.set_min_height(26.0);
        snap.set_height(26.0);
        snap.set_font_size(10.0);
        snap.set_padding(Thickness::symmetric(5.0, 2.0));
        let mut rates = vec![None, Some(24), Some(30), Some(60)];
        if !rates.contains(&frame_rate) {
            rates.push(frame_rate);
        }
        for rate in &rates {
            snap.items().add(Some(Rc::new(
                rate.map_or_else(|| "Sec".to_owned(), |rate| format!("{rate} fps")),
            )));
        }
        snap.set_selected_index(
            rates
                .iter()
                .position(|rate| *rate == frame_rate)
                .unwrap_or(0) as i32,
        );
        self.timeline_ui.borrow_mut().snap_picker = Some(snap.downgrade());
        AutomationProperties::set_name(&snap, Some("Timeline snap unit"));
        ToolTip::set_tip(
            &snap,
            Some(Rc::new(
                "Snap to seconds or exact frames. Alt temporarily disables pointer snapping."
                    .to_owned(),
            )),
        );
        let weak = Rc::downgrade(self);
        let weak_snap = snap.downgrade();
        snap.selection_changed(move |_, _| {
            if let (Some(this), Some(snap)) = (weak.upgrade(), weak_snap.upgrade()) {
                if let Some(label) = snap
                    .selected_item()
                    .and_then(|item| item.downcast_ref::<String>().cloned())
                {
                    if label == "Sec" {
                        this.set_timeline_frame_rate(None);
                    } else if let Some(rate) = label
                        .strip_suffix(" fps")
                        .and_then(|rate| rate.parse().ok())
                    {
                        this.set_timeline_frame_rate(Some(rate));
                    }
                }
            }
        });
        header.children().add(snap.upcast::<Control>());
        if context.program.is_some() && !tracks.is_empty() {
            header.children().add(self.timeline_keyframe_menu(&context, !selection_keys.is_empty()).upcast::<Control>());
        }
        ruler_row.children().add(header.upcast::<Control>());
        let ruler = TimelineLane::ruler();
        ruler.configure(
            duration,
            tracks
                .iter()
                .flat_map(|track| track.keyframes.iter().map(|key| key.time))
                .collect(),
        );
        ruler.set_frame_rate(frame_rate);
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
        self.connect_timeline_clipboard(&ruler, &context, None);
        self.timeline_lanes.borrow_mut().push(ruler.downgrade());
        Grid::set_column(&ruler, 1);
        ruler_row.children().add(ruler.clone().upcast::<Control>());
        self.root
            .get_control::<ContentControl>("TimelineRulerHost")
            .set_content(Some(Control::boxed(&ruler_row)));
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
            let is_selected = selection_keys.iter().any(|selected| {
                selected.target == track.target && selected.property == track.property
            });
            let plot = Grid::new();
            plot.set_column_definitions(
                ColumnDefinitions::parse(&format!("{TRACK_HEADER},*")).unwrap(),
            );
            let title = track_header(
                &target_name,
                &track.property,
                track.keyframes.len(),
                is_selected,
            );
            title.set_font_size(11.0);
            title.set_height(36.0);
            title.set_horizontal_alignment(HorizontalAlignment::Stretch);
            title.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            title.set_padding(Thickness::symmetric(10.0, 2.0));
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
            let lane = TimelineLane::new();
            lane.configure(
                duration,
                track.keyframes.iter().map(|key| key.time).collect(),
            );
            lane.set_view(view.0, view.1);
            lane.set_playhead(playhead);
            lane.set_highlighted(is_selected);
            lane.set_frame_rate(frame_rate);
            self.connect_timeline_clipboard(&lane, &context, Some(track));
            lane.set_selected_keys(
                selection_keys
                    .iter()
                    .filter(|key| key.target == target && key.property == track.property)
                    .map(|key| key.time)
                    .collect(),
                selected
                    .as_ref()
                    .filter(|key| key.target == target && key.property == track.property)
                    .map(|key| key.time),
            );
            let curve = curves.and_then(|metric| self.timeline_curve_channel(track).map(|channel| (metric, channel)));
            if let Some((metric, channel)) = curve {
                self.attach_timeline_curve(&lane, track, &context, metric, channel);
                title.set_height(lane.height());
            }
            plot.children().add(match curve {
                Some((metric, channel)) if channel != CurveChannel::Value => self.timeline_curve_title(title, &lane, track, &context, metric, channel),
                _ => title.upcast::<Control>(),
            });
            AutomationProperties::set_name(
                &lane,
                Some(&format!(
                    "{} {} {}",
                    target_name,
                    track.property,
                    match curve {
                        Some((CurveMetric::Value, _)) => "value curve",
                        Some((CurveMetric::Velocity, _)) => "velocity curve",
                        None => "keyframes",
                    }
                )),
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
            let selection_context = context.clone();
            lane.on_selection_changed(move |times, primary, additive| {
                if let Some(this) = weak.upgrade() {
                    let result = selection_context.check(&this).and_then(|_| {
                        this.accept_lane_selection(target, &property, &times, primary, additive);
                        if let Some(time) = primary {
                            this.session.borrow_mut().seek(time)?;
                        }
                        Ok(())
                    });
                    this.refresh(true);
                    this.report(result);
                }
            });
            let weak = Rc::downgrade(self);
            let property = track.property.clone();
            let move_context = context.clone();
            lane.on_move_selection(move |times, delta| {
                if let Some(this) = weak.upgrade() {
                    let result = move_context.check(&this).and_then(|_| {
                        this.prepare_lane_batch(target, &property, &times);
                        this.edit_selected_keys(KeyframeEdit::Move(delta))
                    });
                    this.refresh(true);
                    this.report(result);
                }
            });
            let weak = Rc::downgrade(self);
            let property = track.property.clone();
            let remove_context = context.clone();
            lane.on_remove_selection(move |times| {
                if let Some(this) = weak.upgrade() {
                    let result = remove_context.check(&this).and_then(|_| {
                        this.prepare_lane_batch(target, &property, &times);
                        this.edit_selected_keys(KeyframeEdit::Remove)
                    });
                    this.refresh(true);
                    this.report(result);
                }
            });
            let weak = Rc::downgrade(self);
            let property = track.property.clone();
            let duplicate_context = context.clone();
            lane.on_duplicate_selection(move |times, delta| {
                if let Some(this) = weak.upgrade() {
                    let result = duplicate_context.check(&this).and_then(|_| {
                        this.prepare_lane_batch(target, &property, &times);
                        this.edit_selected_keys(KeyframeEdit::Duplicate(delta))
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
                        this.remember_keyframe(SelectedKey {
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
        if selection_keys.len() > 1 {
            self.build_selection_inspector(context);
        } else if let Some(selected) = selected {
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
            self.build_timeline_empty_state();
        }
        if let Some((name, selection)) = inspector_focus {
            if let Some(control) = self
                .timeline_inspector()
                .into_iter()
                .flat_map(|row| row.children().to_vec())
                .find(|control| control.name().as_deref() == Some(name.as_str()))
            {
                control.focus();
                if let (Some((start, end)), Some(input)) = (selection, control.cast::<TextBox>()) {
                    input.set_selection_start(start);
                    input.set_selection_end(end);
                }
            }
        }
    }
    fn build_timeline_empty_state(self: &Rc<Self>) {
        if self.session.borrow().uses_rust_source() {
            let label = key_caption("No mapped keyframes");
            label.set_margin(Thickness::new(8.0, 12.0, 8.0, 8.0));
            self.tracks.children().add(label.upcast::<Control>());
            return;
        }
        let row = Grid::new();
        row.set_column_definitions(ColumnDefinitions::parse("48,*,Auto").unwrap());
        row.set_margin(Thickness::new(14.0, 18.0, 18.0, 14.0));
        let icon = timeline_icon(
            "F1 M2,8 L8,2 14,8 8,14 Z M18,8 L24,2 30,8 24,14 Z M13,7 L19,7 19,9 13,9 Z",
            0xffb6a2f6,
        );
        icon.set_width(30.0);
        icon.set_height(20.0);
        icon.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
        row.children().add(icon.upcast::<Control>());
        let copy = StackPanel::new();
        copy.set_spacing(4.0);
        let title = TextBlock::new();
        title.set_text(Some("Bring your interface to life"));
        title.set_font_size(12.0);
        title.set_font_weight(FontWeight::SemiBold);
        copy.children().add(title.upcast::<Control>());
        let hint = TextBlock::new();
        hint.set_text(Some(
            "Select an object and add a keyframe, or turn on Record to capture property changes.",
        ));
        hint.set_font_size(11.0);
        hint.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
        hint.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        copy.children().add(hint.upcast::<Control>());
        Grid::set_column(&copy, 1);
        row.children().add(copy.upcast::<Control>());
        let create = button("Add first keyframe");
        create.classes().add("designer-motion");
        create.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
        create.set_margin(Thickness::new(18.0, 0.0, 0.0, 0.0));
        create.set_is_enabled(!self.session.borrow().selection().is_empty());
        ToolTip::set_tip(&create, Some(Rc::new("Record the selected object's current value at the playhead. The first track uses Opacity.".to_owned())));
        let weak = Rc::downgrade(self);
        create.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                let result = this.record_selected_keyframe();
                this.refresh(true);
                this.report(result);
            }
        });
        Grid::set_column(&create, 2);
        row.children().add(create.upcast::<Control>());
        self.tracks.children().add(row.upcast::<Control>());
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
                let duration = this.timeline_view_duration();
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
        let duration = self.timeline_view_duration();
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
        let duration = self.timeline_view_duration();
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
        {
            let mut ui = self.timeline_ui.borrow_mut();
            ui.selection = selection.clone().into_iter().collect();
            ui.selection_explicit = true;
            ui.selected = selection;
        }
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
            .is_some_and(|track| {
                track
                    .keyframes
                    .binary_search_by(|key| key.time.total_cmp(&time))
                    .is_ok()
            });
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
    /// Selects keys across object/property lanes. The final key becomes the primary editor.
    pub fn select_keyframes(self: &Rc<Self>, keys: &[(NodeId, String, f64)]) -> Result<(), String> {
        let session = self.session.borrow();
        let mut selection = Vec::with_capacity(keys.len());
        for (target, property, time) in keys {
            if !time.is_finite()
                || !session
                    .timeline()
                    .track(*target, property)
                    .is_some_and(|track| {
                        track
                            .keyframes
                            .binary_search_by(|key| key.time.total_cmp(time))
                            .is_ok()
                    })
            {
                return Err("The requested keyframe does not exist".into());
            }
            let key = SelectedKey {
                target: *target,
                property: property.clone(),
                time: *time,
            };
            if !selection.contains(&key) {
                selection.push(key);
            }
        }
        let primary = selection.last().cloned();
        let mut ui = self.timeline_ui.borrow_mut();
        ui.document = session.document_uri().to_owned();
        ui.program = session.active_animation_program();
        ui.selection = selection;
        ui.selection_explicit = true;
        ui.selected = primary.clone();
        drop(ui);
        drop(session);
        if let Some(primary) = primary {
            self.session.borrow_mut().select([primary.target]);
            self.session.borrow_mut().seek(primary.time)?;
        }
        self.refresh(true);
        Ok(())
    }
    pub fn selected_keyframes(&self) -> Vec<(NodeId, String, f64)> {
        self.timeline_ui
            .borrow()
            .selection
            .iter()
            .map(|key| (key.target, key.property.clone(), key.time))
            .collect()
    }
    /// One source transaction and one undo step, including moves across multiple tracks.
    pub fn move_selected_keyframes(self: &Rc<Self>, delta: f64) -> Result<(), String> {
        self.edit_selected_keys(KeyframeEdit::Move(delta))?;
        self.refresh(true);
        Ok(())
    }
    pub fn scale_selected_keyframes(self: &Rc<Self>, pivot: f64, factor: f64) -> Result<(), String> {
        self.edit_selected_keys(KeyframeEdit::Scale { pivot, factor })?;
        self.refresh(true);
        Ok(())
    }
    pub fn duplicate_selected_keyframes(self: &Rc<Self>, delta: f64) -> Result<(), String> {
        self.edit_selected_keys(KeyframeEdit::Duplicate(delta))?;
        self.refresh(true);
        Ok(())
    }
    pub fn remove_selected_keyframes(self: &Rc<Self>) -> Result<(), String> {
        self.edit_selected_keys(KeyframeEdit::Remove)?;
        self.refresh(true);
        Ok(())
    }
    pub fn timeline_frame_rate(&self) -> Option<u32> {
        self.timeline_ui.borrow().frame_rate
    }
    /// Changes only the editing grid; authored seconds and the running animation are unchanged.
    pub fn set_timeline_frame_rate(&self, rate: Option<u32>) {
        let rate = rate.filter(|rate| (1..=1000).contains(rate));
        let picker = {
            let mut ui = self.timeline_ui.borrow_mut();
            ui.frame_rate = rate;
            ui.snap_picker.as_ref().and_then(WeakRef::upgrade)
        };
        if let Some(picker) = picker {
            let label = rate.map_or_else(|| "Sec".to_owned(), |rate| format!("{rate} fps"));
            let found = (0..picker.items().count()).find(|index| {
                picker
                    .items()
                    .get_at(*index)
                    .and_then(|item| item.downcast_ref::<String>().cloned())
                    .as_deref()
                    == Some(label.as_str())
            });
            let index = found.unwrap_or_else(|| {
                let index = picker.items().count();
                picker.items().add(Some(Rc::new(label)));
                index
            });
            if picker.selected_index() != index as i32 {
                picker.set_selected_index(index as i32);
            }
        }
        for lane in self
            .timeline_lanes
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
        {
            lane.set_frame_rate(rate);
        }
    }
    fn accept_lane_selection(
        &self,
        target: NodeId,
        property: &str,
        times: &[f64],
        primary: Option<f64>,
        additive: bool,
    ) {
        let mut ui = self.timeline_ui.borrow_mut();
        ui.selection_explicit = true;
        if additive {
            ui.selection
                .retain(|key| key.target != target || key.property != property);
        } else {
            ui.selection.clear();
        }
        ui.selection.extend(times.iter().map(|time| SelectedKey {
            target,
            property: property.to_owned(),
            time: *time,
        }));
        ui.selected = primary
            .map(|time| SelectedKey {
                target,
                property: property.to_owned(),
                time,
            })
            .or_else(|| ui.selection.last().cloned());
        drop(ui);
        self.session.borrow_mut().select([target]);
    }
    fn prepare_lane_batch(&self, target: NodeId, property: &str, times: &[f64]) {
        let ui = self.timeline_ui.borrow();
        let mut previous = ui
            .selection
            .iter()
            .filter(|key| key.target == target && key.property == property)
            .map(|key| key.time)
            .collect::<Vec<_>>();
        previous.sort_by(f64::total_cmp);
        let preserve_other_lanes = previous == times;
        let primary = times.last().copied();
        drop(ui);
        self.accept_lane_selection(target, property, times, primary, preserve_other_lanes);
    }
    fn edit_selected_keys(&self, operation: KeyframeEdit) -> Result<(), String> {
        let ui = self.timeline_ui.borrow();
        let mut session = self.session.borrow_mut();
        if ui.document != session.document_uri() || ui.program != session.active_animation_program()
        {
            return Err(
                "The animation selection belongs to a different document or program".into(),
            );
        }
        let selected = ui.selection.clone();
        let primary = ui.selected.clone();
        drop(ui);
        let id = session
            .active_animation_program()
            .ok_or("Choose an animation")?;
        let changed = session.edit_program_keyframes(id, &selected, operation)?;
        drop(session);
        let primary = primary.and_then(|key| selected.iter().position(|original| *original == key));
        self.accept_timeline_key_batch(primary, changed);
        Ok(())
    }
    fn accept_timeline_key_batch(&self, primary: Option<usize>, changed: Vec<SelectedKey>) {
        let mut ui = self.timeline_ui.borrow_mut();
        ui.selected = primary.and_then(|index| changed.get(index)).or_else(|| changed.last()).cloned();
        ui.selection = changed;
    }
    fn build_selection_inspector(self: &Rc<Self>, context: TimelineContext) {
        let count = self.timeline_ui.borrow().selection.len();
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(6.0);
        row.classes().add("designer-key-inspector");
        row.set_margin(Thickness::new(4.0, 6.0, 0.0, 6.0));
        row.children()
            .add(key_caption(&format!("{count} keyframes selected")).upcast::<Control>());
        let offset = key_field(&row, "Offset", "KeyframeOffset", "0.1", 64.0);
        let factor = key_field(&row, "Factor", "KeyframeScaleFactor", "2", 56.0);
        for (name, label) in [
            ("MoveKeyframes", "Move"),
            ("ScaleKeyframes", "Scale from playhead"),
            ("DuplicateKeyframes", "Duplicate"),
            ("RemoveKeyframes", "Delete"),
        ] {
            let control = button(label);
            control.classes().add("designer-timeline-tool");
            if name == "MoveKeyframes" {
                control.classes().add("designer-motion");
            } else {
                control.classes().add("nav");
            }
            control.set_name(Some(name.into()));
            control.set_min_height(26.0);
            control.set_font_size(11.0);
            let weak = Rc::downgrade(self);
            let field = if name == "ScaleKeyframes" { &factor } else { &offset }.downgrade();
            let context = context.clone();
            control.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let result = context.check(&this).and_then(|_| {
                        let operation = if name == "RemoveKeyframes" {
                            KeyframeEdit::Remove
                        } else {
                            let value: f64 = field
                                .upgrade()
                                .ok_or("The timing editor was closed")?
                                .text()
                                .unwrap_or_default()
                                .parse()
                                .map_err(|_| "Enter a numeric offset or scale factor")?;
                            match name {
                                "MoveKeyframes" => KeyframeEdit::Move(value),
                                "ScaleKeyframes" => KeyframeEdit::Scale { pivot: this.session.borrow().time(), factor: value },
                                _ => KeyframeEdit::Duplicate(value),
                            }
                        };
                        this.edit_selected_keys(operation)
                    });
                    if result.is_ok() {
                        this.refresh(true);
                    }
                    this.report(result);
                }
            });
            row.children().add(control.upcast::<Control>());
        }
        self.root
            .get_control::<ContentControl>("TimelineInspectorHost")
            .set_content(Some(Control::boxed(&row)));
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
    fn remember_keyframe(&self, mut selected: SelectedKey) {
        let session = self.session.borrow();
        selected.reconcile_time(session.timeline());
        let mut ui = self.timeline_ui.borrow_mut();
        if ui.document != session.document_uri() || ui.program != session.active_animation_program()
        {
            ui.view = None;
        }
        ui.document = session.document_uri().to_owned();
        ui.program = session.active_animation_program();
        ui.selection_explicit = true;
        ui.selection = vec![selected.clone()];
        ui.selected = Some(selected);
    }
    fn record_timeline_property(
        &self,
        target: NodeId,
        property: &str,
        time: f64,
    ) -> Result<(), String> {
        self.session
            .borrow_mut()
            .record_effective_keyframe(target, property, time)
    }
    fn build_keyframe_inspector(
        self: &Rc<Self>,
        selected: &SelectedKey,
        key: &Keyframe,
        context: TimelineContext,
    ) {
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(6.0);
        row.classes().add("designer-key-inspector");
        row.set_margin(Thickness::new(4.0, 6.0, 0.0, 6.0));
        row.children()
            .add(key_caption(&format!("{} keyframe", selected.property)).upcast::<Control>());
        let time = key_field(&row, "Time", "KeyframeTime", &key.time.to_string(), 65.0);
        let value = key_field(&row, "Value", "KeyframeValue", &key.value.to_xaml(), 104.0);
        let (curve, preview) = easing::easing_fields(&row, &key.easing, !self.session.borrow().is_read_only());
        let apply = button("Apply keyframe");
        apply.classes().add("designer-motion");
        apply.set_name(Some("ApplyKeyframe".into()));
        apply.set_font_size(11.0);
        apply.set_min_height(26.0);
        apply.set_padding(Thickness::symmetric(8.0, 3.0));
        let weak = Rc::downgrade(self);
        let selected = selected.clone();
        let editing = selected.clone();
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
                    let time: f64 = time
                        .text()
                        .unwrap_or_default()
                        .parse()
                        .map_err(|_| "Invalid keyframe time")?;
                    this.apply_timeline_keyframe(
                        &commit_context,
                        &editing,
                        time,
                        &value.text().unwrap_or_default(),
                        &curve.text().unwrap_or_default(),
                    )
                    .map(|_| ())
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
        for input in [time.upcast::<Control>(), value.upcast(), curve.upcast(), preview.upcast()] {
            let commit = commit.clone();
            input.add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Enter {
                    event.set_handled(true);
                    commit();
                }
            });
        }
        row.children().add(apply.upcast::<Control>());
        self.root
            .get_control::<ContentControl>("TimelineInspectorHost")
            .set_content(Some(Control::boxed(&row)));
        let remove = button("Remove");
        remove.classes().add("nav");
        remove.classes().add("designer-timeline-tool");
        remove.set_content(Some(Control::boxed(timeline_icon("F1 M5,2 L11,2 11,4 14,4 14,5.5 2,5.5 2,4 5,4 Z M4,7 L12,7 11.2,14 4.8,14 Z M6,8 L6.5,12.5 7.5,12.5 7,8 Z M9,8 L8.5,12.5 9.5,12.5 10,8 Z", 0xffa9adba))));
        remove.set_width(28.0);
        remove.set_font_size(11.0);
        remove.set_min_height(26.0);
        remove.set_padding(Thickness::symmetric(8.0, 3.0));
        AutomationProperties::set_name(&remove, Some("Remove selected keyframe"));
        ToolTip::set_tip(
            &remove,
            Some(Rc::new("Remove selected keyframe · Delete".to_owned())),
        );
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
fn timeline_icon(path: &str, color: u32) -> Ref<PathIcon> {
    let icon = crate::icons::vector_icon(path, 14.0);
    icon.set_foreground(Some(SolidColorBrush::from_uint32(color).into()));
    icon
}
fn key_caption(value: &str) -> Ref<StackPanel> {
    let caption = StackPanel::new();
    caption.set_orientation(Orientation::Horizontal);
    caption.set_spacing(8.0);
    caption.set_width(TRACK_HEADER - 10.0);
    caption.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    let icon = timeline_icon("F1 M8,1 L15,8 8,15 1,8 Z M8,4 L4,8 8,12 12,8 Z", 0xffb6a2f6);
    caption.children().add(icon.upcast::<Control>());
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_font_size(11.0);
    label.set_font_weight(FontWeight::SemiBold);
    label.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    label.set_max_width(TRACK_HEADER - 40.0);
    caption.children().add(label.upcast::<Control>());
    caption
}
fn track_header(target: &str, property: &str, key_count: usize, selected: bool) -> Ref<Button> {
    let title = Button::new();
    title.classes().add("designer-track-header");
    title.classes().set("active", selected);
    title.set_border_thickness(Thickness::new(0.0, 0.0, 1.0, 1.0));
    title.set_border_brush(Some(SolidColorBrush::from_uint32(0xff303239).into()));
    title.set_background(Some(
        SolidColorBrush::from_uint32(if selected { 0xff2b2c3b } else { 0xff25262b }).into(),
    ));
    let content = Grid::new();
    content.set_column_definitions(ColumnDefinitions::parse("22,*,Auto").unwrap());
    let icon = timeline_icon(
        "F1 M2,2 L14,2 14,14 2,14 Z M3.5,3.5 L3.5,12.5 12.5,12.5 12.5,3.5 Z M6,6 L10,6 10,10 6,10 Z",
        0xffb6a2f6,
    );
    icon.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    content.children().add(icon.upcast::<Control>());
    let labels = StackPanel::new();
    labels.set_spacing(0.0);
    labels.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    let name = TextBlock::new();
    name.set_text(Some(target));
    name.set_font_size(11.0);
    name.set_font_weight(FontWeight::SemiBold);
    name.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    labels.children().add(name.upcast::<Control>());
    let detail = TextBlock::new();
    detail.set_text(Some(property));
    detail.set_font_size(10.0);
    detail.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
    detail.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    labels.children().add(detail.upcast::<Control>());
    Grid::set_column(&labels, 1);
    content.children().add(labels.upcast::<Control>());
    let count = TextBlock::new();
    count.set_text(Some(&key_count.to_string()));
    count.set_font_size(10.0);
    count.set_foreground(Some(SolidColorBrush::from_uint32(0xff979caa).into()));
    count.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    count.set_margin(Thickness::new(8.0, 0.0, 0.0, 0.0));
    Grid::set_column(&count, 2);
    content.children().add(count.upcast::<Control>());
    title.set_content(Some(Control::boxed(&content)));
    AutomationProperties::set_name(
        &title,
        Some(&format!("{target}, {property}, {key_count} keyframes")),
    );
    title
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
    label.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
    label.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    panel.children().add(label.upcast::<Control>());
    let input = TextBox::new();
    input.classes().add("designer-field");
    input.set_name(Some(name.into()));
    input.set_text(Some(value));
    input.set_width(width);
    input.set_font_size(11.0);
    input.set_min_height(26.0);
    input.set_height(26.0);
    input.set_padding(Thickness::symmetric(6.0, 3.0));
    AutomationProperties::set_name(&input, Some(&format!("Keyframe {caption}")));
    panel.children().add(input.clone().upcast::<Control>());
    input
}

#[cfg(test)]
#[path = "timeline_selection_tests.rs"]
mod selection_tests;
