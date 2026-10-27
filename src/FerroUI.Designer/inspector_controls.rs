//! Virtualized inspectors shared by the source/runtime outline and solution explorer.
//! Row models contain data and semantic callbacks; controls are created only for realized cells.
use super::*;
use ferroui_base::data::BindingOperations;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::input::{DataTransferExtensions, DragDrop, DragDropEffects, IDataTransfer};
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::{BoxedValue, FerroObject};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{ColumnDefinitions, GridLength, GridUnitType, ToolTip};
use ferroui_controls_treedatagrid::models::{
    BeginEditGestures, HierarchicalExpanderColumn, TemplateColumn, TemplateColumnOptions,
};
use ferroui_controls_treedatagrid::{
    TreeDataGrid, TreeDataGridSourceExtensions, TreeDataGridSourceRef,
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
        // The outline is a layer navigator, not a spreadsheet. A single native template
        // column keeps its expander, row selection and drag/drop contracts while giving the
        // name all available width. Type information lives in the icon and accessible tooltip.
        let options = Rc::new(TemplateColumnOptions::<TreeRow>::new());
        options.set_min_width(GridLength::from_pixels(105.0));
        options.set_can_user_resize_column(Some(false));
        options.set_can_user_sort_column(Some(false));
        options.set_begin_edit_gestures(BeginEditGestures::NONE);
        options.set_is_text_search_enabled(true);
        options.set_text_search_value_selector(Some(Rc::new(move |row| {
            Some(hierarchy_name(&row.entry, project).to_owned())
        })));
        let inner = TemplateColumn::new(
            heading(if project { "Project / file" } else { "Element" }),
            FuncDataTemplate::for_type::<TreeRow>(
                move |row, _| Some(hierarchy_cell(row, project)),
                false,
            ),
            None,
            Some(GridLength::new(1.0, GridUnitType::Star)),
            Some(options),
        );
        let source = HierarchicalTreeDataGridSource::new(Rc::new(Vec::<Rc<TreeRow>>::new()))
            .with_column(HierarchicalExpanderColumn::new(
                inner,
                |row| Some(row.children.clone() as Models<TreeRow>),
                Some(Accessor::member("HasChildren", |row: &Rc<TreeRow>| {
                    !row.children.is_empty()
                })),
                Some(
                    Accessor::member("IsExpanded", |row: &Rc<TreeRow>| row.expanded.get())
                        .with_setter(|row, value| row.expanded.set(value)),
                ),
            ));
        source
            .row_selection()
            .unwrap()
            .base()
            .set_single_select(project);
        let control = make_grid(name);
        control.classes().add(if project {
            "designer-project-tree"
        } else {
            "designer-layer-tree"
        });
        control.set_show_column_headers(false);
        control.set_source(Some(TreeDataGridSourceRef::of::<TreeRow, _>(
            source.clone(),
        )));
        control.row_prepared(move |event| {
            let model = event.row().model();
            if let Some(row) = model
                .as_ref()
                .and_then(|model| model.downcast_ref::<TreeRow>())
            {
                let name = hierarchy_name(&row.entry, project);
                let kind = if project {
                    &row.entry.name
                } else {
                    &row.entry.label
                };
                let context = row.entry.uri.as_deref().unwrap_or(&row.entry.detail);
                let description = if name == kind {
                    format!("{name}\n{context}")
                } else {
                    format!("{name} · {kind}\n{context}")
                };
                AutomationProperties::set_name(event.row(), Some(&description));
                ToolTip::set_tip(event.row(), Some(Rc::new(description)));
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

fn hierarchy_name(entry: &TreeEntry, project: bool) -> &str {
    if !project && !entry.name.is_empty() {
        &entry.name
    } else {
        &entry.label
    }
}

fn hierarchy_cell(row: &TreeRow, project: bool) -> Ref<Control> {
    use ferroui_base::media::{Geometry, TextTrimming};
    use ferroui_controls::PathIcon;

    let root = Grid::new();
    root.classes().add("designer-layer-cell");
    root.set_column_definitions(ColumnDefinitions::parse("22,*").unwrap());
    root.set_min_height(28.0);
    root.set_margin(Thickness::new(0.0, 0.0, 6.0, 0.0));
    let (path, color, metadata) = hierarchy_icon(&row.entry, project);
    let icon = PathIcon::new();
    icon.set_width(14.0);
    icon.set_height(14.0);
    icon.set_horizontal_alignment(HorizontalAlignment::Left);
    icon.set_vertical_alignment(VerticalAlignment::Center);
    icon.set_foreground(Some(SolidColorBrush::from_uint32(color).into()));
    icon.set_data(Geometry::parse(path).expect("designer layer vector"));
    root.children().add(icon.upcast::<Control>());
    let name = TextBlock::new();
    name.classes().add(if metadata {
        "layer-metadata"
    } else if row.entry.name.is_empty() && !project {
        "layer-type"
    } else {
        "layer-name"
    });
    name.set_text(Some(hierarchy_name(&row.entry, project)));
    name.set_font_size(12.0);
    name.set_foreground(Some(
        SolidColorBrush::from_uint32(if metadata {
            0xff9299aa
        } else if row.entry.name.is_empty() && !project {
            0xffb8becc
        } else {
            0xfff2f3f5
        })
        .into(),
    ));
    name.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    name.set_vertical_alignment(VerticalAlignment::Center);
    Grid::set_column(&name, 1);
    root.children().add(name.upcast::<Control>());
    root.upcast()
}

/// Stable semantic glyphs for both authored and live trees. Unknown project controls
/// retain the neutral component glyph; property elements stay visibly distinct.
fn hierarchy_icon(entry: &TreeEntry, project: bool) -> (&'static str, u32, bool) {
    const FRAME: &str = "M3 3H21V21H3Z M5 5V19H19V5Z M8 3H10V21H8Z M3 8H21V10H3Z";
    const COMPONENT: &str = "M12 2L22 12L12 22L2 12Z M12 5L5 12L12 19L19 12Z";
    const TEXT: &str = "M3 4H21V6H13V21H11V6H3Z";
    const FOLDER: &str = "M2 5H10L12 7H22V20H2Z M4 7V18H20V9H11L9 7Z";
    const FILE: &str = "M5 2H14L20 8V22H5Z M7 4V20H18V9H13V4Z M15 5V7H17Z";
    const META: &str = "M4 6H10V8H4Z M14 6H20V8H14Z M4 11H20V13H4Z M4 16H10V18H4Z M14 16H20V18H14Z";
    const RECT: &str = "M3 4H21V20H3Z M5 6V18H19V6Z";
    const STACK: &str = "M3 3H21V7H3Z M5 5H19V5H5Z M3 10H21V14H3Z M3 17H21V21H3Z";
    const INPUT: &str = "M2 5H22V19H2Z M4 7V17H20V7Z M8 8H12V10H11V14H12V16H8V14H9V10H8Z";
    const TOGGLE: &str = "M3 3H21V21H3Z M5 5V19H19V5Z M7 11L10 14L17 7L18.5 8.5L10 17L5.5 12.5Z";
    const CIRCLE: &str =
        "M12 2A10 10 0 1 1 12 22A10 10 0 1 1 12 2Z M12 4A8 8 0 1 0 12 20A8 8 0 1 0 12 4Z";
    const IMAGE: &str =
        "M3 3H21V21H3Z M5 5V19H19V5Z M6 16L10 11L13 14L16 10L19 17H6Z M7 7H10V10H7Z";
    if project {
        return match entry.name.as_str() {
            "Project" => (COMPONENT, 0xffb6a2f6, false),
            "Folder" => (FOLDER, 0xff9caecb, false),
            "XAML" => (FILE, 0xff79a8ff, false),
            "Rust" => (FILE, 0xffd5ad8f, false),
            "Asset" => (IMAGE, 0xffa4c5b7, false),
            _ => (FILE, 0xffa9adba, false),
        };
    }
    let kind = entry.label.rsplit(':').next().unwrap_or(&entry.label);
    if kind.contains('.')
        || matches!(
            kind,
            "Style" | "Setter" | "ControlTheme" | "ResourceDictionary"
        )
    {
        return (META, 0xffa195c8, true);
    }
    let (path, color) = match kind {
        "Grid" | "Canvas" | "UniformGrid" | "DockPanel" | "WrapPanel" | "FlexPanel" => {
            (FRAME, 0xff9caecb)
        }
        "StackPanel" => (STACK, 0xff9caecb),
        "TextBlock" | "Run" | "Span" | "Label" => (TEXT, 0xffa9adba),
        "TextBox" | "ComboBox" | "NumericUpDown" => (INPUT, 0xffa9adba),
        "CheckBox" | "RadioButton" | "ToggleSwitch" | "ToggleButton" => (TOGGLE, 0xffa9adba),
        "Ellipse" => (CIRCLE, 0xffb6a2f6),
        "Border" | "Rectangle" | "Window" => (RECT, 0xff9caecb),
        "Image" | "Viewbox" => (IMAGE, 0xffa4c5b7),
        "Button" | "ContentControl" | "UserControl" => (RECT, 0xffa9adba),
        _ => (COMPONENT, 0xffa9adba),
    };
    (path, color, false)
}

pub(super) struct PropertyRow {
    category: Option<PropertyCategoryHeader>,
    pub name: String,
    pub display_value: String,
    pub effective: String,
    pub origin: String,
    pub runtime_type: String,
    pub context: Option<PropertyEditorContext>,
    pub factories: Rc<Vec<Rc<dyn PropertyEditorFactory>>>,
    pub can_reset: bool,
    pub reveal: Option<Rc<dyn Fn()>>,
    validate_drop: Option<PropertyDropValidator>,
    // Resolve on hover so animations can update diagnostics without replacing an editor
    // or changing the authored value while the user is typing.
    tooltip: Rc<dyn Fn() -> Option<String>>,
}

struct PropertyCategoryHeader {
    count: usize,
    expanded: bool,
    toggle: Rc<dyn Fn()>,
}

type PropertyDropValidator = Rc<dyn Fn(&PropertyDropPayload) -> bool>;
struct PropertyDropPayload {
    value: String,
    reference: Option<crate::property_controls::PropertyReference>,
}
struct CachedPropertyDrop {
    value: String,
    reference: Option<(NodeId, crate::property_controls::PropertyReferenceKind)>,
    accepted: bool,
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
    property_expansion: RefCell<BTreeMap<&'static str, bool>>,
}

impl InspectorControls {
    pub(super) fn replace_selection_guard(&self, guarded: bool) -> bool {
        self.guard.replace(guarded)
    }

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
            .with_template_column(
                heading("Property"),
                FuncDataTemplate::for_type::<PropertyRow>(
                    |row, _| Some(property_label(row)),
                    false,
                ),
                None,
                |options| {
                    options.width = GridLength::new(110.0, GridUnitType::Pixel);
                    options.min_width = GridLength::new(80.0, GridUnitType::Pixel);
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
        properties.set_show_column_headers(false);
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
            property_expansion: RefCell::new(BTreeMap::new()),
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
        ("TreeDataGridGridLinesBrush", 0x003b3d46),
        ("TreeDataGridSelectedCellBackgroundBrush", 0xff2d4061),
        ("TreeDataGridHeaderBackgroundPointerOverBrush", 0xff303138),
        ("TreeDataGridHeaderBackgroundPressedBrush", 0xff3b3d46),
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
    if let Some(category) = &row.category {
        let count = TextBlock::new();
        count.set_text(Some(&format!("{}", category.count)));
        count.set_font_size(10.0);
        count.set_foreground(Some(SolidColorBrush::from_uint32(0xff999999).into()));
        count.set_margin(Thickness::new(6.0, 0.0, 6.0, 0.0));
        count.set_vertical_alignment(VerticalAlignment::Center);
        count.set_horizontal_alignment(HorizontalAlignment::Right);
        return count.upcast::<Control>();
    }
    let cell = Grid::new();
    cell.classes().add("property-value-cell");
    cell.set_column_definitions(ColumnDefinitions::parse("*,Auto").unwrap());
    cell.set_min_height(26.0);
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
        if let Some(validate) = &row.validate_drop {
            connect_property_drop(&cell, context.apply.clone(), validate.clone());
        }
        let editor = if let Some(reveal) = &row.reveal {
            property_source_link("Edit XAML…", reveal)
        } else {
            row.factories
                .iter()
                .find_map(|factory| factory.create(context))
                .unwrap_or_else(|| {
                    // Typed editors start from the accepted effective value when inherited.
                    // Initialization never calls the revision-bound authoring callback.
                    let mut effective_context = context.clone();
                    if context.value.is_none()
                        && !context.mixed
                        && (crate::editors::components_kind(context).is_some()
                            || (crate::editors::numeric_options(context).is_some()
                                && row
                                    .effective
                                    .parse::<f64>()
                                    .is_ok_and(|value| value.is_finite())))
                    {
                        effective_context.value = Some(row.effective.clone());
                    }
                    crate::editors::built_in(&effective_context)
                })
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
        editor.set_min_height(26.0);
        editor.set_vertical_alignment(VerticalAlignment::Center);
        editor.classes().add("property-editor");
        AutomationProperties::set_name(&editor, Some(&format!("{} value", row.name)));
        cell.children().add(editor);
        let reset = button("↺");
        reset.classes().add("property-reset");
        reset.set_width(20.0);
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
        value.set_min_height(26.0);
        value.set_font_size(12.0);
        AutomationProperties::set_name(&value, Some(&format!("{} read only value", row.name)));
        cell.children().add(value.upcast::<Control>());
    }
    cell.upcast::<Control>()
}

fn dropped_property_value(data: &dyn IDataTransfer) -> Option<PropertyDropPayload> {
    Some(PropertyDropPayload {
        value: data
            .try_get_value(&crate::property_controls::property_value_data_format())
            .or_else(|| data.try_get_text())?,
        reference: data.try_get_value(&crate::property_controls::property_reference_data_format()),
    })
}

/// Native drops use the same validation and revision-bound commit as typing. Marking the
/// event handled keeps TreeDataGrid's row-reorder machinery out of a property-value edit.
fn connect_property_drop(
    cell: &Ref<Grid>,
    apply: Rc<dyn Fn(Option<String>)>,
    validate: PropertyDropValidator,
) {
    DragDrop::set_allow_drop(cell, true);
    let accepts = validate.clone();
    let weak = cell.downgrade();
    DragDrop::add_drag_over_handler(cell, move |_, event| {
        let accepted = event.drag_effects().contains(DragDropEffects::COPY)
            && dropped_property_value(event.data_transfer().as_ref())
                .is_some_and(|value| accepts(&value));
        event.set_drag_effects(if accepted {
            DragDropEffects::COPY
        } else {
            DragDropEffects::NONE
        });
        if let Some(cell) = weak.upgrade() {
            cell.classes().set("property-drop-target", accepted);
            cell.set_background(Some(
                SolidColorBrush::from_uint32(if accepted { 0xff35474e } else { 0x00000000 }).into(),
            ));
        }
        event.set_handled(true);
    });
    let weak = cell.downgrade();
    DragDrop::add_drag_leave_handler(cell, move |_, _| {
        if let Some(cell) = weak.upgrade() {
            cell.classes().remove("property-drop-target");
            cell.set_background(Some(SolidColorBrush::from_uint32(0x00000000).into()));
        }
    });
    let weak = cell.downgrade();
    DragDrop::add_drop_handler(cell, move |_, event| {
        if let Some(cell) = weak.upgrade() {
            cell.classes().remove("property-drop-target");
            cell.set_background(Some(SolidColorBrush::from_uint32(0x00000000).into()));
        }
        let value = dropped_property_value(event.data_transfer().as_ref()).filter(|value| {
            event.drag_effects().contains(DragDropEffects::COPY) && validate(value)
        });
        event.set_drag_effects(if value.is_some() {
            DragDropEffects::COPY
        } else {
            DragDropEffects::NONE
        });
        event.set_handled(true);
        if let Some(value) = value {
            apply(Some(value.value));
        }
    });
}

fn property_label(row: &PropertyRow) -> Ref<Control> {
    if let Some(category) = &row.category {
        let label = button(&format!(
            "{} {}",
            if category.expanded { "⌄" } else { "›" },
            row.name
        ));
        label.classes().add("property-category");
        label.set_min_height(26.0);
        label.set_min_width(0.0);
        label.set_font_size(11.0);
        label.set_padding(Thickness::new(2.0, 3.0, 0.0, 3.0));
        label.set_horizontal_content_alignment(HorizontalAlignment::Left);
        label.set_background(Some(SolidColorBrush::from_uint32(0x00000000).into()));
        label.set_border_thickness(Thickness::uniform(0.0));
        AutomationProperties::set_name(
            &label,
            Some(&format!(
                "{} {} properties",
                if category.expanded {
                    "Collapse"
                } else {
                    "Expand"
                },
                row.name
            )),
        );
        let toggle = category.toggle.clone();
        label.click(move |_, _| toggle());
        return label.upcast::<Control>();
    }
    let root = Grid::new();
    root.set_column_definitions(ColumnDefinitions::parse("7,*").unwrap());
    let state = TextBlock::new();
    state.set_text(Some(if row.origin == "Expression" {
        "◆"
    } else if row.can_reset {
        "•"
    } else {
        ""
    }));
    state.set_font_size(9.0);
    state.set_foreground(Some(
        SolidColorBrush::from_uint32(if row.origin == "Expression" {
            0xffc6a7ef
        } else {
            0xff60cdff
        })
        .into(),
    ));
    state.set_vertical_alignment(VerticalAlignment::Center);
    root.children().add(state.upcast::<Control>());
    let scrubber = row
        .context
        .as_ref()
        .filter(|context| !context.mixed && row.reveal.is_none())
        .and_then(|context| {
            let options = crate::editors::numeric_options(context)?;
            let value = context
                .value
                .as_deref()
                .unwrap_or(&row.effective)
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())?;
            let scrubber = crate::PropertyNumberScrubber::new();
            scrubber
                .configure(property_label_text(&row.name), value, options)
                .ok()?;
            AutomationProperties::set_name(&scrubber, Some(&format!("Adjust {}", row.name)));
            let apply = context.apply.clone();
            scrubber
                .on_commit(move |value| apply(Some(crate::property_controls::number_text(value))));
            Some(scrubber.upcast::<Control>())
        });
    let label = scrubber.unwrap_or_else(|| {
        let label = TextBlock::new();
        label.set_text(Some(property_label_text(&row.name)));
        label.set_font_size(11.0);
        label.set_vertical_alignment(VerticalAlignment::Center);
        label.set_text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis());
        label.upcast::<Control>()
    });
    ToolTip::set_tip(
        &label,
        Some(Rc::new(format!(
            "{} · {}\n{}{}",
            row.name,
            row.runtime_type,
            row.origin,
            if label.cast::<crate::PropertyNumberScrubber>().is_some() {
                "\nDrag to adjust · Shift ×10 · Ctrl ÷10 · Escape cancels"
            } else {
                ""
            }
        ))),
    );
    Grid::set_column(&label, 1);
    root.children().add(label);
    root.upcast::<Control>()
}

fn property_label_text(name: &str) -> &str {
    match name {
        "HorizontalAlignment" => "Horizontal",
        "VerticalAlignment" => "Vertical",
        "HorizontalContentAlignment" => "Content H align",
        "VerticalContentAlignment" => "Content V align",
        "Canvas.Left" => "X",
        "Canvas.Top" => "Y",
        "Canvas.Right" => "Right edge",
        "Canvas.Bottom" => "Bottom edge",
        "Grid.Row" => "Row",
        "Grid.Column" => "Column",
        "Grid.RowSpan" => "Row span",
        "Grid.ColumnSpan" => "Column span",
        "MinWidth" => "Min width",
        "MaxWidth" => "Max width",
        "MinHeight" => "Min height",
        "MaxHeight" => "Max height",
        "BorderBrush" => "Border color",
        "BorderThickness" => "Border width",
        "CornerRadius" => "Corner radius",
        "RenderTransform" => "Transform",
        "RenderTransformOrigin" => "Transform origin",
        "FontFamily" => "Font family",
        "FontSize" => "Font size",
        "FontWeight" => "Font weight",
        "TextWrapping" => "Text wrapping",
        "TextAlignment" => "Text alignment",
        _ => name,
    }
}

fn property_source_link(label: &str, reveal: &Rc<dyn Fn()>) -> Ref<Control> {
    let link = button(label);
    link.classes().add("property-source-link");
    link.set_horizontal_content_alignment(HorizontalAlignment::Left);
    let reveal = reveal.clone();
    link.click(move |_, _| reveal());
    link.upcast::<Control>()
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
            .filter(|name| {
                selected.iter().any(|id| {
                    session.document().ast().node(*id).is_some_and(|node| {
                        node.attribute(name).is_some()
                            || property_element(session.document(), node, name).is_some()
                    })
                }) || selected
                    .iter()
                    .all(|id| attached_layout_property_applies(session.document(), *id, name))
            })
            .filter(|name| {
                property_matches_filter(
                    name,
                    &filter,
                    selected.iter().any(|id| {
                        session.document().ast().node(*id).is_some_and(|node| {
                            node.attribute(name).is_some()
                                || property_element(session.document(), node, name).is_some()
                        })
                    }),
                )
            })
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
                let validate_drop = (!read_only && graph.is_none()).then(|| {
                    let weak = weak.clone();
                    let uri = uri.clone();
                    let ast_snapshot = ast_snapshot.clone();
                    let ids = ids.clone();
                    let property = property.clone();
                    let cached = RefCell::<Option<CachedPropertyDrop>>::new(None);
                    Rc::new(move |payload: &PropertyDropPayload| {
                        let Some(this) = weak.upgrade() else { return false; };
                        let Ok(session) = this.session.try_borrow() else { return false; };
                        if this.pending_source.get() || this.runtime_selection.get().is_some()
                            || session.is_read_only() || session.document_uri() != uri
                            || session.document().revision() != revision
                            || session.selection() != ids.as_slice()
                            || !std::ptr::eq(ast_snapshot.as_ref(), session.document().ast()) {
                            return false;
                        }
                        if payload.reference.as_ref().is_some_and(|reference| !property_reference_is_current(reference, &session)) {
                            return false;
                        }
                        let value = payload.value.as_str();
                        let reference = payload.reference.as_ref().map(|reference| (reference.node, reference.kind));
                        if let Some(previous) = cached.borrow().as_ref() {
                            if previous.value == value && previous.reference == reference { return previous.accepted; }
                        }
                        let accepted = payload.reference.as_ref().is_none_or(|reference| property_reference_applies(reference, &session, &ids)) && session.property_tools().plan(session.document(), &ids, &property, Some(value), ferroui_designer_tools::ExpressionPolicy::ReplaceExpressions).is_ok();
                        *cached.borrow_mut() = Some(CachedPropertyDrop { value: value.to_owned(), reference, accepted });
                        accepted
                    }) as PropertyDropValidator
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
                    category: None,
                    name,
                    display_value,
                    effective: displayed,
                    origin,
                    runtime_type: runtime_type.unwrap_or_else(|| "XAML".into()),
                    context,
                    factories: factories.clone(),
                    can_reset,
                    reveal,
                    validate_drop,
                    tooltip,
                })
            })
            .collect::<Vec<_>>();
        self.set_property_rows(rows, !filter.trim().is_empty());
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
            .filter(|info| property_matches_filter(&info.name, &filter, false))
            .map(|info| {
                let weak = Rc::downgrade(self);
                let uri = uri.clone();
                let ast_snapshot = ast_snapshot.clone();
                let name = info.name.clone();
                Rc::new(PropertyRow {
                    category: None,
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
                    validate_drop: None,
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
        self.set_property_rows(rows, !filter.trim().is_empty());
        true
    }

    fn set_property_rows(self: &Rc<Self>, rows: Vec<Rc<PropertyRow>>, filtering: bool) {
        let mut groups = BTreeMap::<(usize, &'static str), Vec<Rc<PropertyRow>>>::new();
        for row in rows {
            groups
                .entry(property_category(&row.name))
                .or_default()
                .push(row);
        }
        let mut visible = Vec::new();
        for ((_, name), rows) in groups {
            let expanded = filtering
                || self
                    .inspectors
                    .property_expansion
                    .borrow()
                    .get(name)
                    .copied()
                    .unwrap_or(name != "Advanced");
            let weak = Rc::downgrade(self);
            visible.push(Rc::new(PropertyRow {
                name: name.to_owned(),
                category: Some(PropertyCategoryHeader {
                    count: rows.len(),
                    expanded,
                    toggle: Rc::new(move || {
                        let Some(this) = weak.upgrade() else {
                            return;
                        };
                        if filtering {
                            return;
                        }
                        this.inspectors
                            .property_expansion
                            .borrow_mut()
                            .insert(name, !expanded);
                        this.rebuild_properties();
                    }),
                }),
                display_value: String::new(),
                effective: String::new(),
                origin: String::new(),
                runtime_type: String::new(),
                context: None,
                factories: Rc::new(Vec::new()),
                can_reset: false,
                reveal: None,
                validate_drop: None,
                tooltip: Rc::new(|| None),
            }));
            if expanded {
                visible.extend(rows);
            }
        }
        self.inspectors.property_source.set_items(Rc::new(visible));
    }
}

fn property_category(name: &str) -> (usize, &'static str) {
    match name {
        "Name"
        | "x:Name"
        | "Width"
        | "Height"
        | "Margin"
        | "Padding"
        | "HorizontalAlignment"
        | "VerticalAlignment"
        | "HorizontalContentAlignment"
        | "VerticalContentAlignment"
        | "RowDefinitions"
        | "ColumnDefinitions"
        | "Rows"
        | "Columns"
        | "Orientation"
        | "Spacing"
        | "RowSpacing"
        | "ColumnSpacing"
        | "FirstColumn"
        | "LastChildFill" => (0, "Layout"),
        "Background" | "Foreground" | "BorderBrush" | "BorderThickness" | "CornerRadius"
        | "Opacity" | "IsVisible" | "ClipToBounds" | "BoxShadow" => (1, "Appearance"),
        "Text" | "Content" | "Header" | "PlaceholderText" | "Watermark" | "TextAlignment"
        | "TextWrapping" | "TextTrimming" | "LineHeight" | "LetterSpacing" | "FlowDirection" => {
            (2, "Content & text")
        }
        "RenderTransform" | "RenderTransformOrigin" | "ZIndex" => (3, "Transform"),
        "IsEnabled" | "IsHitTestVisible" | "Focusable" | "IsTabStop" | "TabIndex"
        | "IsReadOnly" | "IsChecked" | "IsSelected" | "IsExpanded" | "Cursor" => (4, "Interaction"),
        name if name.starts_with("Canvas.")
            || name.starts_with("Grid.")
            || name.starts_with("DockPanel.")
            || name.starts_with("RelativePanel.")
            || name.starts_with("FlexPanel.") =>
        {
            (0, "Layout")
        }
        name if name.starts_with("Font") => (2, "Content & text"),
        _ => (5, "Advanced"),
    }
}

fn property_matches_filter(name: &str, filter: &str, modified: bool) -> bool {
    let haystack = format!(
        "{} {}",
        name.to_lowercase(),
        property_category(name).1.to_lowercase()
    );
    filter.split_whitespace().all(|term| {
        if matches!(term, "@modified" | "@local") {
            modified
        } else {
            haystack.contains(term)
        }
    })
}

/// Layout attached values are offered only for their actual parent panel. Authored values
/// are retained by the caller so moving an element never conceals an existing override.
fn attached_layout_property_applies(
    document: &ferroui_designer_core::Document,
    node: NodeId,
    property: &str,
) -> bool {
    let Some((owner, _)) = property.split_once('.') else {
        return true;
    };
    if !matches!(
        owner,
        "Canvas" | "Grid" | "DockPanel" | "RelativePanel" | "FlexPanel"
    ) {
        return true;
    }
    let mut parent = document.ast().node(node).and_then(|node| node.parent);
    while let Some(id) = parent {
        let Some(node) = document.ast().node(id) else {
            return false;
        };
        if node.kind != ferroui_designer_core::NodeKind::PropertyElement {
            return node.local_name() == owner;
        }
        parent = node.parent;
    }
    false
}

fn resource_key(node: &ferroui_designer_core::Node) -> Option<&str> {
    node.attributes
        .iter()
        .find(|attribute| {
            attribute.name.rsplit(':').next() == Some("Key")
                && attribute.namespace_uri.as_deref()
                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
        })
        .map(|attribute| attribute.value.as_str())
}

/// Finds a key only in this resource scope, descending through anonymous/merged
/// dictionaries but never into a resource object's nested content or Resources.
fn scoped_resource(ast: &ferroui_designer_core::Ast, owner: NodeId, key: &str) -> Option<NodeId> {
    let owner = ast.node(owner)?;
    let children = if owner.local_name() == "ResourceDictionary" {
        owner.children.clone()
    } else {
        owner
            .children
            .iter()
            .filter_map(|id| ast.node(*id))
            .filter(|node| {
                node.kind == ferroui_designer_core::NodeKind::PropertyElement
                    && node.local_name().ends_with(".Resources")
            })
            .flat_map(|node| node.children.iter().copied())
            .collect()
    };
    let mut scopes = vec![children];
    while let Some(children) = scopes.pop() {
        // Local entries override merged dictionaries independently of their source order.
        for id in &children {
            if ast
                .node(*id)
                .is_some_and(|node| resource_key(node) == Some(key))
            {
                return Some(*id);
            }
        }
        // Stack order visits the last merged dictionary first, matching resource lookup.
        for node in children.iter().filter_map(|id| ast.node(*id)) {
            if node.local_name().ends_with(".MergedDictionaries") {
                for dictionary in node
                    .children
                    .iter()
                    .filter_map(|id| ast.node(*id))
                    .filter(|node| node.local_name() == "ResourceDictionary")
                {
                    scopes.push(dictionary.children.clone());
                }
            }
        }
        for node in children.iter().rev().filter_map(|id| ast.node(*id)) {
            if node.local_name() == "ResourceDictionary" && resource_key(node).is_none() {
                scopes.push(node.children.clone());
            }
        }
    }
    None
}

fn property_reference_is_current(
    reference: &crate::property_controls::PropertyReference,
    session: &DesignerSession,
) -> bool {
    reference.document_uri == session.document_uri()
        && reference.revision == session.document().revision()
        && std::ptr::eq(reference.ast.as_ref(), session.document().ast())
        && session.document().ast().node(reference.node).is_some()
}

fn property_reference_applies(
    reference: &crate::property_controls::PropertyReference,
    session: &DesignerSession,
    targets: &[NodeId],
) -> bool {
    use crate::property_controls::PropertyReferenceKind;
    let ast = session.document().ast();
    if !property_reference_is_current(reference, session) {
        return false;
    }
    let Some(source) = ast.node(reference.node) else {
        return false;
    };
    match reference.kind {
        PropertyReferenceKind::Element => {
            source.xaml_name().is_some()
                && targets
                    .iter()
                    .all(|target| ast.name_scope(*target) == ast.name_scope(source.id))
        }
        PropertyReferenceKind::Resource => {
            let Some(key) = resource_key(source) else {
                return false;
            };
            targets.iter().all(|target| {
                let mut ancestor = Some(*target);
                while let Some(id) = ancestor {
                    let Some(node) = ast.node(id) else {
                        return false;
                    };
                    if let Some(found) = scoped_resource(ast, id, key) {
                        return found == source.id;
                    }
                    ancestor = node.parent;
                }
                false
            })
        }
    }
}

#[cfg(test)]
#[path = "inspector_controls_tests.rs"]
mod tests;
