//! Native frame/style guide stacks sharing inspector draft and lifecycle semantics.
use super::*;
use ferrodesign_core::{LayoutGrid, LayoutGridAlignment, LayoutGridAxis, LayoutGridEdit, LayoutGridKind,
    LayoutGridPattern, LayoutGuideEdit, LayoutGuideProperty};
#[path = "layout_grids/variables.rs"]
mod guide_variables;

#[derive(Clone)]
enum GuideTarget { Frames(Vec<NodeId>), Style(StyleId) }

fn control_name(target: &GuideTarget, stem: &str, index: usize) -> String {
    let prefix = match target { GuideTarget::Frames(_) => "LayoutGrid", GuideTarget::Style(_) => "StyleGrid" };
    if index == 0 { format!("{prefix}{stem}") } else { format!("{prefix}{stem}{index}") }
}

fn pattern(index: usize, axis: LayoutGridAxis) -> LayoutGridPattern {
    match index { 1 => LayoutGridPattern::Columns { axis }, 2 => LayoutGridPattern::Rows { axis }, _ => LayoutGridPattern::Square }
}
fn style_grids(value: &StyleValue) -> Result<Vec<LayoutGrid>, String> {
    match value {
        StyleValue::Grid { size, color } => Ok(vec![LayoutGrid { size: *size, color: *color, ..LayoutGrid::default() }]),
        StyleValue::GridStack { grids } => Ok(grids.clone()),
        _ => Err(stale_style()),
    }
}
fn edit_style_grids(style: &mut Style, edit: LayoutGuideEdit) -> Result<(), String> {
    let mut grids = style_grids(&style.value)?;
    edit.apply(&mut grids).map_err(|error| error.to_string())?;
    style.value = match grids.as_slice() {
        [grid] if grid.pattern.is_square() => StyleValue::Grid { size: grid.size, color: grid.color },
        _ => StyleValue::GridStack { grids },
    };
    Ok(())
}

impl EditorShell {
    pub(in crate::shell) fn layout_grid_dialog(self: &Rc<Self>) {
        if !self.flush_inspector_drafts() { return; }
        let snapshot = {
            let editor = self.editor.borrow();
            let selection = editor.selection.clone();
            let source = selection.iter().filter_map(|id| editor.document.nodes.get(id))
                .find(|node| LayoutGrid::supports(node.kind) && !editor.document.effective_locked(node.id));
            source.map(|source| {
                let grids: Vec<_> = source.layout_guides().copied().collect();
                let mixed = selection.iter().filter_map(|id| editor.document.nodes.get(id))
                    .filter(|node| LayoutGrid::supports(node.kind) && !editor.document.effective_locked(node.id))
                    .any(|node| !node.layout_guides().eq(source.layout_guides()));
                (selection.clone(), source.name.clone(), grids, mixed)
            })
        };
        let Some((selection, name, grids, mixed)) = snapshot else {
            self.toast("Select an unlocked frame, component or instance to edit layout guides.");
            return;
        };
        self.updating.set(true);
        self.modal("Layout guides", &format!("{name} · Square, column and row guides stay inside the frame and are excluded from exports and presentations."));
        self.updating.set(false);
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(10.0);
        host.children().add(style_label("With Snap enabled, objects align to guide lines and band edges. Stretch columns and rows are the references for child constraints; hidden guides still work."));
        if mixed { host.children().add(style_label("Mixed guide stacks · each indexed setting updates matching rows on unlocked selected frames.")); }
        let target = GuideTarget::Frames(selection.clone());
        for (index, grid) in grids.iter().copied().enumerate() {
            self.layout_guide_controls(&host, target.clone(), index, grid, grids.len());
        }
        if grids.is_empty() { host.children().add(style_label("This frame has no guides. Add a square grid, columns or rows, or apply a local style.")); }
        self.layout_guide_add_controls(&host, target, grids.len());
        let row = style_row();
        let selected = selection.clone();
        row.children().add(self.style_button("Add missing grids", "LayoutGridAdd", true, move |shell| {
            if !shell.flush_inspector_drafts() { return; }
            let changed = {
                let mut editor = shell.editor.borrow_mut();
                if editor.selection != selected {
                    drop(editor);
                    shell.toast("The selection changed. Reopen guide settings before editing.");
                    return;
                }
                editor.edit_layout_grids(&selected, LayoutGridEdit::Add)
            };
            match changed {
                Ok(changed) => { if changed != 0 { shell.changed(); } shell.layout_grid_dialog(); }
                Err(error) => shell.toast(&error.to_string()),
            }
        }));
        let selected = selection.clone();
        row.children().add(self.style_button("Remove all guides", "LayoutGridRemove", !grids.is_empty() || mixed, move |shell| {
            shell.guide_action(GuideTarget::Frames(selected.clone()), LayoutGuideEdit::Clear);
        }));
        host.children().add(row);
        let actions = self.control::<StackPanel>("ModalActions");
        actions.children().add(self.style_button("Create grid style", "LayoutGridCreateStyle", true,
            |shell| shell.create_style("create-grid-style")));
        actions.children().add(self.style_button("Local styles", "LayoutGridStyles", true, |shell| shell.styles_dialog()));
        let done = self.style_button("Done", "LayoutGridDone", true, |shell| shell.close_modal());
        done.classes().add("primary");
        actions.children().add(done);
    }

    pub(super) fn style_layout_guides(self: &Rc<Self>, host: &Ref<StackPanel>, id: StyleId, value: &StyleValue) {
        let Ok(grids) = style_grids(value) else { return; };
        host.children().add(style_label("The complete guide stack propagates to linked frames. Per-frame visibility stays local; guides never enter artwork exports or presentation."));
        for (index, grid) in grids.iter().copied().enumerate() {
            self.layout_guide_controls(host, GuideTarget::Style(id), index, grid, grids.len());
        }
        self.layout_guide_add_controls(host, GuideTarget::Style(id), grids.len());
    }

    fn layout_guide_controls(self: &Rc<Self>, host: &Ref<StackPanel>, target: GuideTarget, index: usize, grid: LayoutGrid, count: usize) {
        let panel = StackPanel::new(); panel.set_spacing(5.0);
        panel.set_name(Some(control_name(&target, "Card", index)));
        panel.children().add(style_label(&format!("Guide {}", index + 1)));
        let selected = match grid.pattern { LayoutGridPattern::Square => 0, LayoutGridPattern::Columns { .. } => 1, LayoutGridPattern::Rows { .. } => 2 };
        let kinds = [LayoutGridKind::Square, LayoutGridKind::Columns, LayoutGridKind::Rows];
        panel.children().add(self.layout_guide_choice(target.clone(), &control_name(&target, "Pattern", index),
            &["Square grid", "Columns", "Rows"], selected, move |choice| LayoutGuideEdit::Set {
                index, property: LayoutGuideProperty::Kind(kinds[choice]),
            }));
        if let Some(axis) = grid.pattern.axis() {
            let labels = if selected == 1 { ["Left", "Center", "Right", "Stretch"] } else { ["Top", "Center", "Bottom", "Stretch"] };
            let alignments = [LayoutGridAlignment::Start, LayoutGridAlignment::Center, LayoutGridAlignment::End, LayoutGridAlignment::Stretch];
            let selected = alignments.iter().position(|alignment| *alignment == axis.alignment).unwrap();
            panel.children().add(self.layout_guide_choice(target.clone(), &control_name(&target, "Alignment", index), &labels,
                selected, move |choice| LayoutGuideEdit::Set { index, property: LayoutGuideProperty::Alignment(alignments[choice]) }));
            panel.children().add(self.layout_guide_field(target.clone(), "Count · number or Auto", &control_name(&target, "Count", index),
                &axis.count.map_or_else(|| "Auto".into(), |count| count.to_string()), move |text| {
                    let count = if text.trim().eq_ignore_ascii_case("auto") { None }
                        else { Some(text.trim().parse::<u32>().map_err(|_| "Enter a positive count or Auto.".to_string())?) };
                    Ok(LayoutGuideEdit::Set { index, property: LayoutGuideProperty::Count(count) })
                }));
            panel.children().add(self.layout_guide_field(target.clone(), "Gutter", &control_name(&target, "Gutter", index), &axis.gutter.to_string(), move |text| {
                Ok(LayoutGuideEdit::Set { index, property: LayoutGuideProperty::Gutter(style_number(text)?) })
            }));
            if axis.alignment != LayoutGridAlignment::Center {
                panel.children().add(self.layout_guide_field(target.clone(), if axis.alignment == LayoutGridAlignment::Stretch { "Margin" } else { "Offset" },
                    &control_name(&target, "Offset", index), &axis.offset.to_string(), move |text| {
                        Ok(LayoutGuideEdit::Set { index, property: LayoutGuideProperty::Offset(style_number(text)?) })
                    }));
            }
        }
        let caption = match grid.pattern.axis() {
            Some(axis) if axis.alignment == LayoutGridAlignment::Stretch => "Preferred size for Auto count · fixed count stretches automatically",
            Some(_) if matches!(grid.pattern, LayoutGridPattern::Columns { .. }) => "Column width",
            Some(_) => "Row height",
            None => "Spacing",
        };
        panel.children().add(self.layout_guide_field(target.clone(), caption, &control_name(&target, "Size", index), &grid.size.to_string(), move |text| {
            Ok(LayoutGuideEdit::Set { index, property: LayoutGuideProperty::Size(style_number(text)?) })
        }));
        panel.children().add(self.layout_guide_field(target.clone(), "Color · RGBA hex", &control_name(&target, "Color", index), &grid.color.hex(), move |text| {
            Ok(LayoutGuideEdit::Set { index, property: LayoutGuideProperty::Color(style_color(text)?) })
        }));
        if matches!(target, GuideTarget::Frames(_)) {
            let toggle = CheckBox::new(); toggle.set_name(Some(control_name(&target, "Visible", index)));
            toggle.set_content(Some(Rc::new("Show guide on this frame".to_string()))); toggle.set_is_checked(Some(grid.visible));
            let weak = Rc::downgrade(self); let weak_toggle = toggle.downgrade(); let target = target.clone();
            toggle.is_checked_changed(move |_, _| {
                let (Some(shell), Some(toggle)) = (weak.upgrade(), weak_toggle.upgrade()) else { return; };
                if shell.updating.get() || !toggle.is_attached_to_visual_tree() { return; }
                if !shell.flush_inspector_drafts() {
                    shell.updating.set(true); toggle.set_is_checked(Some(grid.visible)); shell.updating.set(false); return;
                }
                let result = shell.apply_guide_edit(&target, LayoutGuideEdit::Set { index, property: LayoutGuideProperty::Visible(toggle.is_checked() == Some(true)) });
                match result {
                    Ok(_) => shell.refresh_guide_editor(&target),
                    Err(error) => { shell.updating.set(true); toggle.set_is_checked(Some(grid.visible)); shell.updating.set(false); shell.toast(&error); }
                }
            });
            panel.children().add(toggle);
        }
        if let GuideTarget::Frames(selection) = &target {
            let selection = selection.clone();
            panel.children().add(self.style_button("Bind variable…", &control_name(&target, "Variables", index), true,
                move |shell| shell.guide_variables_dialog(selection.clone(), index, core::LayoutGuideVariableField::Size)));
        }
        let row = style_row();
        for (label, stem, enabled, edit) in [
            ("↑", "Up", index > 0, LayoutGuideEdit::Move { index, to: index.saturating_sub(1) }),
            ("↓", "Down", index + 1 < count, LayoutGuideEdit::Move { index, to: index + 1 }),
            ("Duplicate", "Duplicate", count < core::MAX_LAYOUT_GUIDES, LayoutGuideEdit::Duplicate(index)),
            ("Remove", "RemoveGuide", true, LayoutGuideEdit::Remove(index)),
        ] {
            let action_target = target.clone();
            row.children().add(self.style_button(label, &control_name(&target, stem, index), enabled,
                move |shell| shell.guide_action(action_target.clone(), edit)));
        }
        panel.children().add(row); host.children().add(panel);
    }

    fn layout_guide_add_controls(self: &Rc<Self>, host: &Ref<StackPanel>, target: GuideTarget, count: usize) {
        let row = style_row();
        for (index, label, stem) in [(0, "+ Square", "AddSquare"), (1, "+ Columns", "AddColumns"), (2, "+ Rows", "AddRows")] {
            let action_target = target.clone();
            row.children().add(self.style_button(label, &control_name(&target, stem, 0), count < core::MAX_LAYOUT_GUIDES, move |shell| {
                let grid = LayoutGrid { pattern: pattern(index, LayoutGridAxis::default()), ..LayoutGrid::default() };
                shell.guide_action(action_target.clone(), LayoutGuideEdit::Add(grid));
            }));
        }
        host.children().add(row);
    }

    fn guide_action(self: &Rc<Self>, target: GuideTarget, edit: LayoutGuideEdit) {
        if self.updating.get() || !self.flush_inspector_drafts() { return; }
        match self.apply_guide_edit(&target, edit) {
            Ok(_) => self.refresh_guide_editor(&target),
            Err(error) => self.toast(&error),
        }
    }

    fn refresh_guide_editor(self: &Rc<Self>, target: &GuideTarget) {
        match target { GuideTarget::Frames(_) => self.layout_grid_dialog(), GuideTarget::Style(id) => self.style_dialog(*id) }
    }

    fn apply_guide_edit(self: &Rc<Self>, target: &GuideTarget, edit: LayoutGuideEdit) -> Result<bool, String> {
        match target {
            GuideTarget::Style(id) => self.apply_style_edit(*id, |style| edit_style_grids(style, edit)),
            GuideTarget::Frames(selection) => {
                let result = {
                    let mut editor = self.editor.borrow_mut();
                    if editor.selection != *selection { return Err("The selection changed. Reopen guide settings before editing.".into()); }
                    editor.edit_layout_guides(selection, edit).map_err(|error| error.to_string())?
                };
                if result != 0 { self.changed(); }
                Ok(result != 0)
            }
        }
    }

    fn layout_guide_choice(self: &Rc<Self>, target: GuideTarget, name: &str, labels: &[&str], selected: usize,
        edit: impl Fn(usize) -> LayoutGuideEdit + 'static) -> Ref<ComboBox> {
        let choice = ComboBox::new(); choice.set_name(Some(name.into()));
        for label in labels { choice.items().add(Some(Rc::new(label.to_string()))); }
        choice.set_selected_index(selected as i32);
        let weak = Rc::downgrade(self); let weak_choice = choice.downgrade(); let length = labels.len();
        choice.selection_changed(move |_, _| {
            let (Some(shell), Some(choice)) = (weak.upgrade(), weak_choice.upgrade()) else { return; };
            let Ok(index) = usize::try_from(choice.selected_index()) else { return; };
            if index >= length || shell.updating.get() || !choice.is_attached_to_visual_tree() { return; }
            if !shell.flush_inspector_drafts() {
                shell.updating.set(true); choice.set_selected_index(selected as i32); shell.updating.set(false); return;
            }
            match shell.apply_guide_edit(&target, edit(index)) {
                Ok(_) => shell.refresh_guide_editor(&target),
                Err(error) => { shell.updating.set(true); choice.set_selected_index(selected as i32); shell.updating.set(false); shell.toast(&error); }
            }
        });
        choice
    }

    fn layout_guide_field(
        self: &Rc<Self>, target: GuideTarget, caption: &str, name: &str, value: &str,
        parse: impl Fn(&str) -> Result<LayoutGuideEdit, String> + 'static,
    ) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(4.0);
        panel.children().add(style_label(caption));
        let field = TextBox::new();
        field.set_name(Some(name.into()));
        field.set_text(Some(value));
        panel.children().add(field.clone());
        let error = style_label("");
        error.set_name(Some(format!("{name}Error")));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        panel.children().add(error.clone());
        let committed = Rc::new(RefCell::new(value.to_string()));
        let weak = Rc::downgrade(self);
        let weak_field = field.downgrade();
        let weak_error = error.downgrade();
        let previous = committed.clone();
        let commit_value: Rc<dyn Fn(bool)> = Rc::new(move |explicit| {
            let (Some(shell), Some(field), Some(error)) = (weak.upgrade(), weak_field.upgrade(), weak_error.upgrade()) else { return; };
            if shell.updating.get() || field.is_composing_text() || !field.is_attached_to_visual_tree() || !field.is_effectively_visible() { return; }
            let text = field.text().unwrap_or_default();
            // Enter is an explicit literal assignment: a bound value must be
            // detachable even when the typed number equals its current value.
            // Mere focus loss still leaves an untouched binding in place.
            if !explicit && text == *previous.borrow() { return; }
            match parse(&text).and_then(|edit| shell.apply_guide_edit(&target, edit)) {
                Ok(_) => {
                    *previous.borrow_mut() = text;
                    error.set_is_visible(false);
                    field.set_border_brush(Some(icons::brush("#E5E5E5")));
                }
                Err(message) => {
                    error.set_text(Some(&message));
                    error.set_is_visible(true);
                    field.set_border_brush(Some(icons::brush("#C8372B")));
                }
            }
            shell.sync_document_draft_dirty();
        });
        let flush = commit_value.clone();
        let commit: Rc<dyn Fn()> = Rc::new(move || flush(false));
        let blur = commit.clone();
        field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
        let enter = commit_value;
        let weak_field = field.downgrade();
        field.add_handler_with(InputElement::key_down_event(), move |_, event| {
            if event.key == Key::Enter && !weak_field.upgrade().is_some_and(|field| field.is_composing_text()) {
                enter(true);
                event.set_handled(true);
            }
        }, RoutingStrategies::TUNNEL, false);
        self.bind_inspector_draft(&field, committed, commit, Some(&error));
        panel
    }
}
