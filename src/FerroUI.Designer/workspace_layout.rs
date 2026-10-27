//! Contextual panel editing uses the same accepted AST and registered layout adapters.
use super::*;
use crate::GridTrackEditor;
use ferroui_base::data::BindingOperations;
use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{ColumnDefinitions, Panel, ToolTip};
use ferroui_designer_tools::{
    Axis, GridTrackChange, GridTrackPlacement, LayoutAction, grid_track_values,
    plan_grid_track_change_with_placements,
};
use std::collections::BTreeMap;
use std::sync::Arc;

struct LayoutEditorModel(PropertyEditorContext);
impl PartialEq for LayoutEditorModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub(super) struct LayoutUi {
    uri: String,
    panel: Option<NodeId>,
    rows: usize,
    columns: usize,
    expanded: bool,
    editors: Vec<Rc<GridTrackEditor>>,
}
impl Default for LayoutUi {
    fn default() -> Self {
        Self {
            uri: String::new(),
            panel: None,
            rows: 0,
            columns: 0,
            expanded: true,
            editors: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct Context {
    uri: String,
    revision: u64,
    ast: Arc<ferroui_designer_core::Ast>,
    selection: Vec<NodeId>,
    panel: NodeId,
}
impl Context {
    fn check(&self, workspace: &DesignerWorkspace) -> Result<(), String> {
        let session = workspace.session.borrow();
        session
            .ensure_writable()
            .map_err(|error| error.to_string())?;
        if workspace.pending_source.get()
            || workspace.runtime_selection.get().is_some()
            || session.document_uri() != self.uri
            || session.document().revision() != self.revision
            || session.selection() != self.selection
            || !Arc::ptr_eq(&self.ast, &session.document().ast_snapshot())
        {
            return Err("The layout selection changed; use its current editor".into());
        }
        if !session.can_manipulate() {
            return Err("Layout editing requires the current preview".into());
        }
        Ok(())
    }
}

impl DesignerSession {
    /// Edits definitions and affected child cells in a single ordinary source transaction.
    /// Grid structure is not recorded as an animation, even when recording is enabled.
    pub fn edit_grid_tracks(
        &mut self,
        panel: NodeId,
        axis: Axis,
        change: GridTrackChange,
    ) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !self.can_manipulate() {
            return Err("Grid editing requires a successful current preview".into());
        }
        let grid = self
            .preview
            .visual(panel)
            .and_then(|visual| visual.cast::<Grid>())
            .ok_or("Select an authored Grid with a live preview")?;
        let (position, span, collection, count) = match axis {
            Axis::Vertical => (
                Grid::row_property().as_property(),
                Grid::row_span_property().as_property(),
                "RowDefinitions",
                grid.row_definitions().count(),
            ),
            Axis::Horizontal => (
                Grid::column_property().as_property(),
                Grid::column_span_property().as_property(),
                "ColumnDefinitions",
                grid.column_definitions().count(),
            ),
        };
        let node = self
            .document()
            .ast()
            .node(panel)
            .ok_or("The Grid no longer exists")?;
        let authored = node
            .attributes
            .iter()
            .any(|attribute| attribute.name.rsplit(':').next() == Some(collection))
            || node
                .children
                .iter()
                .filter_map(|id| self.document().ast().node(*id))
                .any(|child| child.local_name().ends_with(&format!(".{collection}")));
        if !authored && count > 0 {
            return Err("Grid definitions come from a style or application factory; edit their source before changing tracks".into());
        }
        let mut placements = BTreeMap::new();
        if !matches!(change, GridTrackChange::Set { .. }) {
            for child in grid.children().to_vec() {
                let id = self
                    .preview
                    .source_id(&child)
                    .ok_or("A generated Grid child has no authored layout; edit its source")?;
                if BindingOperations::get_binding_expression_base(&child, position).is_some()
                    || BindingOperations::get_binding_expression_base(&child, span).is_some()
                {
                    return Err("A Grid child has a bound cell or span; edit its source before changing tracks".into());
                }
                let (index, span) = match axis {
                    Axis::Vertical => (Grid::get_row(&child), Grid::get_row_span(&child)),
                    Axis::Horizontal => (Grid::get_column(&child), Grid::get_column_span(&child)),
                };
                placements.insert(
                    id,
                    GridTrackPlacement {
                        index: index.max(0) as usize,
                        span: span.max(1) as usize,
                    },
                );
            }
        }
        let edits = plan_grid_track_change_with_placements(
            self.document(),
            panel,
            axis,
            change,
            &placements,
        )
        .map_err(|error| error.to_string())?;
        if !edits.is_empty() {
            self.apply("Edit Grid tracks", edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn panel_for(session: &DesignerSession, mut node: NodeId) -> Option<NodeId> {
    loop {
        if session
            .preview
            .visual(node)
            .and_then(|visual| visual.cast::<Panel>())
            .is_some()
        {
            return Some(node);
        }
        node = session.document().ast().node(node)?.parent?;
    }
}

impl DesignerWorkspace {
    /// Current contextual editors, in row/column order when the selection belongs to a Grid.
    pub fn grid_track_editors(&self) -> Vec<Rc<GridTrackEditor>> {
        self.layout.borrow().editors.clone()
    }

    pub(super) fn rebuild_layout(self: &Rc<Self>) {
        let host = self.root.get_control::<ContentControl>("LayoutHost");
        self.layout.borrow_mut().editors.clear();
        host.set_content(None);
        host.set_is_visible(false);
        if self.runtime_selection.get().is_some() {
            return;
        }
        let session = self.session.borrow();
        let selected = session.selection();
        let Some(first) = selected.first().copied() else {
            return;
        };
        let Some(panel) = panel_for(&session, first) else {
            return;
        };
        if !selected
            .iter()
            .all(|node| panel_for(&session, *node) == Some(panel))
        {
            return;
        }
        let Some(node) = session.document().ast().node(panel) else {
            return;
        };
        let Some(adapter) = session
            .layout_tools()
            .adapter_for(session.document(), node.id)
        else {
            return;
        };
        let name = node.xaml_name().unwrap_or(node.local_name()).to_owned();
        let kind = node.local_name().to_owned();
        let properties = adapter
            .panel_properties()
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>();
        let context = Context {
            uri: session.document_uri().to_owned(),
            revision: session.document().revision(),
            ast: session.document().ast_snapshot(),
            selection: selected.to_vec(),
            panel,
        };
        let writable = session.can_manipulate()
            && !session.is_read_only()
            && !self.pending_source.get()
            && session
                .runtime_layout_capabilities(panel)
                .is_some_and(|capabilities| !capabilities.generated_items);
        let mut ui = self.layout.borrow_mut();
        if ui.uri != context.uri || ui.panel != Some(panel) {
            ui.uri = context.uri.clone();
            ui.panel = Some(panel);
            ui.rows = 0;
            ui.columns = 0;
        }
        let preferred = [ui.rows, ui.columns];
        let expanded = ui.expanded;
        drop(ui);
        let root = StackPanel::new();
        root.classes().add("designer-layout-card");
        root.set_spacing(6.0);
        let header = Grid::new();
        header.set_column_definitions(ColumnDefinitions::parse("*,Auto").unwrap());
        let has_settings = kind == "Grid" || !properties.is_empty();
        let title = if panel == first {
            format!("{kind} layout")
        } else {
            format!("Parent · {kind}")
        };
        let toggle = button("");
        toggle.set_content(Some(Control::boxed(layout_header(
            &title,
            expanded,
            has_settings,
        ))));
        toggle.classes().add("property-category");
        toggle.set_font_size(11.0);
        toggle.set_padding(Thickness::symmetric(0.0, 2.0));
        toggle.set_horizontal_content_alignment(HorizontalAlignment::Left);
        ToolTip::set_tip(
            &toggle,
            Some(Rc::new(format!("{name} · {kind} container layout"))),
        );
        header.children().add(toggle.clone().upcast::<Control>());
        if panel != first {
            let parent = button("");
            let icon = crate::property_controls::property_icon(
                "M2,8 L7,3 12,8 10.8,9.2 7.8,6.2 7.8,12 14,12 14,13.5 6.2,13.5 6.2,6.2 3.2,9.2 Z",
                13.0,
            );
            parent.set_content(Some(Control::boxed(&icon)));
            parent.classes().add("property-track-command");
            parent.set_font_size(10.0);
            parent.set_width(26.0);
            parent.set_padding(Thickness::symmetric(4.0, 2.0));
            ferroui_controls::automation::AutomationProperties::set_name(
                &parent,
                Some("Select parent container"),
            );
            Grid::set_column(&parent, 1);
            ToolTip::set_tip(
                &parent,
                Some(Rc::new(format!(
                    "Select the {kind} that controls this layout"
                ))),
            );
            let weak = Rc::downgrade(self);
            let captured = context.clone();
            parent.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let session = this.session.borrow();
                    let current = session.document_uri() == captured.uri
                        && Arc::ptr_eq(&captured.ast, &session.document().ast_snapshot());
                    drop(session);
                    if current {
                        this.session.borrow_mut().select([panel]);
                        this.refresh(true);
                    }
                }
            });
            header.children().add(parent.upcast::<Control>());
        }
        root.children().add(header.upcast::<Control>());
        let body = StackPanel::new();
        body.set_spacing(5.0);
        body.set_is_visible(expanded);
        let weak_body = body.downgrade();
        let weak = Rc::downgrade(self);
        let weak_toggle = toggle.downgrade();
        toggle.click(move |_, _| {
            if !has_settings {
                return;
            }
            if let Some((this, body)) = weak.upgrade().zip(weak_body.upgrade()) {
                let expanded = !this.layout.borrow().expanded;
                this.layout.borrow_mut().expanded = expanded;
                body.set_is_visible(expanded);
                if let Some(toggle) = weak_toggle.upgrade() {
                    toggle.set_content(Some(Control::boxed(layout_header(
                        &title,
                        expanded,
                        has_settings,
                    ))));
                }
            }
        });
        if kind == "Grid" {
            for (axis, preferred) in [Axis::Vertical, Axis::Horizontal]
                .into_iter()
                .zip(preferred)
            {
                match grid_track_values(session.document(), panel, axis) {
                    Ok(values) => {
                        let editor = GridTrackEditor::new(axis);
                        editor.configure(values, preferred, writable);
                        let weak = Rc::downgrade(self);
                        let selected_context = context.clone();
                        editor.on_selected(move |index| {
                            if let Some(this) = weak.upgrade() {
                                let session = this.session.borrow();
                                if session.document_uri() != selected_context.uri
                                    || !Arc::ptr_eq(
                                        &selected_context.ast,
                                        &session.document().ast_snapshot(),
                                    )
                                    || session.selection() != selected_context.selection
                                {
                                    return;
                                }
                                drop(session);
                                let mut ui = this.layout.borrow_mut();
                                match axis {
                                    Axis::Vertical => ui.rows = index,
                                    Axis::Horizontal => ui.columns = index,
                                }
                            }
                        });
                        let captured = context.clone();
                        let weak = Rc::downgrade(self);
                        let weak_editor = Rc::downgrade(&editor);
                        editor.on_change(move |change| {
                            let this = weak.upgrade().ok_or("The workspace was closed")?;
                            captured.check(&this)?;
                            let focused = weak_editor.upgrade().and_then(|editor| {
                                let value = editor.length_editor();
                                if value.is_keyboard_focus_within() {
                                    Some((0, value.selection_start(), value.selection_end()))
                                } else if editor.unit_selector().is_keyboard_focus_within() {
                                    Some((1, 0, 0))
                                } else {
                                    None
                                }
                            });
                            let preferred = match &change {
                                GridTrackChange::Set { index, .. }
                                | GridTrackChange::Insert { index, .. } => *index,
                                GridTrackChange::Remove { index } => index.saturating_sub(1),
                            };
                            this.session.borrow_mut().edit_grid_tracks(
                                captured.panel,
                                axis,
                                change,
                            )?;
                            {
                                let mut ui = this.layout.borrow_mut();
                                match axis {
                                    Axis::Vertical => ui.rows = preferred,
                                    Axis::Horizontal => ui.columns = preferred,
                                }
                            }
                            this.refresh(true);
                            if let Some((field, start, end)) = focused {
                                let accepted = this.session.borrow().document().ast_snapshot();
                                let selected = this.session.borrow().selection().to_vec();
                                let weak = Rc::downgrade(&this);
                                Dispatcher::ui_thread().post_local(
                                    move || {
                                        let Some(this) = weak.upgrade() else {
                                            return;
                                        };
                                        let session = this.session.borrow();
                                        if !Arc::ptr_eq(
                                            &accepted,
                                            &session.document().ast_snapshot(),
                                        ) || selected != session.selection()
                                        {
                                            return;
                                        }
                                        drop(session);
                                        this.root.update_layout();
                                        let editor = this
                                            .grid_track_editors()
                                            .into_iter()
                                            .find(|editor| editor.axis() == axis);
                                        if let Some(editor) = editor {
                                            if field == 0 {
                                                let value = editor.length_editor();
                                                value.focus();
                                                value.set_selection_start(start);
                                                value.set_selection_end(end);
                                            } else {
                                                editor.unit_selector().focus();
                                            }
                                        }
                                    },
                                    DispatcherPriority::LOADED,
                                );
                            }
                            Ok(())
                        });
                        body.children().add(editor.control());
                        self.layout.borrow_mut().editors.push(editor);
                    }
                    Err(error) => {
                        let text = TextBlock::new();
                        text.set_text(Some(&error.to_string()));
                        text.set_font_size(10.0);
                        text.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
                        body.children().add(text.upcast::<Control>());
                    }
                }
            }
        } else {
            let preferred: &[&str] = match kind.as_str() {
                "StackPanel" | "ReversibleStackPanel" => &["Orientation", "Spacing"],
                "WrapPanel" => &["Orientation", "ItemSpacing"],
                "DockPanel" => &["LastChildFill"],
                "UniformGrid" => &["Rows", "Columns"],
                "FlexPanel" => &["Direction", "Wrap"],
                _ => &[],
            };
            let quick = if preferred.is_empty() {
                properties
                    .iter()
                    .take(2)
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            } else {
                preferred.to_vec()
            };
            for property in quick
                .into_iter()
                .filter(|property| properties.iter().any(|candidate| candidate == property))
            {
                let row = Grid::new();
                row.set_column_definitions(ColumnDefinitions::parse("84,*").unwrap());
                let label = TextBlock::new();
                label.set_text(Some(match property {
                    "Spacing" | "ItemSpacing" => "Gap",
                    "LastChildFill" => "Fill last child",
                    _ => property,
                }));
                label.set_font_size(11.0);
                label.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
                label.set_vertical_alignment(VerticalAlignment::Center);
                row.children().add(label.upcast::<Control>());
                let visual = session.preview.visual(panel).unwrap();
                let runtime = Inspector::property(&visual, property).ok();
                let effective = runtime.map(|property| {
                    ferroui_inspect::display_value(Some(&visual.get_value_untyped(property)))
                });
                let value = node.attribute(property).map(str::to_owned).or(effective);
                let captured = context.clone();
                let weak = Rc::downgrade(self);
                let property_name = property.to_owned();
                let editor_context = PropertyEditorContext {
                    property: property.to_owned(),
                    targets: vec![panel],
                    value: value.clone(),
                    mixed: false,
                    runtime_type: runtime.map(|property| property.property_type_name().to_owned()),
                    apply: Rc::new(move |value| {
                        let Some(this) = weak.upgrade() else {
                            return;
                        };
                        let result = captured.check(&this).and_then(|_| {
                            let value = value
                                .ok_or("Reset this property through the property inspector")?;
                            this.session.borrow_mut().apply_layout_action(
                                LayoutAction::PanelProperty {
                                    panel: captured.panel,
                                    name: property_name.clone(),
                                    value,
                                },
                                "Edit panel layout",
                            )
                        });
                        if result.is_ok() {
                            this.refresh(true);
                        }
                        this.report(result);
                    }),
                };
                // Match the property grid's realization contract: factories run only when
                // the native content presenter realizes this contextual editor.
                let editor = ContentControl::new();
                let factories = self.property_editors.borrow().clone();
                let enabled =
                    writable && !value.as_ref().is_some_and(|value| value.starts_with('{'));
                editor.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
                editor.set_content_template(Some(FuncDataTemplate::for_type::<LayoutEditorModel>(
                    move |model, _| {
                        let context = &model.0;
                        let control = factories
                            .iter()
                            .find_map(|factory| factory.create(context))
                            .unwrap_or_else(|| crate::editors::built_in(context));
                        control.set_is_enabled(enabled);
                        Some(control)
                    },
                    false,
                )));
                editor.set_content(Some(Rc::new(LayoutEditorModel(editor_context))));
                Grid::set_column(&editor, 1);
                row.children().add(editor.upcast::<Control>());
                body.children().add(row.upcast::<Control>());
            }
        }
        drop(session);
        root.children().add(body.upcast::<Control>());
        let frame = ferroui_controls::Border::new();
        frame.classes().add("designer-layout-frame");
        frame.set_border_brush(Some(SolidColorBrush::from_uint32(0xff3b3d46).into()));
        frame.set_border_thickness(Thickness::uniform(1.0));
        frame.set_corner_radius(ferroui_base::CornerRadius::uniform(5.0));
        frame.set_padding(Thickness::new(
            8.0,
            3.0,
            8.0,
            if expanded && has_settings { 8.0 } else { 3.0 },
        ));
        frame.set_child(Some(root.upcast::<Control>()));
        host.set_content(Some(Control::boxed(&frame)));
        host.set_is_visible(true);
    }
}

fn layout_header(title: &str, expanded: bool, expandable: bool) -> Ref<Control> {
    let content = Grid::new();
    content.set_column_definitions(ColumnDefinitions::parse("16,*").unwrap());
    let icon = crate::property_controls::property_icon(
        if expandable {
            if expanded {
                crate::property_controls::CHEVRON_DOWN_ICON
            } else {
                crate::property_controls::CHEVRON_RIGHT_ICON
            }
        } else {
            "M1,1 L15,1 15,15 1,15 Z M2.5,2.5 L2.5,13.5 13.5,13.5 13.5,2.5 Z"
        },
        if expandable { 8.0 } else { 11.0 },
    );
    content.children().add(icon);
    let label = TextBlock::new();
    label.set_text(Some(title));
    label.set_font_size(11.0);
    label.set_font_weight(ferroui_base::media::FontWeight::SemiBold);
    label.set_vertical_alignment(VerticalAlignment::Center);
    Grid::set_column(&label, 1);
    content.children().add(label.upcast::<Control>());
    content.upcast::<Control>()
}

#[cfg(test)]
#[path = "workspace_layout_tests.rs"]
mod tests;
