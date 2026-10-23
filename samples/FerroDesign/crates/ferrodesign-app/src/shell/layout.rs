//! Native child placement, sizing and flow-order authoring.
use super::*;
use ferrodesign_core::{GridPlacement, LayoutChild, SizingMode};

#[path = "layout_padding.rs"]
mod padding;

struct UpdatingLayout<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingLayout<'_> { fn drop(&mut self) { self.0.set(self.1); } }

struct LayoutSnapshot {
    parent: NodeId,
    child: Option<NodeId>,
    name: String,
    parent_name: String,
    settings: LayoutChild,
    grid: bool,
    reverse: bool,
    children: Vec<(NodeId, String)>,
    layout: core::AutoLayout,
}

// Padding and track controls edit the same frame without rebuilding the
// dialog. Share their accepted baseline, but reject another editor, selection,
// page or intervening document edit before applying a still-visible draft.
struct LayoutEditGuard {
    token: core::EditorRenderToken,
    parent: Node,
    selection: Vec<NodeId>,
    page: NodeId,
}
impl LayoutEditGuard {
    fn new(editor: &Editor, parent: NodeId) -> Self {
        Self { token: editor.render_change_token(), parent: editor.document.nodes[&parent].clone(),
            selection: editor.selection.clone(), page: editor.active_page }
    }
    fn matches(&self, editor: &Editor) -> bool {
        self.token == editor.render_change_token() && self.selection == editor.selection
            && self.page == editor.active_page && editor.document.nodes.get(&self.parent.id) == Some(&self.parent)
    }
    fn accept(&mut self, editor: &Editor) { *self = Self::new(editor, self.parent.id); }
}

struct LayoutInputs {
    row: WeakRef<TextBox>, column: WeakRef<TextBox>,
    row_span: WeakRef<TextBox>, column_span: WeakRef<TextBox>,
    width: WeakRef<ComboBox>, height: WeakRef<ComboBox>,
    min_width: WeakRef<TextBox>, max_width: WeakRef<TextBox>,
    min_height: WeakRef<TextBox>, max_height: WeakRef<TextBox>,
    grow: WeakRef<TextBox>, absolute: WeakRef<CheckBox>,
}
impl LayoutInputs {
    fn read(&self, mut settings: LayoutChild, is_grid: bool) -> Result<LayoutChild, String> {
        let text = |field: &WeakRef<TextBox>| field.upgrade().map(|field| field.text().unwrap_or_default())
            .ok_or_else(|| "The layout editor was closed.".to_string());
        let mode = |choice: &WeakRef<ComboBox>| match choice.upgrade().map(|choice| choice.selected_index()) {
            Some(0) => Ok(SizingMode::Fixed), Some(1) => Ok(SizingMode::Hug), Some(2) => Ok(SizingMode::Fill),
            _ => Err("Choose a width and height sizing mode.".to_string()),
        };
        settings.width = mode(&self.width)?;
        settings.height = mode(&self.height)?;
        settings.min_width = layout_number(&text(&self.min_width)?, "Minimum width")?;
        settings.min_height = layout_number(&text(&self.min_height)?, "Minimum height")?;
        settings.max_width = layout_optional_number(&text(&self.max_width)?, "Maximum width")?;
        settings.max_height = layout_optional_number(&text(&self.max_height)?, "Maximum height")?;
        settings.grow = layout_number(&text(&self.grow)?, "Growth weight")?;
        settings.absolute = self.absolute.upgrade().ok_or_else(|| "The layout editor was closed.".to_string())?.is_checked() == Some(true);
        if is_grid {
            let grid = GridPlacement {
                row: layout_track(&text(&self.row)?, core::MAX_GRID_ROWS, "Row")?,
                column: layout_track(&text(&self.column)?, core::MAX_GRID_COLUMNS, "Column")?,
                row_span: layout_span(&text(&self.row_span)?, core::MAX_GRID_ROWS, "Row span")?,
                column_span: layout_span(&text(&self.column_span)?, core::MAX_GRID_COLUMNS, "Column span")?,
            };
            settings.grid = (grid != GridPlacement::default()).then_some(grid);
        }
        Ok(settings)
    }
}

impl EditorShell {
    pub(super) fn layout_dialog(self: &Rc<Self>) { self.layout_view(None); }

    fn layout_view(self: &Rc<Self>, requested: Option<NodeId>) {
        self.layout_view_at(requested, None);
    }

    fn layout_view_at(self: &Rc<Self>, requested: Option<NodeId>, parent_id: Option<NodeId>) {
        let snapshot = {
            let editor = self.editor.borrow();
            let document = &editor.document;
            let selected = requested.or_else(|| editor.selection.first().copied())
                .and_then(|id| document.nodes.get(&id));
            let parent = parent_id.and_then(|id| document.nodes.get(&id))
                .filter(|node| node.layout.is_some())
                .or_else(|| selected.filter(|node| node.layout.is_some()))
                .or_else(|| selected.and_then(|node| node.parent.and_then(|parent| document.nodes.get(&parent))
                    .filter(|parent| parent.layout.is_some())));
            parent.and_then(|parent| {
                let layout = parent.layout.as_ref()?;
                let mut children: Vec<_> = parent.children.iter().filter_map(|id| document.nodes.get(id))
                    .map(|node| (node.id, format!("{}{}{}", node.name,
                        if !node.visible { " · hidden" } else { "" },
                        if node.layout_child.absolute { " · absolute" } else { "" }))).collect();
                if layout.reverse { children.reverse(); }
                let child = selected.filter(|node| node.parent == Some(parent.id))
                    .or_else(|| children.first().and_then(|(id, _)| document.nodes.get(id)));
                Some(LayoutSnapshot { parent: parent.id, child: child.map(|node| node.id),
                    name: child.map(|node| node.name.clone()).unwrap_or_else(|| "Empty frame".into()), parent_name: parent.name.clone(),
                    settings: child.map(|node| node.layout_child).unwrap_or_default(), grid: layout.axis == LayoutAxis::Grid,
                    reverse: layout.reverse, children, layout: layout.clone() })
            })
        };
        let _updating = UpdatingLayout(&self.updating, self.updating.replace(true));
        self.modal("Layout padding, placement and order", "Edit frame padding, size grid tracks or a child, and arrange layers. Each committed edit is one undo step.");
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(12.0);
        let actions = self.control::<StackPanel>("ModalActions");
        actions.children().add(self.layout_button("Done", "LayoutDone", true, |shell| shell.close_modal()));
        actions.children().add(self.layout_button("Save design", "LayoutSave", true, |shell| { shell.command("save"); }));
        let Some(snapshot) = snapshot else {
            host.children().add(layout_text("Select an auto-layout frame or one of its direct children.", true));
            return;
        };
        let parent_id = snapshot.parent;
        let settings = snapshot.settings;
        let is_grid = snapshot.grid;
        host.children().add(layout_text(&format!("{} · {}", snapshot.parent_name, snapshot.name), false));

        let error = layout_text("", true);
        error.set_name(Some("LayoutError".into()));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        host.children().add(error.clone());
        let guard = Rc::new(RefCell::new(LayoutEditGuard::new(&self.editor.borrow(), parent_id)));
        self.layout_padding_controls(&snapshot, &error, guard.clone());
        if is_grid { self.layout_track_controls(&snapshot, &error, guard); }
        let Some(child_id) = snapshot.child else {
            host.children().add(layout_text("This frame has no children. Add or drag layers into it to edit child sizing and placement.", true));
            return;
        };
        let labels: Vec<_> = snapshot.children.iter().map(|(_, name)| name.clone()).collect();
        let child_choice = layout_combo("LayoutChildChoice", &labels,
            snapshot.children.iter().position(|(id, _)| *id == child_id).unwrap_or(0));
        let ids: Vec<_> = snapshot.children.iter().map(|(id, _)| *id).collect();
        let weak_shell = Rc::downgrade(self);
        let weak_choice = child_choice.downgrade();
        child_choice.selection_changed(move |_, _| {
            if let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) {
                if shell.updating.get() { return; }
                if let Some(id) = usize::try_from(choice.selected_index()).ok().and_then(|index| ids.get(index)).copied() {
                    if !shell.flush_inspector_drafts() {
                        let _updating = UpdatingLayout(&shell.updating, shell.updating.replace(true));
                        choice.set_selected_index(ids.iter().position(|id| *id == child_id).unwrap_or(0) as i32);
                        return;
                    }
                    shell.layout_view_at(Some(id), Some(parent_id));
                }
            }
        });
        host.children().add(child_choice);

        let grid = settings.grid.unwrap_or_default();
        host.children().add(layout_text(if is_grid { "Grid placement" } else { "Grid placement is available in a grid frame." }, false));
        host.children().add(layout_text("Rows and columns start at 1. Leave either blank or type Auto for automatic placement. Pinned cells reserve space before automatic children.", true));
        let row = layout_input("LayoutGridRow", &grid.row.map(|row| (row + 1).to_string()).unwrap_or_default());
        let column = layout_input("LayoutGridColumn", &grid.column.map(|column| (column + 1).to_string()).unwrap_or_default());
        let row_span = layout_input("LayoutGridRowSpan", &grid.row_span.to_string());
        let column_span = layout_input("LayoutGridColumnSpan", &grid.column_span.to_string());
        for field in [&row, &column, &row_span, &column_span] { field.set_is_enabled(is_grid); }
        host.children().add(layout_pair("Row · Auto if blank", &row, "Column · Auto if blank", &column));
        host.children().add(layout_pair("Row span", &row_span, "Column span", &column_span));
        let weak_error = error.downgrade();
        host.children().add(self.layout_button("Reset placement to Auto", "LayoutClearPlacement", is_grid, move |shell| {
            shell.layout_apply_result(child_id, &weak_error,
                |editor| editor.set_grid_placement(child_id, None));
        }));

        host.children().add(layout_text("Child sizing", false));
        let sizing = vec!["Fixed".to_string(), "Hug".to_string(), "Fill".to_string()];
        let width = layout_combo("LayoutWidthMode", &sizing, sizing_index(settings.width) as usize);
        let height = layout_combo("LayoutHeightMode", &sizing, sizing_index(settings.height) as usize);
        host.children().add(layout_choice_pair("Width", &width, "Height", &height));
        let min_width = layout_input("LayoutMinWidth", &number(settings.min_width));
        let max_width = layout_input("LayoutMaxWidth", &settings.max_width.map(number).unwrap_or_default());
        let min_height = layout_input("LayoutMinHeight", &number(settings.min_height));
        let max_height = layout_input("LayoutMaxHeight", &settings.max_height.map(number).unwrap_or_default());
        host.children().add(layout_pair("Minimum width", &min_width, "Maximum width · blank for none", &max_width));
        host.children().add(layout_pair("Minimum height", &min_height, "Maximum height · blank for none", &max_height));
        let grow = layout_input("LayoutGrow", &number(settings.grow));
        let grow_panel = layout_panel();
        grow_panel.children().add(layout_text("Growth weight", true));
        grow_panel.children().add(grow.clone());
        host.children().add(grow_panel);
        let absolute = CheckBox::new();
        absolute.set_name(Some("LayoutAbsolute".into()));
        absolute.set_content(Some(Rc::new("Absolute position · exclude from layout flow".to_string())));
        absolute.set_is_checked(Some(settings.absolute));
        host.children().add(absolute.clone());
        let inputs = LayoutInputs { row: row.downgrade(), column: column.downgrade(),
            row_span: row_span.downgrade(), column_span: column_span.downgrade(),
            width: width.downgrade(), height: height.downgrade(),
            min_width: min_width.downgrade(), max_width: max_width.downgrade(),
            min_height: min_height.downgrade(), max_height: max_height.downgrade(),
            grow: grow.downgrade(), absolute: absolute.downgrade() };
        let weak_error = error.downgrade();
        let apply = self.layout_button("Apply placement and sizing", "LayoutApply", true, move |shell| {
            match inputs.read(settings, is_grid) {
                Ok(settings) => shell.layout_apply_result(child_id, &weak_error,
                    |editor| editor.set_layout_child(child_id, settings)),
                Err(message) => layout_show_error(&weak_error, &message),
            }
        });
        apply.classes().add("primary");
        actions.children().add(apply);

        host.children().add(layout_text("Layer order", false));
        host.children().add(layout_text(if snapshot.reverse {
            "Before and after follow the reversed visual flow. Pinned grid cells keep their coordinates."
        } else {
            "Before and after follow visual flow. Pinned grid cells keep their coordinates."
        }, true));
        let targets: Vec<_> = snapshot.children.iter().filter(|(id, _)| *id != child_id).cloned().collect();
        let labels: Vec<_> = targets.iter().map(|(_, name)| name.clone()).collect();
        let target = layout_combo("LayoutReorderTarget", &labels, 0);
        host.children().add(target.clone());
        let reorder = layout_grid("*,*");
        for (column, (caption, name, after)) in [
            ("Move before", "LayoutMoveBefore", false), ("Move after", "LayoutMoveAfter", true)
        ].into_iter().enumerate() {
            let weak_target = target.downgrade();
            let targets = targets.clone();
            let weak_error = error.downgrade();
            let button = self.layout_button(caption, name, !targets.is_empty(), move |shell| {
                let Some(target) = weak_target.upgrade() else { return; };
                let Some((anchor, _)) = usize::try_from(target.selected_index()).ok().and_then(|index| targets.get(index)) else { return; };
                let anchor = *anchor;
                shell.layout_apply_result(child_id, &weak_error,
                    |editor| editor.reorder_layout_child(child_id, anchor, after));
            });
            Grid::set_column(&button, column as i32);
            reorder.children().add(button);
        }
        host.children().add(reorder);
    }

    fn layout_apply_result(self: &Rc<Self>, child: NodeId, error: &WeakRef<TextBlock>,
        operation: impl FnOnce(&mut Editor) -> Result<core::LayoutResult, core::DocumentError>) {
        if self.updating.get() { return; }
        if !self.flush_inspector_drafts() { return; }
        let result = { let mut editor = self.editor.borrow_mut(); operation(&mut editor) };
        match result {
            Ok(result) => {
                self.changed();
                let parent = self.editor.borrow().document.nodes.get(&child).and_then(|node| node.parent);
                self.layout_view_at(Some(child), parent);
                if !result.warnings.is_empty() { self.toast(&result.warnings.join("\n")); }
            }
            Err(message) => layout_show_error(error, &message.to_string()),
        }
    }

    fn layout_track_controls(self: &Rc<Self>, snapshot: &LayoutSnapshot, error: &Ref<TextBlock>,
        expected: Rc<RefCell<LayoutEditGuard>>) {
        let host = self.control::<StackPanel>("ModalContent");
        host.children().add(layout_text("Grid track sizing", false));
        host.children().add(layout_text("Separate tracks with commas: 120, auto, 2fr. Sizes are pixels, auto fits content, and fr shares remaining space. A blank list restores implicit tracks. Enter, Tab, Apply, or Save commits both lists together.", true));
        let columns = layout_input("LayoutColumnTracks", &core::format_grid_tracks(&snapshot.layout.column_tracks));
        let rows = layout_input("LayoutRowTracks", &core::format_grid_tracks(&snapshot.layout.row_tracks));
        host.children().add(layout_pair("Column tracks", &columns, "Row tracks", &rows));
        let committed_columns = Rc::new(RefCell::new(columns.text().unwrap_or_default()));
        let committed_rows = Rc::new(RefCell::new(rows.text().unwrap_or_default()));
        let weak_columns = columns.downgrade(); let weak_rows = rows.downgrade();
        let weak_error = error.downgrade(); let weak_shell = Rc::downgrade(self);
        let previous_columns = committed_columns.clone(); let previous_rows = committed_rows.clone();
        let parent = snapshot.parent;
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(shell), Some(columns), Some(rows)) = (weak_shell.upgrade(), weak_columns.upgrade(), weak_rows.upgrade()) else { return; };
            if shell.updating.get() || !columns.is_attached_to_visual_tree() || !columns.is_effectively_visible()
                || columns.is_composing_text() || rows.is_composing_text() { return; }
            let column_text = columns.text().unwrap_or_default(); let row_text = rows.text().unwrap_or_default();
            if column_text == *previous_columns.borrow() && row_text == *previous_rows.borrow() { return; }
            let parsed = core::parse_grid_tracks(&column_text, core::MAX_GRID_COLUMNS)
                .and_then(|columns| core::parse_grid_tracks(&row_text, core::MAX_GRID_ROWS).map(|rows| (columns, rows)));
            let result = parsed.and_then(|(columns, rows)| {
                let mut editor = shell.editor.borrow_mut();
                if !expected.borrow().matches(&editor) {
                    return Err(core::DocumentError::Invalid("The frame changed. Reopen layout authoring before applying these tracks.".into()));
                }
                let revision = editor.revision;
                editor.set_grid_tracks(parent, columns, rows).map(|result| (result, editor.revision != revision))
            });
            match result {
                Ok((result, changed)) => {
                    *previous_columns.borrow_mut() = column_text;
                    *previous_rows.borrow_mut() = row_text;
                    expected.borrow_mut().accept(&shell.editor.borrow());
                    if let Some(error) = weak_error.upgrade() { error.set_is_visible(false); }
                    if changed { shell.changed(); }
                    if !result.warnings.is_empty() { shell.toast(&result.warnings.join("\n")); }
                }
                Err(error) => layout_show_error(&weak_error, &error.to_string()),
            }
            shell.sync_document_draft_dirty();
        });
        for (field, committed) in [(&columns, committed_columns), (&rows, committed_rows)] {
            let blur = commit.clone();
            field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
            let enter = commit.clone(); let weak_field = field.downgrade();
            field.add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Enter && weak_field.upgrade().is_some_and(|field| !field.is_composing_text()) {
                    enter(); event.set_handled(true);
                }
            });
            self.bind_inspector_draft(field, committed, commit.clone(), Some(error));
        }
        host.children().add(self.layout_button("Apply grid tracks", "LayoutApplyTracks", true, move |_| commit()));
    }

    fn layout_button(self: &Rc<Self>, caption: &str, name: &str, enabled: bool,
        operation: impl Fn(&Rc<Self>) + 'static) -> Ref<Button> {
        let button = Button::new();
        button.set_name(Some(name.into()));
        button.set_content(Some(Rc::new(caption.to_string())));
        button.set_is_enabled(enabled);
        button.set_height(30.0);
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(shell) = weak.upgrade() { if !shell.updating.get() { operation(&shell); } }
        });
        button
    }
}

fn layout_show_error(error: &WeakRef<TextBlock>, message: &str) {
    if let Some(error) = error.upgrade() { error.set_text(Some(message)); error.set_is_visible(true); }
}
fn layout_panel() -> Ref<StackPanel> { let panel = StackPanel::new(); panel.set_spacing(6.0); panel }
fn layout_grid(columns: &str) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(columns).expect("layout columns"));
    grid.set_column_spacing(8.0); grid
}
fn layout_text(value: &str, muted: bool) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(if muted { 11.0 } else { 12.0 });
    text.set_foreground(Some(icons::brush(if muted { "#737373" } else { "#242424" })));
    text.set_text_wrapping(TextWrapping::Wrap); text
}
fn layout_input(name: &str, value: &str) -> Ref<TextBox> {
    let field = TextBox::new(); field.set_name(Some(name.into())); field.set_text(Some(value)); field
}
fn layout_combo(name: &str, labels: &[String], selected: usize) -> Ref<ComboBox> {
    let choice = ComboBox::new(); choice.set_name(Some(name.into()));
    for label in labels { choice.items().add(Some(Rc::new(label.clone()))); }
    choice.set_selected_index(if labels.is_empty() { -1 } else { selected.min(labels.len() - 1) as i32 });
    choice.set_is_enabled(!labels.is_empty()); choice
}
fn layout_pair(left_label: &str, left: &Ref<TextBox>, right_label: &str, right: &Ref<TextBox>) -> Ref<Grid> {
    let grid = layout_grid("*,*");
    for (column, (label, field)) in [(left_label, left), (right_label, right)].into_iter().enumerate() {
        let panel = layout_panel(); panel.children().add(layout_text(label, true)); panel.children().add(field.clone());
        Grid::set_column(&panel, column as i32); grid.children().add(panel);
    }
    grid
}
fn layout_choice_pair(left_label: &str, left: &Ref<ComboBox>, right_label: &str, right: &Ref<ComboBox>) -> Ref<Grid> {
    let grid = layout_grid("*,*");
    for (column, (label, field)) in [(left_label, left), (right_label, right)].into_iter().enumerate() {
        let panel = layout_panel(); panel.children().add(layout_text(label, true)); panel.children().add(field.clone());
        Grid::set_column(&panel, column as i32); grid.children().add(panel);
    }
    grid
}
fn layout_number(value: &str, label: &str) -> Result<f64, String> {
    value.trim().parse::<f64>().ok().filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| format!("{label} must be a finite nonnegative number."))
}
fn layout_optional_number(value: &str, label: &str) -> Result<Option<f64>, String> {
    if value.trim().is_empty() { Ok(None) } else { layout_number(value, label).map(Some) }
}
fn layout_track(value: &str, maximum: usize, label: &str) -> Result<Option<usize>, String> {
    if value.trim().is_empty() || value.trim().eq_ignore_ascii_case("auto") { return Ok(None); }
    layout_span(value, maximum, label).map(|value| Some(value - 1))
}
fn layout_span(value: &str, maximum: usize, label: &str) -> Result<usize, String> {
    value.trim().parse::<usize>().ok().filter(|value| (1..=maximum).contains(value))
        .ok_or_else(|| format!("{label} must be a whole number from 1 to {maximum}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_grid_fields_translate_one_based_tracks_and_reject_invalid_numbers() {
        assert_eq!(layout_track("", 4096, "Column"), Ok(None));
        assert_eq!(layout_track(" Auto ", 4096, "Column"), Ok(None));
        assert_eq!(layout_track("1", 4096, "Column"), Ok(Some(0)));
        assert_eq!(layout_track("4096", 4096, "Column"), Ok(Some(4095)));
        for text in ["0", "-1", "1.5", "4097", "NaN", "inf"] {
            assert!(layout_track(text, 4096, "Column").is_err());
        }
        assert_eq!(layout_span("2", 4096, "Span"), Ok(2));
        assert!(layout_span("", 4096, "Span").is_err());
        assert!(layout_number("NaN", "Size").is_err());
        assert!(layout_number("-2", "Size").is_err());
        assert_eq!(layout_optional_number("", "Maximum"), Ok(None));
    }
}
