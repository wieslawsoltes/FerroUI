//! Virtualized inspectors shared by the source/runtime outline and solution explorer.
//! Row models contain data and semantic callbacks; controls are created only for realized cells.
use super::*;
use ferroui_base::data::BindingOperations;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::{BoxedValue, FerroObject};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{ColumnDefinitions, GridLength, GridUnitType, ToolTip};
use ferroui_controls_treedatagrid::{
    HierarchicalTreeDataGridSourceExtensions, TreeDataGrid, TreeDataGridSourceExtensions,
    TreeDataGridSourceRef,
};
use ferroui_treedatagrid_core::selection::IReadOnlyList;
use ferroui_treedatagrid_core::{
    Accessor, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, IndexPath, Models,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub(super) struct TreeEntry {
    pub key: String,
    pub parent: Option<String>,
    pub label: String,
    pub name: String,
    pub detail: String,
    pub source: Option<NodeId>,
    pub runtime: Option<u64>,
    pub project: Option<usize>,
    pub uri: Option<String>,
    pub initially_expanded: Option<bool>,
}

pub(super) struct TreeRow {
    pub entry: TreeEntry,
    children: Rc<Vec<Rc<TreeRow>>>,
    expanded: Cell<bool>,
}

impl PartialEq for TreeRow {
    fn eq(&self, other: &Self) -> bool {
        // Rows are immutable snapshots. Replacing one with the same authored identity must
        // still rebind cells when its label, name or runtime mapping changed.
        std::ptr::eq(self, other)
    }
}

pub(super) struct HierarchyGrid {
    pub control: Ref<TreeDataGrid>,
    pub source: Rc<HierarchicalTreeDataGridSource<TreeRow>>,
    paths: RefCell<Vec<(Rc<TreeRow>, IndexPath)>>,
    expansion: RefCell<BTreeMap<String, bool>>,
    filtering: Cell<bool>,
}

impl HierarchyGrid {
    fn new(name: &str, project: bool) -> Self {
        let source = HierarchicalTreeDataGridSource::new(Rc::new(Vec::<Rc<TreeRow>>::new()))
            .with_hierarchical_expander_text_column(
                heading(if project { "Project / file" } else { "Element" }),
                Accessor::member("Label", |row: &Rc<TreeRow>| row.entry.label.clone()),
                |row| Some(row.children.clone() as Models<TreeRow>),
                |options| {
                    options.width = GridLength::new(1.0, GridUnitType::Star);
                    options.min_width = GridLength::new(105.0, GridUnitType::Pixel);
                    options.can_user_sort_column = Some(false);
                    options.has_children =
                        Some(Accessor::member("HasChildren", |row: &Rc<TreeRow>| {
                            !row.children.is_empty()
                        }));
                    options.is_expanded = Some(
                        Accessor::member("IsExpanded", |row: &Rc<TreeRow>| row.expanded.get())
                            .with_setter(|row, value| row.expanded.set(value)),
                    );
                },
            )
            .with_text_column(
                heading(if project { "Kind" } else { "Name" }),
                Accessor::member("Name", |row: &Rc<TreeRow>| row.entry.name.clone()),
                |options| {
                    options.width =
                        GridLength::new(if project { 70.0 } else { 85.0 }, GridUnitType::Pixel);
                    options.is_read_only = true;
                    options.can_user_sort_column = Some(false);
                },
            );
        source
            .row_selection()
            .unwrap()
            .base()
            .set_single_select(project);
        let control = make_grid(name);
        control.set_source(Some(TreeDataGridSourceRef::of::<TreeRow, _>(
            source.clone(),
        )));
        control.row_prepared(|event| {
            let model = event.row().model();
            if let Some(row) = model
                .as_ref()
                .and_then(|model| model.downcast_ref::<TreeRow>())
            {
                ToolTip::set_tip(
                    event.row(),
                    Some(Rc::new(format!(
                        "{} {}\n{}",
                        row.entry.label,
                        row.entry.name,
                        row.entry.uri.as_deref().unwrap_or(&row.entry.detail),
                    ))),
                );
            }
        });
        Self {
            control,
            source,
            paths: RefCell::new(Vec::new()),
            expansion: RefCell::new(BTreeMap::new()),
            filtering: Cell::new(false),
        }
    }

    /// The input is in parent-before-child order. Reverse construction avoids recursive walks
    /// and retains matching ancestors without repeated descendant scans.
    pub fn replace(&self, entries: Vec<TreeEntry>, filter: &str) {
        let mut expansion = self.expansion.borrow_mut();
        if !self.filtering.get() {
            for (row, _) in self.paths.borrow().iter() {
                expansion.insert(row.entry.key.clone(), row.expanded.get());
            }
        }
        let indexes = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.key.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        expansion.retain(|key, _| indexes.contains_key(key.as_str()));
        self.filtering.set(!filter.is_empty());
        let parents = entries
            .iter()
            .map(|entry| {
                entry
                    .parent
                    .as_deref()
                    .and_then(|key| indexes.get(key).copied())
            })
            .collect::<Vec<_>>();
        let mut children = vec![Vec::new(); entries.len()];
        let mut visible = entries
            .iter()
            .map(|entry| {
                filter.is_empty()
                    || entry.label.to_lowercase().contains(filter)
                    || entry.name.to_lowercase().contains(filter)
                    || entry.detail.to_lowercase().contains(filter)
            })
            .collect::<Vec<_>>();
        for index in (0..entries.len()).rev() {
            if let Some(parent) = parents[index] {
                debug_assert!(parent < index, "tree parents precede their children");
                if visible[index] {
                    visible[parent] = true;
                }
                children[parent].push(index);
            }
        }
        // Child lists were filled in reverse source order.
        for children in &mut children {
            children.reverse();
        }
        let mut rows = vec![None; entries.len()];
        for (index, entry) in entries.into_iter().enumerate().rev() {
            if !visible[index] {
                continue;
            }
            let expanded = if filter.is_empty() {
                expansion
                    .get(&entry.key)
                    .copied()
                    .unwrap_or(entry.initially_expanded.unwrap_or(true))
            } else {
                true
            };
            let descendants = children[index]
                .iter()
                .filter_map(|child| rows[*child].clone())
                .collect();
            rows[index] = Some(Rc::new(TreeRow {
                entry,
                children: Rc::new(descendants),
                expanded: Cell::new(expanded),
            }));
        }
        drop(expansion);
        let roots = rows
            .into_iter()
            .enumerate()
            .filter_map(|(index, row)| parents[index].is_none().then_some(row).flatten())
            .collect::<Vec<_>>();
        let mut pending = roots
            .iter()
            .enumerate()
            .rev()
            .map(|(index, row)| (row.clone(), IndexPath::new(index as i32)))
            .collect::<Vec<_>>();
        let mut paths = Vec::new();
        while let Some((row, path)) = pending.pop() {
            pending.extend(
                row.children
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(index, child)| (child.clone(), path.append(index as i32))),
            );
            paths.push((row, path));
        }
        *self.paths.borrow_mut() = paths;
        self.source.set_items(Rc::new(roots));
    }

    fn selected(&self) -> Vec<Rc<TreeRow>> {
        self.source
            .row_selection()
            .unwrap()
            .base()
            .selected_items()
            .iter()
            .flatten()
            .collect()
    }

    fn select(&self, predicate: impl Fn(&TreeEntry) -> bool) {
        let selection = self.source.row_selection().unwrap();
        let selection = selection.base();
        let paths = self.paths.borrow();
        let wanted = paths
            .iter()
            .filter(|(row, _)| predicate(&row.entry))
            .map(|(_, path)| path.clone())
            .collect::<Vec<_>>();
        if selection.selected_indexes().to_vec() == wanted {
            return;
        }
        selection.begin_batch_update();
        selection.clear();
        for path in wanted {
            let mut parent = path.parent();
            while !parent.is_empty() {
                self.source.expand(&parent);
                parent = parent.parent();
            }
            selection.select(&path);
        }
        selection.end_batch_update();
    }
}

pub(super) struct PropertyRow {
    pub name: String,
    pub display_value: String,
    pub effective: String,
    pub origin: String,
    pub runtime_type: String,
    pub context: Option<PropertyEditorContext>,
    pub factories: Rc<Vec<Rc<dyn PropertyEditorFactory>>>,
    pub can_reset: bool,
    pub reveal: Option<Rc<dyn Fn()>>,
    // Resolve on hover so animations can update diagnostics without replacing an editor
    // or changing the authored value while the user is typing.
    tooltip: Rc<dyn Fn() -> Option<String>>,
}

impl PartialEq for PropertyRow {
    fn eq(&self, other: &Self) -> bool {
        // A new revision must replace the editor callback as well as its displayed value.
        std::ptr::eq(self, other)
    }
}

#[derive(Clone)]
struct OutlineSnapshot {
    uri: String,
    revision: u64,
    ast: Arc<ferroui_designer_core::Ast>,
    generation: u64,
    mode: OutlineMode,
    filter: String,
}

impl OutlineSnapshot {
    fn matches_document(&self, session: &DesignerSession) -> bool {
        self.uri == session.document_uri()
            && self.revision == session.document().revision()
            && std::ptr::eq(self.ast.as_ref(), session.document().ast())
    }
}

impl PartialEq for OutlineSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.uri == other.uri
            && self.revision == other.revision
            && Arc::ptr_eq(&self.ast, &other.ast)
            && self.generation == other.generation
            && self.mode == other.mode
            && self.filter == other.filter
    }
}

impl Eq for OutlineSnapshot {}

pub(super) struct InspectorControls {
    pub outline: HierarchyGrid,
    pub project: HierarchyGrid,
    pub properties: Ref<TreeDataGrid>,
    pub property_source: Rc<FlatTreeDataGridSource<PropertyRow>>,
    guard: Cell<bool>,
    outline_snapshot: RefCell<Option<OutlineSnapshot>>,
}

impl InspectorControls {
    pub(super) fn source_outline_is_current(&self, session: &DesignerSession) -> bool {
        self.outline_snapshot
            .borrow()
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.mode == OutlineMode::Source && snapshot.matches_document(session)
            })
    }

    pub fn new(root: &Ref<Control>) -> Self {
        let outline = HierarchyGrid::new("Objects and document hierarchy", false);
        let project = HierarchyGrid::new("Solution explorer", true);
        let property_source = FlatTreeDataGridSource::new(Rc::new(Vec::<Rc<PropertyRow>>::new()))
            .with_text_column(
                heading("Property"),
                Accessor::member("Property", |row: &Rc<PropertyRow>| row.name.clone()),
                |options| {
                    options.width = GridLength::new(115.0, GridUnitType::Pixel);
                    options.is_read_only = true;
                    options.can_user_sort_column = Some(false);
                },
            )
            .with_template_column(
                heading("Value"),
                FuncDataTemplate::for_type::<PropertyRow>(|row, _| Some(property_cell(row)), false),
                None,
                |options| {
                    options.width = GridLength::new(1.0, GridUnitType::Star);
                    options.min_width = GridLength::new(125.0, GridUnitType::Pixel);
                    options.can_user_sort_column = Some(false);
                },
            );
        let properties = make_grid("Property inspector");
        properties.classes().add("property-inspector");
        properties.set_source(Some(TreeDataGridSourceRef::of::<PropertyRow, _>(
            property_source.clone(),
        )));
        for (name, grid) in [
            ("OutlineHost", &outline.control),
            ("PropertyHost", &properties),
            ("ProjectTreeHost", &project.control),
        ] {
            root.get_control::<ContentControl>(name)
                .set_content(Some(Control::boxed(grid)));
        }
        Self {
            outline,
            project,
            properties,
            property_source,
            guard: Cell::new(false),
            outline_snapshot: RefCell::new(None),
        }
    }

    pub fn replace_project(&self, rows: Vec<TreeEntry>) {
        let previous = self.guard.replace(true);
        let selected = self
            .project
            .selected()
            .first()
            .map(|row| row.entry.key.clone());
        self.project.replace(rows, "");
        self.project
            .select(|entry| Some(&entry.key) == selected.as_ref());
        self.guard.set(previous);
    }
}

fn heading(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_owned()))
}

fn make_grid(name: &str) -> Ref<TreeDataGrid> {
    let grid = TreeDataGrid::new();
    grid.classes().add("designer-inspector");
    // Designer chrome owns its palette. A workspace-level override would also recolor
    // authored TreeDataGrids on the artboard, bypassing their Light/Dark theme resources.
    for (key, color) in [
        ("TreeDataGridGridLinesBrush", 0xff343434),
        ("TreeDataGridSelectedCellBackgroundBrush", 0xff35474e),
        ("TreeDataGridHeaderBackgroundPointerOverBrush", 0xff373737),
        ("TreeDataGridHeaderBackgroundPressedBrush", 0xff404040),
        ("TreeDataGridHeaderBorderBrushPointerOverBrush", 0x00ffffff),
        ("TreeDataGridHeaderBorderBrushPressedBrush", 0x00ffffff),
        ("TreeDataGridHeaderForegroundPointerOverBrush", 0xfff5f5f5),
        ("TreeDataGridHeaderForegroundPressedBrush", 0xfff5f5f5),
    ] {
        grid.resources().add_value(
            key,
            SolidColorBrush::from_uint32(color).upcast::<FerroObject>(),
        );
    }
    grid.set_horizontal_alignment(HorizontalAlignment::Stretch);
    grid.set_vertical_alignment(VerticalAlignment::Stretch);
    AutomationProperties::set_name(&grid, Some(name));
    grid
}

fn property_cell(row: &PropertyRow) -> Ref<Control> {
    let cell = Grid::new();
    cell.set_column_definitions(ColumnDefinitions::parse("*,Auto").unwrap());
    cell.set_min_height(30.0);
    ToolTip::set_tip(
        &cell,
        Some(Rc::new(format!(
            "{} · {}\nAuthored value: {}\nEffective value: {}",
            row.name, row.runtime_type, row.origin, row.effective,
        ))),
    );
    ToolTip::add_tool_tip_opening_handler(&cell, {
        let weak_cell = cell.downgrade();
        let tooltip = row.tooltip.clone();
        move |_, args| {
            if let Some((cell, text)) = weak_cell.upgrade().zip(tooltip()) {
                ToolTip::set_tip(&cell, Some(Rc::new(text)));
            } else {
                args.set_cancel(true);
            }
        }
    });
    if let Some(context) = &row.context {
        let editor = if let Some(reveal) = &row.reveal {
            property_source_link("Edit XAML…", reveal)
        } else {
            row.factories
                .iter()
                .find_map(|factory| factory.create(context))
                .unwrap_or_else(|| crate::editors::built_in(context))
        };
        if context.value.is_none() && !context.mixed {
            if let Some(text) = editor.cast::<TextBox>() {
                text.set_placeholder_text(Some(&row.effective));
            }
            if let Some(number) = editor.cast::<ferroui_controls::NumericUpDown>() {
                if let Ok(effective) = ferroui_base::utilities::Decimal::parse(&row.effective) {
                    // Editing/spinning starts at the effective inherited value. Setting this
                    // initial control value does not author a property; only commit does.
                    number.set_numeric_value(Some(effective));
                } else {
                    number.set_placeholder_text(Some(if row.effective == "NaN" {
                        "Auto"
                    } else {
                        &row.effective
                    }));
                }
            }
        }
        editor.set_min_height(28.0);
        editor.set_vertical_alignment(VerticalAlignment::Center);
        editor.classes().add("property-editor");
        AutomationProperties::set_name(&editor, Some(&format!("{} value", row.name)));
        cell.children().add(editor);
        let reset = button("↺");
        reset.classes().add("property-reset");
        reset.set_width(26.0);
        reset.set_min_width(0.0);
        reset.set_padding(Thickness::new(2.0, 2.0, 2.0, 2.0));
        reset.set_is_enabled(row.can_reset);
        AutomationProperties::set_name(
            &reset,
            Some(&format!("Reset {} to inherited value", row.name)),
        );
        ToolTip::set_tip(&reset, Some(Rc::new("Reset to inherited value".to_owned())));
        Grid::set_column(&reset, 1);
        let apply = context.apply.clone();
        reset.click(move |_, _| apply(None));
        cell.children().add(reset.upcast::<Control>());
    } else if let Some(reveal) = &row.reveal {
        cell.children()
            .add(property_source_link("View XAML…", reveal));
    } else {
        let value = TextBox::new();
        value.set_text(Some(&row.display_value));
        value.set_is_read_only(true);
        value.set_min_height(28.0);
        value.set_font_size(12.0);
        AutomationProperties::set_name(&value, Some(&format!("{} read only value", row.name)));
        cell.children().add(value.upcast::<Control>());
    }
    cell.upcast()
}

fn property_source_link(label: &str, reveal: &Rc<dyn Fn()>) -> Ref<Control> {
    let link = button(label);
    link.classes().add("property-source-link");
    link.set_horizontal_content_alignment(HorizontalAlignment::Left);
    let reveal = reveal.clone();
    link.click(move |_, _| reveal());
    link.upcast()
}

fn live_property_tooltip(object: &FerroObject, name: &str, origin: &str) -> Option<String> {
    let property = Inspector::property(object, name).ok()?;
    let diagnostic = object.get_diagnostic(property);
    let priority = if property.is_direct() {
        "Direct".to_owned()
    } else {
        format!("{:?}", diagnostic.priority())
    };
    let mut text = format!(
        "{} · {}\n{}\nEffective value: {}\nRuntime origin: {}",
        name,
        property.property_type_name(),
        origin,
        ferroui_inspect::display_value(Some(diagnostic.value())),
        priority,
    );
    if BindingOperations::get_binding_expression_base(object, property).is_some() {
        text.push_str(" · binding");
    }
    if diagnostic.is_overridden_current_value() {
        text.push_str(" · current value override");
    }
    if property.is_read_only() {
        text.push_str(" · read only");
    }
    Some(text)
}

fn property_element<'a>(
    document: &'a ferroui_designer_core::Document,
    node: &ferroui_designer_core::Node,
    property: &str,
) -> Option<&'a ferroui_designer_core::Node> {
    node.children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .find(|child| {
            child.kind == ferroui_designer_core::NodeKind::PropertyElement
                && (child.local_name() == property
                    || child
                        .local_name()
                        .strip_suffix(property)
                        .is_some_and(|prefix| prefix.ends_with('.')))
        })
}

fn is_metadata_container(node: &ferroui_designer_core::Node) -> bool {
    node.kind == ferroui_designer_core::NodeKind::PropertyElement
        && matches!(
            node.local_name().rsplit('.').next().unwrap_or_default(),
            "Styles"
                | "Resources"
                | "Animations"
                | "Transitions"
                | "DataTemplates"
                | "Template"
                | "ControlTheme"
                | "VisualStateGroups"
                | "RowDefinitions"
                | "ColumnDefinitions"
        )
}

fn property_element_content<'a>(
    document: &'a ferroui_designer_core::Document,
    node: &ferroui_designer_core::Node,
    property: &str,
) -> Option<&'a str> {
    let element = property_element(document, node, property)?;
    let end = element
        .closing_span
        .map_or(element.span.end, |span| span.start);
    document.source().get(element.opening_span.end..end)
}

impl DesignerWorkspace {
    pub fn outline_grid(&self) -> Ref<TreeDataGrid> {
        self.inspectors.outline.control.clone()
    }

    pub fn property_grid(&self) -> Ref<TreeDataGrid> {
        self.inspectors.properties.clone()
    }

    pub fn project_grid(&self) -> Ref<TreeDataGrid> {
        self.inspectors.project.control.clone()
    }

    pub(super) fn connect_inspectors(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.inspectors
            .outline
            .source
            .row_selection()
            .unwrap()
            .add_selection_changed(Rc::new(move |_| {
                let Some(this) = weak.upgrade() else { return };
                if this.inspectors.guard.get() {
                    return;
                }
                if !this
                    .inspectors
                    .outline_snapshot
                    .borrow()
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.matches_document(&this.session.borrow()))
                {
                    return;
                }
                let rows = this.inspectors.outline.selected();
                let ids = rows
                    .iter()
                    .filter_map(|row| row.entry.source)
                    .collect::<BTreeSet<_>>();
                let runtime = if ids.is_empty() {
                    rows.first().and_then(|row| row.entry.runtime)
                } else {
                    None
                };
                this.runtime_selection.set(runtime);
                this.session.borrow_mut().select(ids.iter().copied());
                if let Some(id) = ids.first() {
                    this.reveal_source(*id);
                }
                // Selection events are raised while the selection model is committing.
                // Refresh other inspectors without mutating that model reentrantly.
                this.inspectors.guard.set(true);
                this.refresh(true);
                this.inspectors.guard.set(false);
            }));
        let weak = Rc::downgrade(self);
        self.inspectors
            .project
            .source
            .row_selection()
            .unwrap()
            .add_selection_changed(Rc::new(move |_| {
                let Some(this) = weak.upgrade() else { return };
                if this.inspectors.guard.get() {
                    return;
                }
                let Some(row) = this.inspectors.project.selected().into_iter().next() else {
                    return;
                };
                if let Some(project) = row.entry.project {
                    let result = this.select_project(project);
                    if result.is_err() {
                        this.report(result);
                        return;
                    }
                }
                if let Some(uri) = &row.entry.uri {
                    let result = this.activate_document(uri);
                    this.report(result);
                }
            }));
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<TextBox>("OutlineSearch")
            .text_changing(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.rebuild_outline();
                }
            });
    }

    pub(super) fn rebuild_outline(self: &Rc<Self>) {
        if self.inspectors.guard.get() {
            return;
        }
        let session = self.session.borrow();
        let selected = session.selection().iter().copied().collect::<BTreeSet<_>>();
        let snapshot = OutlineSnapshot {
            uri: session.document_uri().to_owned(),
            revision: session.document().revision(),
            ast: session.document().ast_snapshot(),
            generation: if self.outline_mode.get() == OutlineMode::Source {
                0
            } else {
                session.generation()
            },
            mode: self.outline_mode.get(),
            filter: self
                .root
                .get_control::<TextBox>("OutlineSearch")
                .text()
                .unwrap_or_default()
                .trim()
                .to_lowercase(),
        };
        let previous = self.inspectors.guard.replace(true);
        if self.inspectors.outline_snapshot.borrow().as_ref() != Some(&snapshot) {
            if self
                .inspectors
                .outline_snapshot
                .borrow()
                .as_ref()
                .is_some_and(|old| old.uri != snapshot.uri)
            {
                self.inspectors.outline.expansion.borrow_mut().clear();
                self.inspectors.outline.paths.borrow_mut().clear();
            }
            let entries = if snapshot.mode == OutlineMode::Source {
                session
                    .document()
                    .ast()
                    .nodes_in_source_order()
                    .filter(|node| node.is_element())
                    .map(|node| TreeEntry {
                        key: format!("source:{}", node.id),
                        parent: node.parent.map(|id| format!("source:{id}")),
                        label: node.name.clone(),
                        name: node.xaml_name().unwrap_or_default().to_owned(),
                        detail: "XAML".into(),
                        source: Some(node.id),
                        initially_expanded: Some(!is_metadata_container(node)),
                        ..Default::default()
                    })
                    .collect()
            } else {
                session
                    .runtime_tree(if snapshot.mode == OutlineMode::Visual {
                        TreeKind::Visual
                    } else {
                        TreeKind::Logical
                    })
                    .into_iter()
                    .map(|row| TreeEntry {
                        key: format!("runtime:{}", row.inspector_id),
                        parent: row.parent.map(|id| format!("runtime:{id}")),
                        label: row.type_name,
                        name: row.name.unwrap_or_default(),
                        detail: if row.source_id.is_some() {
                            "XAML"
                        } else {
                            "Runtime"
                        }
                        .into(),
                        source: row.source_id,
                        runtime: Some(row.inspector_id),
                        ..Default::default()
                    })
                    .collect()
            };
            self.inspectors.outline.replace(entries, &snapshot.filter);
            *self.inspectors.outline_snapshot.borrow_mut() = Some(snapshot);
        }
        let runtime = self.runtime_selection.get();
        drop(session);
        self.inspectors.outline.select(|entry| {
            if let Some(runtime) = runtime {
                entry.runtime == Some(runtime)
            } else {
                entry.source.is_some_and(|id| selected.contains(&id))
            }
        });
        self.inspectors.guard.set(previous);
    }
}

impl DesignerWorkspace {
    pub(super) fn rebuild_properties(self: &Rc<Self>) {
        if let Some(id) = self.runtime_selection.get() {
            if self.rebuild_runtime_properties(id) {
                return;
            }
            self.runtime_selection.set(None);
        }
        let session = self.session.borrow();
        let selected = session.selection().to_vec();
        let revision = session.document().revision();
        let ast_snapshot = session.document().ast_snapshot();
        let uri = session.document_uri().to_owned();
        let read_only = session.is_read_only();
        let Some(first) = selected
            .first()
            .and_then(|id| session.document().ast().node(*id))
        else {
            self.selection_title.set_text(Some("Select an element"));
            self.inspectors
                .property_source
                .set_items(Rc::new(Vec::new()));
            return;
        };
        self.selection_title.set_text(Some(&if selected.len() == 1 {
            first.name.clone()
        } else {
            format!("{} elements", selected.len())
        }));
        let mut properties = vec![
            "Name",
            "Width",
            "Height",
            "MinWidth",
            "MinHeight",
            "Margin",
            "Padding",
            "HorizontalAlignment",
            "VerticalAlignment",
            "Opacity",
            "Background",
            "Foreground",
            "Text",
            "Content",
            "FontSize",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        properties.extend(
            first
                .attributes
                .iter()
                .filter(|attribute| !attribute.name.starts_with("xmlns"))
                .map(|attribute| attribute.name.clone()),
        );
        if let Some(parent) = first
            .parent
            .and_then(|id| session.document().ast().node(id))
        {
            match parent.name.rsplit(':').next().unwrap_or_default() {
                "Canvas" => properties.extend(
                    ["Canvas.Left", "Canvas.Top", "Canvas.Right", "Canvas.Bottom"]
                        .map(str::to_owned),
                ),
                "Grid" => properties.extend(
                    ["Grid.Row", "Grid.Column", "Grid.RowSpan", "Grid.ColumnSpan"]
                        .map(str::to_owned),
                ),
                "DockPanel" => properties.push("DockPanel.Dock".into()),
                _ => {}
            }
        }
        match first.name.rsplit(':').next().unwrap_or_default() {
            "Grid" => properties.extend(["RowDefinitions", "ColumnDefinitions"].map(str::to_owned)),
            "StackPanel" | "WrapPanel" => {
                properties.extend(["Orientation", "Spacing"].map(str::to_owned))
            }
            "UniformGrid" => {
                properties.extend(["Rows", "Columns", "FirstColumn"].map(str::to_owned))
            }
            "DockPanel" => properties.push("LastChildFill".into()),
            _ => {}
        }
        let live = session.preview.visual(first.id);
        let infos = live
            .as_ref()
            .map(|object| Inspector::properties(object))
            .unwrap_or_default();
        let known = live.as_ref().map(|_| {
            infos
                .iter()
                .filter(|info| !info.read_only)
                .map(|info| info.name.clone())
                .collect::<BTreeSet<_>>()
        });
        if let Some(known) = &known {
            properties.extend(known.iter().cloned());
        }
        for child in first
            .children
            .iter()
            .filter_map(|id| session.document().ast().node(*id))
        {
            if child.kind != ferroui_designer_core::NodeKind::PropertyElement {
                continue;
            }
            let qualified = child.local_name();
            let short = qualified.rsplit('.').next().unwrap_or(qualified);
            let property = if properties.iter().any(|property| property == qualified) {
                qualified
            } else if qualified.split('.').next() == Some(first.local_name())
                || known.as_ref().is_some_and(|known| known.contains(short))
            {
                short
            } else {
                qualified
            };
            properties.push(property.to_owned());
        }
        let effective = infos
            .iter()
            .map(|info| {
                (
                    info.name.clone(),
                    ferroui_inspect::display_value(Some(&info.value)),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        properties.sort();
        properties.dedup();
        properties.sort_by(|a, b| property_rank(a).cmp(&property_rank(b)).then(a.cmp(b)));
        let runtime_types = infos
            .iter()
            .map(|info| {
                (
                    info.name.clone(),
                    info.property.property_type_name().to_owned(),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let filter = self
            .root
            .get_control::<TextBox>("PropertySearch")
            .text()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let rows = properties
            .into_iter()
            .filter(|name| {
                first.attribute(name).is_some()
                    || name.contains('.')
                    || name == "Name"
                    || property_element(session.document(), first, name).is_some()
                    || known.as_ref().is_none_or(|known| known.contains(name))
            })
            .filter(|name| filter.is_empty() || name.to_ascii_lowercase().contains(&filter))
            .map(|name| {
                let document = session.document();
                let first_value = first.attribute(&name).map(str::to_owned);
                let first_content = property_element_content(document, first, &name);
                let graph = selected
                    .iter()
                    .filter_map(|id| document.ast().node(*id))
                    .find_map(|node| property_element(document, node, &name).map(|node| node.id));
                let mixed = selected.iter().skip(1).any(|id| {
                    document.ast().node(*id).is_some_and(|node| {
                        node.attribute(&name) != first_value.as_deref()
                            || property_element_content(document, node, &name) != first_content
                    })
                });
                let can_reset = graph.is_some()
                    || first_value.is_some()
                    || selected.iter().skip(1).any(|id| {
                        document
                            .ast()
                            .node(*id)
                            .is_some_and(|node| node.attribute(&name).is_some())
                    });
                (
                    name,
                    if mixed { None } else { first_value },
                    mixed,
                    can_reset,
                    graph,
                )
            })
            .collect::<Vec<_>>();
        drop(session);
        let factories = Rc::new(self.property_editors.borrow().clone());
        let rows = rows
            .into_iter()
            .map(|(name, value, mixed, can_reset, graph)| {
                let weak = Rc::downgrade(self);
                let property = name.clone();
                let ids = selected.clone();
                let uri = uri.clone();
                let ast_snapshot = ast_snapshot.clone();
                let origin = if mixed {
                    "Multiple values".to_owned()
                } else if graph.is_some() {
                    "Property element".to_owned()
                } else if value
                    .as_ref()
                    .is_some_and(|value| value.starts_with('{') && !value.starts_with("{}"))
                {
                    "Expression".to_owned()
                } else if value.is_some() {
                    "Local value".to_owned()
                } else {
                    "Inherited / default".to_owned()
                };
                let displayed = effective
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| value.clone().unwrap_or_default());
                let display_value = if mixed {
                    "Multiple values".to_owned()
                } else {
                    value.clone().unwrap_or_else(|| displayed.clone())
                };
                let runtime_type = runtime_types.get(&name).cloned();
                let tooltip = {
                    let weak = weak.clone();
                    let uri = uri.clone();
                    let ast_snapshot = ast_snapshot.clone();
                    let ids = ids.clone();
                    let name = name.clone();
                    let origin = format!("Authored value: {origin}");
                    let runtime_type = runtime_type.clone().unwrap_or_else(|| "XAML".into());
                    Rc::new(move || {
                        let this = weak.upgrade()?;
                        let session = this.session.borrow();
                        if session.document_uri() != uri
                            || session.document().revision() != revision
                            || !std::ptr::eq(ast_snapshot.as_ref(), session.document().ast())
                            || session.selection() != ids.as_slice()
                            || this.runtime_selection.get().is_some()
                            || !matches!(session.preview_status(), PreviewStatus::Ready { revision: ready } if *ready == revision)
                        {
                            return None;
                        }
                        Some(session.preview.visual(ids[0])
                            .and_then(|object| live_property_tooltip(&object, &name, &origin))
                            .unwrap_or_else(|| format!("{name} · {runtime_type}\n{origin}\nRuntime value: unavailable")))
                    }) as Rc<dyn Fn() -> Option<String>>
                };
                let reveal = graph.map(|node| {
                    let weak = Rc::downgrade(self);
                    let uri = uri.clone();
                    let ast_snapshot = ast_snapshot.clone();
                    let ids = ids.clone();
                    Rc::new(move || {
                        let Some(this) = weak.upgrade() else { return };
                        let session = this.session.borrow();
                        if session.document_uri() != uri
                            || session.document().revision() != revision
                            || !std::ptr::eq(ast_snapshot.as_ref(), session.document().ast())
                            || session.selection() != ids.as_slice()
                        {
                            drop(session);
                            this.rebuild_properties();
                            return;
                        }
                        let span = session.document().ast().node(node).map(|node| node.span);
                        drop(session);
                        if let Some(span) = span {
                            this.session.borrow_mut().select([node]);
                            this.select_source_span(span);
                            this.refresh(true);
                        }
                    }) as Rc<dyn Fn()>
                });
                let context = (!read_only).then(|| PropertyEditorContext {
                    property: name.clone(),
                    targets: selected.clone(),
                    value,
                    mixed,
                    runtime_type: runtime_type.clone(),
                    apply: Rc::new(move |value| {
                        let Some(this) = weak.upgrade() else { return };
                        if this.pending_source.get() {
                            this.apply_source();
                        }
                        // A recycled editor or extension must never apply a stale row to another
                        // tab or to a document changed by source editing/collaboration.
                        let session = this.session.borrow();
                        if session.document_uri() != uri
                            || session.document().revision() != revision
                            || session.selection() != ids.as_slice()
                            || !std::ptr::eq(ast_snapshot.as_ref(), session.document().ast())
                        {
                            drop(session);
                            this.rebuild_properties();
                            this.report(Err(
                                "The document changed; edit the refreshed property value".into(),
                            ));
                            return;
                        }
                        drop(session);
                        let result = match value {
                            None => this.session.borrow_mut().reset_property(&ids, &property),
                            Some(value) => this
                                .session
                                .borrow_mut()
                                .set_property(&ids, &property, &value),
                        };
                        this.refresh(true);
                        this.report(result);
                    }),
                });
                Rc::new(PropertyRow {
                    name,
                    display_value,
                    effective: displayed,
                    origin,
                    runtime_type: runtime_type.unwrap_or_else(|| "XAML".into()),
                    context,
                    factories: factories.clone(),
                    can_reset,
                    reveal,
                    tooltip,
                })
            })
            .collect::<Vec<_>>();
        self.inspectors.property_source.set_items(Rc::new(rows));
    }

    fn rebuild_runtime_properties(self: &Rc<Self>, id: u64) -> bool {
        let Ok(object) = Inspector::shared().borrow().resolve(id) else {
            return false;
        };
        self.selection_title
            .set_text(Some(&format!("{} · runtime", object.get_type().name())));
        let filter = self
            .root
            .get_control::<TextBox>("PropertySearch")
            .text()
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        let factories = Rc::new(Vec::new());
        let session = self.session.borrow();
        let uri = session.document_uri().to_owned();
        let revision = session.document().revision();
        let ast_snapshot = session.document().ast_snapshot();
        drop(session);
        let mut rows = Inspector::properties(&object)
            .into_iter()
            .filter(|info| filter.is_empty() || info.name.to_lowercase().contains(&filter))
            .map(|info| {
                let weak = Rc::downgrade(self);
                let uri = uri.clone();
                let ast_snapshot = ast_snapshot.clone();
                let name = info.name.clone();
                Rc::new(PropertyRow {
                    name: info.name.clone(),
                    display_value: ferroui_inspect::display_value(Some(&info.value)),
                    effective: ferroui_inspect::display_value(Some(&info.value)),
                    origin: format!(
                        "{}{} · read only",
                        info.priority_name(),
                        if info.has_binding { " · binding" } else { "" }
                    ),
                    runtime_type: info.property.property_type_name().to_owned(),
                    context: None,
                    factories: factories.clone(),
                    can_reset: false,
                    reveal: None,
                    tooltip: Rc::new(move || {
                        let this = weak.upgrade()?;
                        let session = this.session.borrow();
                        if session.document_uri() != uri
                            || session.document().revision() != revision
                            || !std::ptr::eq(ast_snapshot.as_ref(), session.document().ast())
                            || this.runtime_selection.get() != Some(id)
                            || !matches!(session.preview_status(), PreviewStatus::Ready { revision: ready } if *ready == revision)
                        {
                            return None;
                        }
                        let object = Inspector::shared().borrow().resolve(id).ok()?;
                        live_property_tooltip(&object, &name, "Runtime object · read only")
                    }),
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            property_rank(&a.name)
                .cmp(&property_rank(&b.name))
                .then(a.name.cmp(&b.name))
        });
        self.inspectors.property_source.set_items(Rc::new(rows));
        true
    }
}

#[cfg(test)]
#[path = "inspector_controls_tests.rs"]
mod tests;
