//! Native width-profile presets and ordered point fields in the XAML inspector.
use super::*;
use ferrodesign_core::{StrokeWidthPoint, StrokeWidthProfile};
use ferroui_base::interactivity::RoutingStrategies;

const LABELS: [&str; 7] = ["Uniform", "Wedge", "Taper", "Quarter taper", "Eye", "Mirrored taper", "Custom points…"];

struct WidthContext {
    selection: Vec<NodeId>,
    stroke: usize,
    points: RefCell<Vec<StrokeWidthPoint>>,
    error: WeakRef<TextBlock>,
}

impl EditorShell {
    pub(super) fn wire_stroke_profiles(self: &Rc<Self>) {
        let choice = self.control::<ComboBox>("StrokeProfileChoice");
        for label in LABELS { choice.items().add(Some(Rc::new(label.to_string()))); }
        let weak = Rc::downgrade(self);
        choice.selection_changed(move |_, _| {
            let Some(shell) = weak.upgrade() else { return; };
            if shell.updating.get() { return; }
            // Committing another inspector field can refresh this ComboBox.
            // Capture the requested choice first, but keep invalid drafts and
            // restore the actual profile instead of replacing their controls.
            let requested = shell.control::<ComboBox>("StrokeProfileChoice").selected_index();
            if !(0..LABELS.len() as i32).contains(&requested) { return; }
            if !shell.flush_inspector_drafts() {
                shell.restore_stroke_profile_choice();
                return;
            }
            let profile = match requested {
                0 => StrokeWidthProfile::Uniform, 1 => StrokeWidthProfile::Wedge,
                2 => StrokeWidthProfile::Taper, 3 => StrokeWidthProfile::QuarterTaper,
                4 => StrokeWidthProfile::Eye, 5 => StrokeWidthProfile::MirroredTaper,
                6 => { shell.stroke_profile_dialog(0); return; }, _ => return,
            };
            shell.canvas.finish_stroke_width_edit();
            let ids = shell.editor.borrow().selection.clone();
            let result = shell.editor.borrow_mut().set_stroke_width_profile(&ids, 0, profile);
            match result { Ok(_) => shell.changed(), Err(error) => { shell.toast(&error.to_string()); shell.refresh(); } }
        });
        self.hook("StrokeWidthCustomButton", "stroke-width-points");
        self.hook("StrokeWidthHandlesButton", "stroke-width-handles");
    }

    pub(super) fn refresh_stroke_profiles(&self, node: Option<&Node>) {
        let profile = node.and_then(|node| node.strokes.first()).map(|stroke| &stroke.width_profile);
        self.control::<ComboBox>("StrokeProfileChoice").set_selected_index(width_profile_index(profile));
        let supported = node.is_some_and(|node| node.strokes.first().is_some_and(|stroke| {
            let mut candidate = Stroke::new(Color::BLACK, stroke.width); candidate.width_profile = StrokeWidthProfile::Wedge;
            candidate.cap = if stroke.cap == core::StrokeCap::Arrow { core::StrokeCap::Butt } else { stroke.cap };
            candidate.dash = stroke.dash.clone();
            core::validate_variable_width_support(node, &candidate).is_ok()
        }));
        self.button("StrokeWidthCustomButton").set_is_enabled(supported);
        self.button("StrokeWidthHandlesButton").set_is_enabled(supported);
    }

    fn restore_stroke_profile_choice(&self) {
        let index = {
            let editor = self.editor.borrow();
            width_profile_index(editor.selection.first().and_then(|id| editor.document.nodes.get(id))
                .and_then(|node| node.strokes.first()).map(|stroke| &stroke.width_profile))
        };
        let previous = self.updating.replace(true);
        self.control::<ComboBox>("StrokeProfileChoice").set_selected_index(index);
        self.updating.set(previous);
    }

    pub(super) fn stroke_profile_dialog(self: &Rc<Self>, index: usize) {
        // Opening or rebuilding the modal must never discard a dirty field.
        if self.updating.get() || !self.flush_inspector_drafts() { return; }
        self.canvas.finish_stroke_width_edit();
        let snapshot = {
            let editor = self.editor.borrow();
            editor.selection.iter().find(|id| !editor.document.effective_locked(**id))
                .and_then(|id| editor.document.nodes.get(id))
                .and_then(|node| node.strokes.get(index).map(|stroke|
                    (node.name.clone(), node.strokes.len(), stroke.width_profile.control_points().to_vec(), editor.selection.clone())))
        };
        let Some((name, count, points, selection)) = snapshot else {
            self.toast("Select an unlocked layer with a stroke."); return;
        };
        // A discarded field must not commit into a rebuilt point stack.
        let previous = self.updating.replace(true);
        self.modal("Stroke width", &format!("{name} · Position follows the path length. Width is a percentage of the base stroke width. Enter or Tab applies; Escape restores a field."));
        let host = self.control::<StackPanel>("ModalContent"); host.set_spacing(10.0);
        let error = width_label(""); error.set_name(Some("StrokeProfileError".into()));
        error.set_foreground(Some(icons::brush("#C8372B"))); error.set_is_visible(false);
        host.children().add(error.clone());
        let context = Rc::new(WidthContext { selection, stroke: index, points: RefCell::new(points.clone()), error: error.downgrade() });
        let stroke_choice = ComboBox::new(); stroke_choice.set_name(Some("StrokeProfileStackChoice".into()));
        for number in 0..count { stroke_choice.items().add(Some(Rc::new(format!("Stroke {}", number + 1)))); }
        stroke_choice.set_selected_index(index as i32);
        let weak_choice = stroke_choice.downgrade(); let weak_shell = Rc::downgrade(self);
        stroke_choice.selection_changed(move |_, _| {
            if let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) {
                if !shell.width_dialog_open() || !choice.is_attached_to_visual_tree() { return; }
                let Ok(requested) = usize::try_from(choice.selected_index()) else { return; };
                if !shell.flush_inspector_drafts() {
                    let previous = shell.updating.replace(true);
                    choice.set_selected_index(index as i32);
                    shell.updating.set(previous);
                    return;
                }
                shell.stroke_profile_dialog(requested);
            }
        });
        host.children().add(stroke_choice);
        for (row, point) in points.iter().enumerate() {
            let grid = Grid::new(); grid.set_column_definitions(ColumnDefinitions::parse("*,12,*,12,72").unwrap());
            for (column, position, label, value) in [(0, true, "Position %", point.position * 100.0), (2, false, "Width %", point.width * 100.0)] {
                let panel = self.width_point_field(&context, row, position, label, value);
                Grid::set_column(&panel, column); grid.children().add(panel);
            }
            let remove = Button::new(); remove.set_name(Some(format!("StrokeWidthRemove{row}")));
            remove.set_content(Some(Rc::new("Remove".to_string()))); remove.set_is_enabled(points.len() > 1);
            Grid::set_column(&remove, 4); grid.children().add(remove.clone());
            let weak_shell = Rc::downgrade(self); let context = context.clone();
            remove.click(move |_, _| {
                let Some(shell) = weak_shell.upgrade().filter(|shell| shell.width_dialog_open()) else { return; };
                if !shell.flush_inspector_drafts() { return; }
                let mut points = context.points.borrow().clone();
                if row >= points.len() || points.len() == 1 { return; }
                points.remove(row);
                if shell.apply_width_points(&context, points).is_ok() { shell.stroke_profile_dialog(context.stroke); }
            });
            host.children().add(grid);
        }
        let actions = self.control::<StackPanel>("ModalActions");
        let add = Button::new(); add.set_name(Some("StrokeWidthAddPoint".into())); add.set_content(Some(Rc::new("Add point".to_string())));
        add.set_is_enabled(points.len() < core::MAX_STROKE_WIDTH_POINTS);
        let weak_shell = Rc::downgrade(self); let add_context = context.clone();
        add.click(move |_, _| {
            let Some(shell) = weak_shell.upgrade().filter(|shell| shell.width_dialog_open()) else { return; };
            if !shell.flush_inspector_drafts() { return; }
            let profile = StrokeWidthProfile::Custom { points: add_context.points.borrow().clone() };
            let mut points = profile.control_points().to_vec();
            let mut positions = vec![0.0]; positions.extend(points.iter().map(|point| point.position)); positions.push(1.0);
            let Some(gap) = positions.windows(2).max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0]))) else { return; };
            let position = (gap[0] + gap[1]) * 0.5;
            if gap[1] <= gap[0] || position == gap[0] || position == gap[1] { return; }
            let insert = points.partition_point(|point| point.position < position);
            points.insert(insert, StrokeWidthPoint::new(position, profile.sample(position)));
            if shell.apply_width_points(&add_context, points).is_ok() { shell.stroke_profile_dialog(add_context.stroke); }
        });
        actions.children().add(add);
        let done = Button::new(); done.set_name(Some("StrokeWidthDone".into())); done.set_content(Some(Rc::new("Done".to_string())));
        done.classes().add("primary"); let weak_shell = Rc::downgrade(self);
        done.click(move |_, _| { if let Some(shell) = weak_shell.upgrade() { shell.close_modal(); } });
        actions.children().add(done);
        self.updating.set(previous);
    }

    fn width_dialog_open(&self) -> bool {
        !self.updating.get() && self.control::<Control>("ModalBackdrop").is_visible()
            && self.control::<TextBlock>("ModalTitle").text().as_deref() == Some("Stroke width")
    }

    fn apply_width_points(self: &Rc<Self>, context: &WidthContext, points: Vec<StrokeWidthPoint>) -> Result<(), String> {
        if !self.width_dialog_open() { return Ok(()); }
        let result = if self.editor.borrow().selection != context.selection {
            Err("Selection changed. Close and reopen the width editor.".to_string())
        } else {
            self.editor.borrow_mut().set_stroke_width_profile(&context.selection, context.stroke,
                StrokeWidthProfile::Custom { points: points.clone() }).map_err(|error| error.to_string())
        };
        match result {
            Ok(changed) => {
                *context.points.borrow_mut() = points;
                if let Some(error) = context.error.upgrade() { error.set_is_visible(false); }
                if changed != 0 { self.changed(); }
                Ok(())
            }
            Err(message) => {
                if let Some(error) = context.error.upgrade() { error.set_text(Some(&message)); error.set_is_visible(true); }
                Err(message)
            }
        }
    }

    fn width_point_field(self: &Rc<Self>, context: &Rc<WidthContext>, row: usize, position: bool,
        label: &str, value: f64) -> Ref<StackPanel>
    {
        let panel = StackPanel::new(); panel.set_spacing(4.0); panel.children().add(width_label(label));
        let field = TextBox::new(); field.set_name(Some(format!("StrokeWidth{}{}", if position { "Position" } else { "Value" }, row)));
        let initial = number(value); field.set_text(Some(&initial)); panel.children().add(field.clone());
        let committed = Rc::new(RefCell::new(initial)); let committed_value = committed.clone();
        let weak_shell = Rc::downgrade(self); let weak_field = field.downgrade(); let context = context.clone();
        let error = context.error.upgrade();
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(shell), Some(field)) = (weak_shell.upgrade(), weak_field.upgrade()) else { return; };
            if !shell.width_dialog_open() || field.is_composing_text() { return; }
            let text = field.text().unwrap_or_default(); if text == *committed_value.borrow() { return; }
            let result = width_percent(&text, position).and_then(|value| {
                let mut points = context.points.borrow().clone();
                let point = points.get_mut(row).ok_or("This width point no longer exists.")?;
                if position { point.position = value; } else { point.width = value; }
                shell.apply_width_points(&context, points)
            });
            match result {
                Ok(()) => { *committed_value.borrow_mut() = text; field.set_border_brush(Some(icons::brush("#E5E5E5"))); }
                Err(message) => {
                    field.set_border_brush(Some(icons::brush("#C8372B")));
                    if let Some(error) = context.error.upgrade() { error.set_text(Some(&message)); error.set_is_visible(true); }
                }
            }
        });
        let blur = commit.clone(); field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
        let enter = commit.clone(); let weak_field = field.downgrade();
        field.add_handler_with(InputElement::key_down_event(), move |_, event| {
            if event.key == Key::Enter && weak_field.upgrade().is_some_and(|field| !field.is_composing_text()) {
                enter(); event.set_handled(true);
            }
        }, RoutingStrategies::TUNNEL, false);
        self.bind_inspector_draft(&field, committed, commit, error.as_ref());
        panel
    }
}

fn width_profile_index(profile: Option<&StrokeWidthProfile>) -> i32 {
    profile.map(|profile| match profile {
        StrokeWidthProfile::Uniform => 0, StrokeWidthProfile::Wedge => 1,
        StrokeWidthProfile::Taper => 2, StrokeWidthProfile::QuarterTaper => 3,
        StrokeWidthProfile::Eye => 4, StrokeWidthProfile::MirroredTaper => 5,
        StrokeWidthProfile::Custom { .. } => 6,
    }).unwrap_or(-1)
}

fn width_label(value: &str) -> Ref<TextBlock> {
    let label = TextBlock::new(); label.set_text(Some(value)); label.set_font_size(11.0);
    label.set_text_wrapping(TextWrapping::Wrap); label
}

fn width_percent(text: &str, position: bool) -> Result<f64, String> {
    let value = text.trim().trim_end_matches('%').trim().parse::<f64>()
        .map_err(|_| "Enter a finite nonnegative percentage.".to_string())?;
    if !value.is_finite() || value < 0.0 || (position && value > 100.0) {
        return Err(if position { "Position must be between 0 and 100%." } else { "Width must be a finite nonnegative percentage." }.into());
    }
    Ok(value / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn width_fields_reject_nonfinite_negative_and_out_of_path_positions() {
        for text in ["NaN", "inf", "-1", "1e999", "", "25px"] { assert!(width_percent(text, false).is_err()); }
        assert!(width_percent("101%", true).is_err());
        assert_eq!(width_percent(" 12.5% ", true).unwrap(), 0.125);
        assert_eq!(width_percent("240", false).unwrap(), 2.4);
    }
}
