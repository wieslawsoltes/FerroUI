//! Studio inspector for the same checked column actions used by VS Code and MCP.
use super::DesignerWorkspace;
use super::inspector_actions::{COPY, RESET, SOURCE, add_button, icon_button, picker, text};
use crate::icons;
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::WeakRef;
use ferroui_controls::{
    Button, ColumnDefinitions, ContentControl, Control, Flyout, Grid, PlacementMode,
    ScrollViewer, StackPanel, TextBlock,
};
use ferroui_designer_core::{NodeId, SourceSpan};
use ferroui_designer_tools::{AuthoredColumn, ColumnChange, TableColumns, nearest_table};
use std::rc::{Rc, Weak};

#[path = "workspace_column_fields.rs"]
mod fields;
#[cfg(test)]
#[path = "workspace_columns_tests.rs"]
mod tests;

impl DesignerWorkspace {
    pub(super) fn connect_columns(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("EditColumns");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Left);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let weak = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                let Some(flyout) = popup.upgrade() else {
                    return;
                };
                let scope = ColumnFlyout {
                    workspace: weak.clone(),
                    flyout: popup.clone(),
                };
                scope.render(&flyout, 0);
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }

    pub(super) fn refresh_columns_command(&self) {
        let session = self.session.borrow();
        let visible = match session.selection() {
            [node] => nearest_table(session.document(), *node).is_some(),
            _ => false,
        };
        self.root
            .get_control::<Button>("EditColumns")
            .set_is_visible(visible);
    }
}

#[derive(Clone)]
struct ColumnFlyout {
    workspace: Weak<DesignerWorkspace>,
    flyout: WeakRef<Flyout>,
}

#[derive(Clone)]
struct ColumnScope {
    ui: ColumnFlyout,
    generation: u64,
    table: NodeId,
    selected: usize,
    writable: bool,
    rust: bool,
    message: WeakRef<TextBlock>,
}

impl ColumnFlyout {
    fn render(&self, flyout: &Flyout, preferred: usize) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let body = StackPanel::new();
        body.set_width(420.0);
        body.set_spacing(7.0);
        flyout.set_content(Some(Control::boxed(body.clone())));
        let session = workspace.session.borrow();
        let model = match session.selection() {
            [node] if !workspace.pending_source.get() => session.column_authoring(*node, None),
            _ => Err("Select one table in the current preview.".into()),
        };
        let Ok(Some(columns)) = model else {
            let message = model
                .err()
                .unwrap_or_else(|| "Select a TableView or TreeDataGrid.".into());
            body.children().add(text(&message).upcast::<Control>());
            return;
        };
        let model = columns.model;
        let title = text(model.name.as_deref().unwrap_or(match model.kind {
            ferroui_designer_tools::TableKind::TableView => "TableView",
            ferroui_designer_tools::TableKind::TreeDataGrid => "TreeDataGrid",
        }));
        title.set_font_weight(ferroui_base::media::FontWeight::SemiBold);
        body.children().add(title.upcast::<Control>());
        let message = text(model.reason.as_deref().unwrap_or(""));
        message.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        message.set_is_visible(model.reason.is_some());
        body.children().add(message.clone().upcast::<Control>());
        let scope = ColumnScope {
            ui: self.clone(),
            generation: session.generation(),
            table: model.table,
            selected: preferred.min(model.columns.len().saturating_sub(1)),
            writable: model.reason.is_none(),
            rust: session.uses_rust_source(),
            message: message.downgrade(),
        };
        drop(session);
        if let Some(source) = columns.source.filter(|_| !scope.writable) {
            let scope = scope.clone();
            add_button(&body, SOURCE, "Reveal column collection in source", move || scope.reveal(source));
        }
        column_list(&body, Rc::new(model), scope);
    }
}

impl ColumnScope {
    fn current(&self) -> Result<Rc<DesignerWorkspace>, String> {
        let workspace = self.ui.workspace.upgrade().ok_or("The workspace closed.")?;
        if workspace.pending_source.get()
            || workspace.session.borrow().generation() != self.generation
        {
            return Err("The document or selection changed. Reopen Columns.".into());
        }
        Ok(workspace)
    }

    fn apply(&self, change: ColumnChange, selected: usize) {
        let result = self.current().and_then(|workspace| {
            workspace
                .session
                .borrow_mut()
                .edit_authored_columns(self.table, change)?;
            workspace.refresh(true);
            if let Some(flyout) = self.ui.flyout.upgrade() {
                self.ui.render(&flyout, selected);
            }
            Ok(())
        });
        self.show_error(result);
    }

    fn reveal(&self, span: SourceSpan) {
        let result = self.current().map(|workspace| {
            if let Some(flyout) = self.ui.flyout.upgrade() {
                flyout.hide();
            }
            workspace.select_source_span(span);
        });
        self.show_error(result);
    }

    fn show_error(&self, result: Result<(), String>) {
        if let (Err(error), Some(message)) = (result, self.message.upgrade()) {
            message.set_text(Some(&error));
            message.set_is_visible(true);
        }
    }

    fn open_template(&self, column: NodeId) {
        let result = self.current().and_then(|workspace| {
            let mut session = workspace.session.borrow_mut();
            if session.rust_item_templates(column).is_empty() {
                return Err("Choose a mapped Rust table column".into());
            }
            session.select([column]);
            drop(session);
            if let Some(flyout) = self.ui.flyout.upgrade() {
                flyout.hide();
            }
            workspace.refresh(true);
            let anchor = workspace.root.get_control::<Button>("EditItemTemplates");
            let flyout = anchor.flyout().and_then(|flyout| flyout.cast::<Flyout>())
                .ok_or("The item template editor is unavailable")?;
            flyout.show_at(&anchor);
            Ok(())
        });
        self.show_error(result);
    }
}

fn column_list(body: &StackPanel, model: Rc<TableColumns>, scope: ColumnScope) {
    let details = ContentControl::new();
    details.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
    if scope.writable && !model.available.is_empty() {
        let bar = Grid::new();
        bar.set_column_definitions(ColumnDefinitions::parse("*,Auto").expect("column add bar"));
        let picker = picker(
            "New column type",
            model.available.iter().map(|item| item.label),
            0,
        );
        bar.children().add(picker.clone().upcast::<Control>());
        let add_scope = scope.clone();
        let choices = model.available.clone();
        let count = model.columns.len();
        let add = icon_button(icons::ADD, "Add column", move || {
            if let Some(choice) = choices.get(picker.selected_index().max(0) as usize) {
                add_scope.apply(
                    ColumnChange::Insert {
                        index: count,
                        kind: choice.kind,
                        header: None,
                    },
                    count,
                );
            }
        });
        Grid::set_column(&add, 1);
        bar.children().add(add.upcast::<Control>());
        body.children().add(bar.upcast::<Control>());
    }
    if model.columns.is_empty() {
        if scope.writable {
            body.children()
                .add(text("Add a column to this table.").upcast::<Control>());
        }
        return;
    }
    let labels = model
        .columns
        .iter()
        .map(|column| {
            if column.label == column.kind.label() {
                column.label.clone()
            } else {
                format!("{} · {}", column.label, column.kind.label())
            }
        })
        .collect::<Vec<_>>();
    let select = picker("Column", labels.iter().map(String::as_str), scope.selected);
    let weak_select = select.downgrade();
    let weak_details = details.downgrade();
    let selected_scope = scope.clone();
    let selected_model = model.clone();
    select.selection_changed(move |_, _| {
        if let (Some(select), Some(details)) = (weak_select.upgrade(), weak_details.upgrade()) {
            let mut scope = selected_scope.clone();
            scope.selected = select.selected_index().max(0) as usize;
            column_details(&details, &selected_model, scope);
        }
    });
    body.children().add(select.upcast::<Control>());
    column_details(&details, &model, scope);
    body.children().add(details.upcast::<Control>());
}

fn column_details(host: &ContentControl, model: &TableColumns, scope: ColumnScope) {
    let Some(column) = model.columns.get(scope.selected) else {
        return;
    };
    let content = StackPanel::new();
    content.set_spacing(7.0);
    let actions = StackPanel::new();
    actions.set_spacing(2.0);
    actions.set_orientation(Orientation::Horizontal);
    let source = scope.clone();
    let span = column.source;
    add_button(&actions, SOURCE, "Reveal column in source", move || {
        source.reveal(span)
    });
    if scope.rust {
        let template = scope.clone();
        let node = column.node;
        add_button(&actions, icons::FIT, "Edit cell template", move || template.open_template(node));
    }
    if scope.writable {
        add_reorder_buttons(&actions, column, model.columns.len(), &scope);
    }
    content.children().add(actions.upcast::<Control>());
    let editor = StackPanel::new();
    editor.set_spacing(3.0);
    fields::add_fields(&editor, column, &scope);
    let scroll = ScrollViewer::new();
    scroll.set_max_height(292.0);
    scroll.set_content(Some(Control::boxed(editor)));
    content.children().add(scroll.upcast::<Control>());
    host.set_content(Some(Control::boxed(content)));
}

fn add_reorder_buttons(
    panel: &StackPanel,
    column: &AuthoredColumn,
    count: usize,
    scope: &ColumnScope,
) {
    let id = column.node;
    let current = scope.selected;
    for (index, icon, label) in [
        (
            current.checked_sub(1),
            icons::MOVE_UP,
            "Move column earlier",
        ),
        (
            (current + 1 < count).then_some(current + 1),
            icons::MOVE_DOWN,
            "Move column later",
        ),
    ] {
        let scope = scope.clone();
        let button = icon_button(icon, label, move || {
            if let Some(index) = index {
                scope.apply(ColumnChange::Move { column: id, index }, index);
            }
        });
        button.set_is_enabled(index.is_some());
        panel.children().add(button.upcast::<Control>());
    }
    if column.can_duplicate {
        let duplicate = scope.clone();
        add_button(panel, COPY, "Duplicate column", move || {
            duplicate.apply(ColumnChange::Duplicate { column: id }, current + 1);
        });
    }
    let remove = scope.clone();
    add_button(panel, icons::REMOVE, "Remove column", move || {
        remove.apply(ColumnChange::Remove { column: id }, current);
    });
}
