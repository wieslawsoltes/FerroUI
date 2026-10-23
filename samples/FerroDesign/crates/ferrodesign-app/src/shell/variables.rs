//! Visual local-variable tables and typed native authoring controls.
use super::*;
use ferrodesign_core::{VariableEdit, VariableId, VariableKind, VariableValue};

const PAGE_SIZE: usize = 12;
const KINDS: [VariableKind; 4] = [VariableKind::Color, VariableKind::Number, VariableKind::String, VariableKind::Boolean];
const BINDINGS: [(&str, &str); 15] = [
    ("fill", "Fill"), ("stroke", "Stroke"), ("width", "Width"), ("height", "Height"),
    ("opacity", "Opacity"), ("corner_radius", "Corner radius"), ("font_size", "Font size"),
    ("text", "Text content"), ("visible", "Visibility"),
    ("layout.padding.top", "Top padding"), ("layout.padding.right", "Right padding"),
    ("layout.padding.bottom", "Bottom padding"), ("layout.padding.left", "Left padding"),
    ("layout.gap", "Layout gap"), ("layout.cross_gap", "Layout cross gap"),
];

#[derive(Clone, Default)]
struct VariableView {
    collection: Option<String>,
    variable: Option<VariableId>,
    mode: Option<String>,
    draft: bool,
    page: usize,
}

struct UpdatingVariables<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingVariables<'_> {
    fn drop(&mut self) { self.0.set(self.1); }
}

#[derive(Clone)]
struct AliasChoice { id: VariableId, label: String, kind: VariableKind }

impl EditorShell {
    pub(super) fn visual_variables_dialog(self: &Rc<Self>) {
        self.variables_view(VariableView::default());
    }

    fn variables_view(self: &Rc<Self>, mut view: VariableView) {
        let (collections, modes, active_mode, rows, total, variable, aliases, selection_count, validation) = {
            let editor = self.editor.borrow();
            let document = &editor.document;
            let collections = document.variable_collections();
            let modes = document.variable_modes();
            let collection = if view.draft {
                view.collection.clone().unwrap_or_else(|| "Local".into())
            } else {
                view.collection.clone().filter(|collection| collections.contains(collection))
                    .or_else(|| collections.first().cloned()).unwrap_or_else(|| "Local".into())
            };
            view.collection = Some(collection.clone());
            let mode = view.mode.clone().filter(|mode| modes.contains(mode))
                .or_else(|| modes.contains(&document.variable_mode).then(|| document.variable_mode.clone()))
                .unwrap_or_else(|| "Default".into());
            view.mode = Some(mode.clone());
            let ids: Vec<_> = document.variables.values().filter(|variable| variable.collection == collection).map(|variable| variable.id).collect();
            let total = ids.len();
            view.page = view.page.min(total.saturating_sub(1) / PAGE_SIZE);
            let kinds = document.variable_kinds(&mode).unwrap_or_default();
            let rows: Vec<_> = ids.iter().skip(view.page * PAGE_SIZE).take(PAGE_SIZE).map(|id| {
                let variable = &document.variables[id];
                let value = mode_value(variable, &mode);
                (*id, variable.name.clone(), kinds.get(id).copied(), variable_value_label(document, value),
                    document.resolve_variable(*id, &mode).and_then(|value| if let VariableValue::Color(color) = value { Some(*color) } else { None }))
            }).collect();
            let variable = if view.draft { None } else {
                view.variable.filter(|id| ids.contains(id)).or_else(|| ids.first().copied())
                    .and_then(|id| document.variables.get(&id)).cloned()
            };
            view.variable = variable.as_ref().map(|variable| variable.id);
            if variable.is_none() { view.draft = true; }
            let aliases: Vec<_> = document.variables.values().filter(|variable| Some(variable.id) != view.variable)
                .filter_map(|variable| kinds.get(&variable.id).copied().map(|kind| AliasChoice {
                    id: variable.id, label: format!("{} / {}", variable.collection, variable.name), kind,
                })).collect();
            (collections, modes, document.variable_mode.clone(), rows, total, variable, aliases,
                editor.selection.len(), document.validate_variables().err().map(|error| error.to_string()))
        };
        let _updating = UpdatingVariables(&self.updating, self.updating.replace(true));
        self.modal("Local variables", &format!(
            "Preview mode: {active_mode}. Modes apply across all collections. Changes and their bound layer updates support undo."
        ));
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(10.0);
        let error = variable_text("", false);
        error.set_name(Some("VariablesError".into()));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        if let Some(message) = validation { error.set_text(Some(&message)); error.set_is_visible(true); }
        host.children().add(error.clone());
        let collection = view.collection.clone().unwrap();
        let mode = view.mode.clone().unwrap();

        let collection_row = variable_grid("82,*,122");
        collection_row.children().add(variable_text("Collection", true));
        let collection_choice = variable_choice("VariablesCollection", &collections,
            collections.iter().position(|value| *value == collection).unwrap_or(0));
        collection_choice.set_is_enabled(!collections.is_empty());
        Grid::set_column(&collection_choice, 1);
        collection_row.children().add(collection_choice.clone());
        let weak_shell = Rc::downgrade(self);
        let weak_choice = collection_choice.downgrade();
        let state = view.clone();
        let collection_values = collections.clone();
        collection_choice.selection_changed(move |_, _| {
            if let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) {
                if shell.updating.get() { return; }
                if let Some(collection) = selected_item(&collection_values, choice.selected_index()) {
                    shell.variables_view(VariableView { collection: Some(collection.clone()), variable: None, draft: false, page: 0, ..state.clone() });
                }
            }
        });
        let state = view.clone();
        let new_collection = self.variable_button("New collection", "VariablesNewCollection", true, move |shell| {
            shell.variables_view(VariableView { collection: Some("New collection".into()), variable: None, draft: true, page: 0, ..state.clone() });
        });
        Grid::set_column(&new_collection, 2);
        collection_row.children().add(new_collection);
        host.children().add(collection_row);

        let mode_row = variable_grid("82,*,122");
        mode_row.children().add(variable_text("Mode shown", true));
        let mode_choice = variable_choice("VariablesMode", &modes, modes.iter().position(|value| *value == mode).unwrap_or(0));
        Grid::set_column(&mode_choice, 1);
        mode_row.children().add(mode_choice.clone());
        let weak_shell = Rc::downgrade(self);
        let weak_choice = mode_choice.downgrade();
        let state = view.clone();
        let mode_values = modes.clone();
        mode_choice.selection_changed(move |_, _| {
            if let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) {
                if shell.updating.get() { return; }
                if let Some(mode) = selected_item(&mode_values, choice.selected_index()) {
                    shell.variables_view(VariableView { mode: Some(mode.clone()), ..state.clone() });
                }
            }
        });
        let state = view.clone();
        let activate_mode = mode.clone();
        let weak_error = error.downgrade();
        let activate = self.variable_button("Use this mode", "VariablesActivateMode", mode != active_mode, move |shell| {
            if let Some(error) = weak_error.upgrade() {
                if shell.commit_variable_edit(VariableEdit::ActivateMode { mode: activate_mode.clone() }, &error).is_some() {
                    shell.variables_view(state.clone());
                }
            }
        });
        Grid::set_column(&activate, 2);
        mode_row.children().add(activate);
        host.children().add(mode_row);
        self.variable_mode_controls(&host, &view, &error);
        self.variable_collection_controls(&host, &view, &collections, &error);

        let table_header = variable_grid("*,68,*,54");
        for (index, value) in ["Variable", "Type", "Value", ""].into_iter().enumerate() {
            let text = variable_text(value, true);
            Grid::set_column(&text, index as i32);
            table_header.children().add(text);
        }
        host.children().add(table_header);
        if rows.is_empty() {
            host.children().add(variable_text("Save the first variable below to create this collection.", true));
        }
        for (id, name, kind, value, color) in rows {
            let row = variable_grid("*,68,*,54");
            let caption = variable_text(&short_value(&name, 40), false);
            if Some(id) == view.variable { caption.set_font_weight(FontWeight::SemiBold); }
            row.children().add(caption);
            let kind = variable_text(kind.map(VariableKind::label).unwrap_or("Invalid"), true);
            Grid::set_column(&kind, 1);
            row.children().add(kind);
            let value_panel = StackPanel::new();
            value_panel.set_orientation(Orientation::Horizontal);
            value_panel.set_spacing(5.0);
            if let Some(color) = color {
                let swatch = Border::new();
                swatch.set_width(16.0);
                swatch.set_height(16.0);
                swatch.set_background(Some(icons::brush(&format!("#{:02X}{:02X}{:02X}{:02X}", color.a, color.r, color.g, color.b))));
                value_panel.children().add(swatch);
            }
            value_panel.children().add(variable_text(&short_value(&value, 32), false));
            Grid::set_column(&value_panel, 2);
            row.children().add(value_panel);
            let state = view.clone();
            let edit = self.variable_button("Edit", &format!("VariableEdit{id}"), true, move |shell| {
                shell.variables_view(VariableView { variable: Some(id), draft: false, ..state.clone() });
            });
            Grid::set_column(&edit, 3);
            row.children().add(edit);
            host.children().add(row);
        }
        let pages = total.div_ceil(PAGE_SIZE).max(1);
        let pager = variable_grid("80,*,80,118");
        let state = view.clone();
        pager.children().add(self.variable_button("Previous", "VariablesPreviousPage", view.page > 0, move |shell| {
            shell.variables_view(VariableView { page: state.page.saturating_sub(1), ..state.clone() });
        }));
        let count = variable_text(&format!("{} variables · page {} / {pages}", total, view.page + 1), true);
        Grid::set_column(&count, 1);
        pager.children().add(count);
        let state = view.clone();
        let next = self.variable_button("Next", "VariablesNextPage", view.page + 1 < pages, move |shell| {
            shell.variables_view(VariableView { page: state.page + 1, ..state.clone() });
        });
        Grid::set_column(&next, 2);
        pager.children().add(next);
        let state = view.clone();
        let add = self.variable_button("Add variable", "VariablesAdd", true, move |shell| {
            shell.variables_view(VariableView { variable: None, draft: true, ..state.clone() });
        });
        Grid::set_column(&add, 3);
        pager.children().add(add);
        host.children().add(pager);
        self.variable_form(&host, view, variable, aliases, selection_count, &error);
    }

    fn variable_mode_controls(self: &Rc<Self>, host: &StackPanel, view: &VariableView, error: &Ref<TextBlock>) {
        let mode = view.mode.clone().unwrap();
        let row = variable_grid("*,76,76,76");
        let field = variable_field("VariablesModeName", if mode == "Default" { "New mode" } else { &mode });
        row.children().add(field.clone());
        for (column, label, control_name) in [(1, "Add mode", "VariablesAddMode"), (2, "Rename", "VariablesRenameMode"), (3, "Delete", "VariablesDeleteMode")] {
            let state = view.clone();
            let field = field.downgrade();
            let weak_error = error.downgrade();
            let old_mode = mode.clone();
            let button = self.variable_button(label, control_name, column == 1 || mode != "Default", move |shell| {
                if let (Some(field), Some(error)) = (field.upgrade(), weak_error.upgrade()) {
                    let name = field.text().unwrap_or_default().trim().to_string();
                    let edit = match column {
                        1 => VariableEdit::AddMode { name: name.clone(), copy_from: old_mode.clone() },
                        2 => VariableEdit::RenameMode { mode: old_mode.clone(), name: name.clone() },
                        _ => VariableEdit::DeleteMode { mode: old_mode.clone() },
                    };
                    if shell.commit_variable_edit(edit, &error).is_some() {
                        shell.variables_view(VariableView { mode: Some(if column == 3 { "Default".into() } else { name }), ..state.clone() });
                    }
                }
            });
            Grid::set_column(&button, column);
            row.children().add(button);
        }
        host.children().add(row);
    }

    fn variable_collection_controls(self: &Rc<Self>, host: &StackPanel, view: &VariableView, collections: &[String], error: &Ref<TextBlock>) {
        let collection = view.collection.clone().unwrap();
        let row = variable_grid("*,125,125");
        let field = variable_field("VariablesCollectionName", &collection);
        row.children().add(field.clone());
        for (column, label, control_name) in [(1, "Rename collection", "VariablesRenameCollection"), (2, "Delete collection", "VariablesDeleteCollection")] {
            let state = view.clone();
            let field = field.downgrade();
            let weak_error = error.downgrade();
            let old_collection = collection.clone();
            let button = self.variable_button(label, control_name, collections.contains(&collection), move |shell| {
                if let (Some(field), Some(error)) = (field.upgrade(), weak_error.upgrade()) {
                    let name = field.text().unwrap_or_default().trim().to_string();
                    let edit = if column == 1 {
                        VariableEdit::RenameCollection { collection: old_collection.clone(), name: name.clone() }
                    } else { VariableEdit::DeleteCollection { collection: old_collection.clone() } };
                    if shell.commit_variable_edit(edit, &error).is_some() {
                        shell.variables_view(VariableView { collection: if column == 1 { Some(name) } else { None }, variable: None, draft: false, page: 0, ..state.clone() });
                    }
                }
            });
            Grid::set_column(&button, column);
            row.children().add(button);
        }
        host.children().add(row);
    }

    fn variable_form(self: &Rc<Self>, host: &StackPanel, view: VariableView, variable: Option<core::Variable>, aliases: Vec<AliasChoice>, selection_count: usize, error: &Ref<TextBlock>) {
        let mode = view.mode.clone().unwrap();
        let original_kind = variable.as_ref().and_then(|variable| self.editor.borrow().document.variable_kind(variable.id, &mode));
        let kind = original_kind.unwrap_or(VariableKind::Color);
        let value = variable.as_ref().and_then(|variable| mode_value(variable, &mode)).cloned().unwrap_or_else(|| kind.default_value());
        let heading = variable_text(if variable.is_some() { "Edit variable" } else { "New variable" }, false);
        heading.set_font_weight(FontWeight::SemiBold);
        host.children().add(heading);
        let identity = variable_grid("*,*");
        let name = variable_field("VariableName", variable.as_ref().map(|variable| variable.name.as_str()).unwrap_or("Variable"));
        let collection = variable_field("VariableCollection", view.collection.as_deref().unwrap_or("Local"));
        identity.children().add(variable_labeled("Name", name.clone()));
        let collection_field = variable_labeled("Collection", collection.clone());
        Grid::set_column(&collection_field, 1);
        identity.children().add(collection_field);
        host.children().add(identity);
        let type_row = variable_grid("*,*");
        let types = variable_choice("VariableType", &KINDS.iter().map(|kind| kind.label().into()).collect::<Vec<_>>(), KINDS.iter().position(|value| *value == kind).unwrap());
        let source = variable_choice("VariableSource", &["Value".into(), "Alias".into()], usize::from(matches!(value, VariableValue::Alias(_))));
        type_row.children().add(variable_labeled_choice("Type", types.clone()));
        let source_field = variable_labeled_choice("Value source", source.clone());
        Grid::set_column(&source_field, 1);
        type_row.children().add(source_field);
        host.children().add(type_row);

        let value_field = variable_field("VariableValue", &primitive_value_text(&value));
        value_field.set_accepts_return(kind == VariableKind::String);
        value_field.set_text_wrapping(TextWrapping::Wrap);
        let boolean = CheckBox::new();
        boolean.set_name(Some("VariableBoolean".into()));
        boolean.set_content(Some(Rc::new("True".to_string())));
        boolean.set_is_checked(Some(matches!(value, VariableValue::Boolean(true))));
        let alias = variable_choice("VariableAlias", &[], 0);
        let alias_ids = Rc::new(RefCell::new(Vec::new()));
        let aliases = Rc::new(aliases);
        populate_aliases(&alias, &alias_ids, &aliases, kind, if let VariableValue::Alias(id) = value { Some(id) } else { None });
        host.children().add(value_field.clone());
        host.children().add(boolean.clone());
        host.children().add(alias.clone());
        update_value_visibility(&types, &source, &value_field, &boolean, &alias);
        let weak_types = types.downgrade();
        let weak_source = source.downgrade();
        let weak_value = value_field.downgrade();
        let weak_boolean = boolean.downgrade();
        let weak_alias = alias.downgrade();
        let alias_options = aliases.clone();
        let displayed_ids = alias_ids.clone();
        types.selection_changed(move |_, _| {
            if let (Some(types), Some(source), Some(value), Some(boolean), Some(alias)) = (
                weak_types.upgrade(), weak_source.upgrade(), weak_value.upgrade(), weak_boolean.upgrade(), weak_alias.upgrade(),
            ) {
                let kind = selected_kind(types.selected_index());
                value.set_text(Some(&primitive_value_text(&kind.default_value())));
                boolean.set_is_checked(Some(true));
                source.set_selected_index(0);
                populate_aliases(&alias, &displayed_ids, &alias_options, kind, None);
                update_value_visibility(&types, &source, &value, &boolean, &alias);
            }
        });
        let weak_types = types.downgrade();
        let weak_source = source.downgrade();
        let weak_value = value_field.downgrade();
        let weak_boolean = boolean.downgrade();
        let weak_alias = alias.downgrade();
        source.selection_changed(move |_, _| {
            if let (Some(types), Some(source), Some(value), Some(boolean), Some(alias)) = (
                weak_types.upgrade(), weak_source.upgrade(), weak_value.upgrade(), weak_boolean.upgrade(), weak_alias.upgrade(),
            ) { update_value_visibility(&types, &source, &value, &boolean, &alias); }
        });
        host.children().add(variable_text(
            "Value edits affect the mode shown. Changing type initializes every mode with this value. Bound properties must remain compatible.", true
        ));
        if variable.as_ref().is_some_and(|variable| !variable.values.contains_key(&mode)) {
            host.children().add(variable_text("This mode currently uses a fallback value. Save creates its own override.", true));
        }
        self.variable_binding_controls(host, &view, kind, selection_count, error);

        let state = view.clone();
        let weak_error = error.downgrade();
        let id = variable.as_ref().map(|variable| variable.id);
        let save = self.variable_button("Save variable", "VariableSave", true, move |shell| {
            let Some(error) = weak_error.upgrade() else { return; };
            let kind = selected_kind(types.selected_index());
            let alias_target = usize::try_from(alias.selected_index()).ok()
                .and_then(|index| alias_ids.borrow().get(index).copied());
            let value = parse_variable_value(kind, source.selected_index() == 1, &value_field.text().unwrap_or_default(), boolean.is_checked() == Some(true), alias_target);
            let value = match value { Ok(value) => value, Err(message) => { show_variable_error(&error, &message); return; } };
            let name = name.text().unwrap_or_default().trim().to_string();
            let collection = collection.text().unwrap_or_default().trim().to_string();
            let edit = VariableEdit::Upsert { id, name, collection: collection.clone(), mode: mode.clone(), value,
                replace_all_modes: original_kind != Some(kind) };
            if let Some(result) = shell.commit_variable_edit(edit, &error) {
                shell.variables_view(VariableView { collection: Some(collection), variable: result.variable, draft: false, ..state.clone() });
            }
        });
        save.classes().add("primary");
        let actions = self.control::<StackPanel>("ModalActions");
        if let Some(id) = variable.as_ref().map(|variable| variable.id) {
            let state = view.clone();
            let weak_error = error.downgrade();
            actions.children().add(self.variable_button("Delete variable", "VariableDelete", true, move |shell| {
                if let Some(error) = weak_error.upgrade() {
                    if shell.commit_variable_edit(VariableEdit::Delete { id }, &error).is_some() {
                        shell.variables_view(VariableView { variable: None, draft: false, ..state.clone() });
                    }
                }
            }));
        }
        actions.children().add(save);
        actions.children().add(self.variable_button("Done", "VariablesDone", true, |shell| shell.close_modal()));
    }

    fn variable_binding_controls(self: &Rc<Self>, host: &StackPanel, view: &VariableView, kind: VariableKind, selection_count: usize, error: &Ref<TextBlock>) {
        host.children().add(variable_text(&format!("Bind selected layers · {selection_count} selected"), true));
        let row = variable_grid("*,130,94");
        let properties = variable_choice("VariableBindingProperty", &BINDINGS.iter().map(|(_, label)| label.to_string()).collect::<Vec<_>>(),
            BINDINGS.iter().position(|(property, _)| core::variable_binding_kind(property) == Some(kind)).unwrap_or(0));
        row.children().add(properties.clone());
        for (column, caption, control_name) in [(1, "Bind this variable", "VariableBind"), (2, "Unbind", "VariableUnbind")] {
            let state = view.clone();
            let variable = view.variable;
            let weak_error = error.downgrade();
            let weak_properties = properties.downgrade();
            let enabled = selection_count > 0 && (column == 2 || variable.is_some());
            let button = self.variable_button(caption, control_name, enabled, move |shell| {
                if let (Some(error), Some(properties)) = (weak_error.upgrade(), weak_properties.upgrade()) {
                    let Some((property, _)) = selected_item(&BINDINGS, properties.selected_index()) else { return; };
                    let nodes = shell.editor.borrow().selection.clone();
                    let edit = VariableEdit::Bind { nodes, property: property.to_string(), variable: if column == 1 { variable } else { None } };
                    if shell.commit_variable_edit(edit, &error).is_some() { shell.variables_view(state.clone()); }
                }
            });
            Grid::set_column(&button, column);
            row.children().add(button);
        }
        host.children().add(row);
        let binding_summary = {
            let editor = self.editor.borrow();
            editor.selection.iter().filter_map(|id| editor.document.nodes.get(id)).take(8).flat_map(|node| {
                node.variable_bindings.iter().map(|(property, id)| format!("{} · {} → {}", short_value(&node.name, 22), property,
                    editor.document.variables.get(id).map(|variable| variable.name.as_str()).unwrap_or("Missing variable"))).collect::<Vec<_>>()
            }).take(12).collect::<Vec<_>>().join("\n")
        };
        if !binding_summary.is_empty() { host.children().add(variable_text(&binding_summary, true)); }
        host.children().add(variable_text("Unbind and Delete preserve the currently resolved layer appearance. Padding and gap bindings need auto layout and nonnegative numbers in every mode. Stroke bindings create a 1 px stroke when needed.", true));
    }

    fn commit_variable_edit(self: &Rc<Self>, edit: VariableEdit, error: &TextBlock) -> Option<core::VariableEditResult> {
        let result = self.editor.borrow_mut().edit_variables(edit);
        match result {
            Ok(result) => { if result.changed { self.changed(); } Some(result) }
            Err(message) => { show_variable_error(error, &message.to_string()); None }
        }
    }

    fn variable_button(self: &Rc<Self>, caption: &str, name: &str, enabled: bool, action: impl Fn(&Rc<Self>) + 'static) -> Ref<Button> {
        let button = Button::new();
        button.set_name(Some(name.into()));
        button.set_content(Some(Rc::new(caption.to_string())));
        button.set_is_enabled(enabled);
        let weak = Rc::downgrade(self);
        button.click(move |_, _| { if let Some(shell) = weak.upgrade() { if !shell.updating.get() { action(&shell); } } });
        button
    }
}

fn variable_text(value: &str, secondary: bool) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(12.0);
    text.set_text_wrapping(TextWrapping::Wrap);
    text.set_vertical_alignment(VerticalAlignment::Center);
    if secondary { text.set_foreground(Some(icons::brush("#777777"))); }
    text
}
fn variable_grid(columns: &str) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(columns).unwrap());
    grid.set_column_spacing(8.0);
    grid
}
fn variable_field(name: &str, value: &str) -> Ref<TextBox> {
    let field = TextBox::new();
    field.set_name(Some(name.into()));
    field.set_text(Some(value));
    field
}
fn variable_choice(name: &str, values: &[String], selected: usize) -> Ref<ComboBox> {
    let choice = ComboBox::new();
    choice.set_name(Some(name.into()));
    for value in values { choice.items().add(Some(Rc::new(value.clone()))); }
    if !values.is_empty() { choice.set_selected_index(selected.min(values.len() - 1) as i32); }
    choice
}
fn variable_labeled(label: &str, field: Ref<TextBox>) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(4.0);
    panel.children().add(variable_text(label, true));
    panel.children().add(field);
    panel
}
fn variable_labeled_choice(label: &str, field: Ref<ComboBox>) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(4.0);
    panel.children().add(variable_text(label, true));
    panel.children().add(field);
    panel
}
fn selected_item<T>(values: &[T], selected: i32) -> Option<&T> {
    usize::try_from(selected).ok().and_then(|selected| values.get(selected))
}
fn selected_kind(index: i32) -> VariableKind { selected_item(&KINDS, index).copied().unwrap_or(VariableKind::Color) }
fn short_value(value: &str, limit: usize) -> String {
    let mut text: String = value.chars().take(limit).map(|ch| if ch.is_control() { ' ' } else { ch }).collect();
    if value.chars().nth(limit).is_some() { text.push('…'); }
    text
}
fn primitive_value_text(value: &VariableValue) -> String {
    match value {
        VariableValue::Color(color) => color.hex(), VariableValue::Number(number) => number.to_string(),
        VariableValue::String(value) => value.clone(), VariableValue::Boolean(value) => value.to_string(),
        VariableValue::Alias(_) => String::new(),
    }
}
fn mode_value<'a>(variable: &'a core::Variable, mode: &str) -> Option<&'a VariableValue> {
    variable.values.get(mode).or_else(|| variable.values.get("Default")).or_else(|| variable.values.values().next())
}
fn variable_value_label(document: &core::Document, value: Option<&VariableValue>) -> String {
    match value {
        Some(VariableValue::Alias(id)) => format!("Alias → {}", document.variables.get(id).map(|variable| variable.name.as_str()).unwrap_or("Missing variable")),
        Some(value) => primitive_value_text(value), None => "No value".into(),
    }
}
fn show_variable_error(error: &TextBlock, message: &str) {
    error.set_text(Some(message));
    error.set_is_visible(true);
    error.bring_into_view();
}
fn populate_aliases(choice: &ComboBox, ids: &RefCell<Vec<VariableId>>, options: &[AliasChoice], kind: VariableKind, current: Option<VariableId>) {
    choice.items().clear();
    let mut ids = ids.borrow_mut();
    ids.clear();
    for option in options.iter().filter(|option| option.kind == kind) {
        choice.items().add(Some(Rc::new(option.label.clone())));
        ids.push(option.id);
    }
    let selected = current.and_then(|id| ids.iter().position(|value| *value == id)).unwrap_or(0);
    choice.set_selected_index(if ids.is_empty() { -1 } else { selected as i32 });
}
fn update_value_visibility(types: &ComboBox, source: &ComboBox, value: &TextBox, boolean: &CheckBox, alias: &ComboBox) {
    let is_alias = source.selected_index() == 1;
    let kind = selected_kind(types.selected_index());
    value.set_is_visible(!is_alias && kind != VariableKind::Boolean);
    value.set_accepts_return(kind == VariableKind::String);
    value.set_placeholder_text(Some(match kind { VariableKind::Color => "#7B61FF", VariableKind::Number => "0", VariableKind::String => "Text value", VariableKind::Boolean => "" }));
    boolean.set_is_visible(!is_alias && kind == VariableKind::Boolean);
    alias.set_is_visible(is_alias);
}
fn parse_variable_value(kind: VariableKind, alias: bool, text: &str, boolean: bool, target: Option<VariableId>) -> Result<VariableValue, String> {
    if alias { return target.map(VariableValue::Alias).ok_or_else(|| "Choose an existing variable of the same type as the alias target.".into()); }
    match kind {
        VariableKind::Color => Color::from_hex(text).map(VariableValue::Color).ok_or_else(|| "Enter a hexadecimal color, such as #7B61FF or #7B61FF80.".into()),
        VariableKind::Number => text.trim().parse::<f64>().ok().filter(|value| value.is_finite()).map(VariableValue::Number)
            .ok_or_else(|| "Enter a finite number.".into()),
        VariableKind::String => Ok(VariableValue::String(text.to_string())),
        VariableKind::Boolean => Ok(VariableValue::Boolean(boolean)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_native_variable_values_preserve_strings_and_validate_numbers_aliases_and_alpha() {
        assert_eq!(parse_variable_value(VariableKind::String, false, "  🦀\n ", false, None), Ok(VariableValue::String("  🦀\n ".into())));
        assert_eq!(parse_variable_value(VariableKind::Color, false, "#12345680", false, None), Ok(VariableValue::Color(Color::rgba(18, 52, 86, 128))));
        assert_eq!(parse_variable_value(VariableKind::Boolean, false, "", false, None), Ok(VariableValue::Boolean(false)));
        assert_eq!(parse_variable_value(VariableKind::Color, true, "", false, Some(42)), Ok(VariableValue::Alias(42)));
        assert!(parse_variable_value(VariableKind::Number, true, "0", false, None).is_err());
        for value in ["NaN", "inf", "-inf", ""] { assert!(parse_variable_value(VariableKind::Number, false, value, false, None).is_err()); }
        assert_eq!(selected_item(&[1, 2], -1), None);
    }
}
