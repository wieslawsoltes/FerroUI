//! Markup metadata of the declarative columns of a grid (`TreeDataGridColumn.cs`,
//! `TreeDataGridColumns.cs`), of the selection mode and of the arguments of the selection change of
//! the grid: what lets a document declare `<TreeDataGrid ItemsSource=".." SelectionMode="Row,Multiple">`
//! with its columns as content.
//!
//! The columns are plain objects, as in the reference. The members every column has
//! (`TreeDataGridColumn`) are declared on each column type, written out per type (the scan of the
//! sources that builds the type model reads the declarations as they are written), and each column
//! type is also a [`TreeDataGridColumn`] (the contract the list of columns and `InnerColumn` hold).
//! `CompareAscending` and `CompareDescending` are delegates, which a document does not state; code
//! sets them.

use crate::models::BeginEditGestures;
use crate::{
    TreeDataGridCheckBoxColumn, TreeDataGridColumn, TreeDataGridColumns, TreeDataGridHierarchicalExpanderColumn,
    TreeDataGridRowHeaderColumn, TreeDataGridSelectionChangedEventArgs, TreeDataGridSelectionMode,
    TreeDataGridTemplateColumn, TreeDataGridTextColumn,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::media::{TextAlignment, TextTrimming, TextWrapping};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::BoxedValue;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{AssignedBinding, GridLength};
use std::rc::Rc;

/// A binding a document assigns to a property (`[AssignBinding]`), as the column holds it.
fn assigned(value: Option<AssignedBinding>) -> Option<Rc<dyn ferroui_base::data::BindingBase>> {
    value.map(|binding| binding.0)
}

ferro_markup_type!(class TreeDataGridTextColumn {
    namespace: "FerroUI.Controls",
    handles: [Rc<TreeDataGridTextColumn>, Option<Rc<TreeDataGridTextColumn>>],
    this: Rc<TreeDataGridTextColumn>,
    interfaces: [Rc<dyn TreeDataGridColumn>],
    constructors: [() => TreeDataGridTextColumn::new],
    properties: [
        Header: Option<BoxedValue> {
            get: |this: &Rc<TreeDataGridTextColumn>| this.header(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: Option<BoxedValue>| this.set_header(value)
        },
        Width: GridLength {
            get: |this: &Rc<TreeDataGridTextColumn>| this.width(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: GridLength| this.set_width(value)
        },
        CanUserResize: Option<bool> {
            get: |this: &Rc<TreeDataGridTextColumn>| this.can_user_resize(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: Option<bool>| this.set_can_user_resize(value)
        },
        CanUserSortColumn: Option<bool> {
            get: |this: &Rc<TreeDataGridTextColumn>| this.can_user_sort_column(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: Option<bool>| this.set_can_user_sort_column(value)
        },
        AllowTriStateSorting: bool {
            get: |this: &Rc<TreeDataGridTextColumn>| this.allow_tri_state_sorting(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: bool| this.set_allow_tri_state_sorting(value)
        },
        MinWidth: GridLength {
            get: |this: &Rc<TreeDataGridTextColumn>| this.min_width(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: GridLength| this.set_min_width(value)
        },
        MaxWidth: Option<GridLength> {
            get: |this: &Rc<TreeDataGridTextColumn>| this.max_width(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: Option<GridLength>| this.set_max_width(value)
        },
        BeginEditGestures: BeginEditGestures {
            get: |this: &Rc<TreeDataGridTextColumn>| this.begin_edit_gestures(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: BeginEditGestures| this.set_begin_edit_gestures(value)
        },
        Binding: Option<AssignedBinding> {
            get: |this: &Rc<TreeDataGridTextColumn>| this.binding().map(AssignedBinding::new),
            set: |this: &Rc<TreeDataGridTextColumn>, value: Option<AssignedBinding>| this.set_binding(assigned(value))
        } [AssignBinding],
        IsReadOnly: bool {
            get: |this: &Rc<TreeDataGridTextColumn>| this.is_read_only(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: bool| this.set_is_read_only(value)
        },
        IsTextSearchEnabled: bool {
            get: |this: &Rc<TreeDataGridTextColumn>| this.is_text_search_enabled(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: bool| this.set_is_text_search_enabled(value)
        },
        StringFormat: String {
            get: |this: &Rc<TreeDataGridTextColumn>| this.string_format(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: String| this.set_string_format(value)
        },
        TextAlignment: TextAlignment {
            get: |this: &Rc<TreeDataGridTextColumn>| this.text_alignment(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: TextAlignment| this.set_text_alignment(value)
        },
        TextTrimming: Rc<dyn TextTrimming> {
            get: |this: &Rc<TreeDataGridTextColumn>| this.text_trimming(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: Rc<dyn TextTrimming>| this.set_text_trimming(value)
        },
        TextWrapping: TextWrapping {
            get: |this: &Rc<TreeDataGridTextColumn>| this.text_wrapping(),
            set: |this: &Rc<TreeDataGridTextColumn>, value: TextWrapping| this.set_text_wrapping(value)
        },
    ],
});

ferro_markup_type!(class TreeDataGridCheckBoxColumn {
    namespace: "FerroUI.Controls",
    handles: [Rc<TreeDataGridCheckBoxColumn>, Option<Rc<TreeDataGridCheckBoxColumn>>],
    this: Rc<TreeDataGridCheckBoxColumn>,
    interfaces: [Rc<dyn TreeDataGridColumn>],
    constructors: [() => TreeDataGridCheckBoxColumn::new],
    properties: [
        Header: Option<BoxedValue> {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.header(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: Option<BoxedValue>| this.set_header(value)
        },
        Width: GridLength {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.width(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: GridLength| this.set_width(value)
        },
        CanUserResize: Option<bool> {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.can_user_resize(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: Option<bool>| this.set_can_user_resize(value)
        },
        CanUserSortColumn: Option<bool> {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.can_user_sort_column(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: Option<bool>| this.set_can_user_sort_column(value)
        },
        AllowTriStateSorting: bool {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.allow_tri_state_sorting(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: bool| this.set_allow_tri_state_sorting(value)
        },
        MinWidth: GridLength {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.min_width(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: GridLength| this.set_min_width(value)
        },
        MaxWidth: Option<GridLength> {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.max_width(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: Option<GridLength>| this.set_max_width(value)
        },
        BeginEditGestures: BeginEditGestures {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.begin_edit_gestures(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: BeginEditGestures| this.set_begin_edit_gestures(value)
        },
        Binding: Option<AssignedBinding> {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.binding().map(AssignedBinding::new),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: Option<AssignedBinding>| this.set_binding(assigned(value))
        } [AssignBinding],
        IsReadOnly: bool {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.is_read_only(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: bool| this.set_is_read_only(value)
        },
        IsThreeState: Option<bool> {
            get: |this: &Rc<TreeDataGridCheckBoxColumn>| this.is_three_state(),
            set: |this: &Rc<TreeDataGridCheckBoxColumn>, value: Option<bool>| this.set_is_three_state(value)
        },
    ],
});

ferro_markup_type!(class TreeDataGridTemplateColumn {
    namespace: "FerroUI.Controls",
    handles: [Rc<TreeDataGridTemplateColumn>, Option<Rc<TreeDataGridTemplateColumn>>],
    this: Rc<TreeDataGridTemplateColumn>,
    interfaces: [Rc<dyn TreeDataGridColumn>],
    constructors: [() => TreeDataGridTemplateColumn::new],
    properties: [
        Header: Option<BoxedValue> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.header(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<BoxedValue>| this.set_header(value)
        },
        Width: GridLength {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.width(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: GridLength| this.set_width(value)
        },
        CanUserResize: Option<bool> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.can_user_resize(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<bool>| this.set_can_user_resize(value)
        },
        CanUserSortColumn: Option<bool> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.can_user_sort_column(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<bool>| this.set_can_user_sort_column(value)
        },
        AllowTriStateSorting: bool {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.allow_tri_state_sorting(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: bool| this.set_allow_tri_state_sorting(value)
        },
        MinWidth: GridLength {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.min_width(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: GridLength| this.set_min_width(value)
        },
        MaxWidth: Option<GridLength> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.max_width(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<GridLength>| this.set_max_width(value)
        },
        BeginEditGestures: BeginEditGestures {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.begin_edit_gestures(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: BeginEditGestures| this.set_begin_edit_gestures(value)
        },
        CellTemplate: Option<Rc<dyn IDataTemplate>> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.cell_template(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<Rc<dyn IDataTemplate>>| this.set_cell_template(value)
        },
        CellEditingTemplate: Option<Rc<dyn IDataTemplate>> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.cell_editing_template(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<Rc<dyn IDataTemplate>>| this.set_cell_editing_template(value)
        },
        CellTemplateResourceKey: Option<ResourceKey> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.cell_template_resource_key(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<ResourceKey>| this.set_cell_template_resource_key(value)
        },
        CellEditingTemplateResourceKey: Option<ResourceKey> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.cell_editing_template_resource_key(),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<ResourceKey>| {
                this.set_cell_editing_template_resource_key(value)
            }
        },
        TextSearchBinding: Option<AssignedBinding> {
            get: |this: &Rc<TreeDataGridTemplateColumn>| this.text_search_binding().map(AssignedBinding::new),
            set: |this: &Rc<TreeDataGridTemplateColumn>, value: Option<AssignedBinding>| this.set_text_search_binding(assigned(value))
        } [AssignBinding],
    ],
});

ferro_markup_type!(class TreeDataGridRowHeaderColumn {
    namespace: "FerroUI.Controls",
    handles: [Rc<TreeDataGridRowHeaderColumn>, Option<Rc<TreeDataGridRowHeaderColumn>>],
    this: Rc<TreeDataGridRowHeaderColumn>,
    interfaces: [Rc<dyn TreeDataGridColumn>],
    constructors: [() => TreeDataGridRowHeaderColumn::new],
    properties: [
        Header: Option<BoxedValue> {
            get: |this: &Rc<TreeDataGridRowHeaderColumn>| this.header(),
            set: |this: &Rc<TreeDataGridRowHeaderColumn>, value: Option<BoxedValue>| this.set_header(value)
        },
        Width: GridLength {
            get: |this: &Rc<TreeDataGridRowHeaderColumn>| this.width(),
            set: |this: &Rc<TreeDataGridRowHeaderColumn>, value: GridLength| this.set_width(value)
        },
        CanUserResize: Option<bool> {
            get: |this: &Rc<TreeDataGridRowHeaderColumn>| this.can_user_resize(),
            set: |this: &Rc<TreeDataGridRowHeaderColumn>, value: Option<bool>| this.set_can_user_resize(value)
        },
        CanUserSortColumn: Option<bool> {
            get: |this: &Rc<TreeDataGridRowHeaderColumn>| this.can_user_sort_column(),
            set: |this: &Rc<TreeDataGridRowHeaderColumn>, value: Option<bool>| this.set_can_user_sort_column(value)
        },
        AllowTriStateSorting: bool {
            get: |this: &Rc<TreeDataGridRowHeaderColumn>| this.allow_tri_state_sorting(),
            set: |this: &Rc<TreeDataGridRowHeaderColumn>, value: bool| this.set_allow_tri_state_sorting(value)
        },
        MinWidth: GridLength {
            get: |this: &Rc<TreeDataGridRowHeaderColumn>| this.min_width(),
            set: |this: &Rc<TreeDataGridRowHeaderColumn>, value: GridLength| this.set_min_width(value)
        },
        MaxWidth: Option<GridLength> {
            get: |this: &Rc<TreeDataGridRowHeaderColumn>| this.max_width(),
            set: |this: &Rc<TreeDataGridRowHeaderColumn>, value: Option<GridLength>| this.set_max_width(value)
        },
        BeginEditGestures: BeginEditGestures {
            get: |this: &Rc<TreeDataGridRowHeaderColumn>| this.begin_edit_gestures(),
            set: |this: &Rc<TreeDataGridRowHeaderColumn>, value: BeginEditGestures| this.set_begin_edit_gestures(value)
        },
    ],
});

ferro_markup_type!(class TreeDataGridHierarchicalExpanderColumn {
    namespace: "FerroUI.Controls",
    handles: [Rc<TreeDataGridHierarchicalExpanderColumn>, Option<Rc<TreeDataGridHierarchicalExpanderColumn>>],
    this: Rc<TreeDataGridHierarchicalExpanderColumn>,
    interfaces: [Rc<dyn TreeDataGridColumn>],
    constructors: [() => TreeDataGridHierarchicalExpanderColumn::new],
    content: InnerColumn,
    properties: [
        Header: Option<BoxedValue> {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.header(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: Option<BoxedValue>| this.set_header(value)
        },
        Width: GridLength {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.width(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: GridLength| this.set_width(value)
        },
        CanUserResize: Option<bool> {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.can_user_resize(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: Option<bool>| this.set_can_user_resize(value)
        },
        CanUserSortColumn: Option<bool> {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.can_user_sort_column(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: Option<bool>| this.set_can_user_sort_column(value)
        },
        AllowTriStateSorting: bool {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.allow_tri_state_sorting(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: bool| this.set_allow_tri_state_sorting(value)
        },
        MinWidth: GridLength {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.min_width(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: GridLength| this.set_min_width(value)
        },
        MaxWidth: Option<GridLength> {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.max_width(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: Option<GridLength>| this.set_max_width(value)
        },
        BeginEditGestures: BeginEditGestures {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.begin_edit_gestures(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: BeginEditGestures| this.set_begin_edit_gestures(value)
        },
        ChildrenBinding: Option<AssignedBinding> {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.children_binding().map(AssignedBinding::new),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: Option<AssignedBinding>| {
                this.set_children_binding(assigned(value))
            }
        } [AssignBinding],
        HasChildrenBinding: Option<AssignedBinding> {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.has_children_binding().map(AssignedBinding::new),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: Option<AssignedBinding>| {
                this.set_has_children_binding(assigned(value))
            }
        } [AssignBinding],
        IsExpandedBinding: Option<AssignedBinding> {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.is_expanded_binding().map(AssignedBinding::new),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: Option<AssignedBinding>| {
                this.set_is_expanded_binding(assigned(value))
            }
        } [AssignBinding],
        InnerColumn: Option<Rc<dyn TreeDataGridColumn>> {
            get: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>| this.inner_column(),
            set: |this: &Rc<TreeDataGridHierarchicalExpanderColumn>, value: Option<Rc<dyn TreeDataGridColumn>>| {
                this.set_inner_column(value)
            }
        },
    ],
});

// The contract of the columns: what the list of columns and `InnerColumn` hold.
ferro_markup_type!(interface dyn TreeDataGridColumn as "TreeDataGridColumn" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn TreeDataGridColumn>, Option<Rc<dyn TreeDataGridColumn>>],
    this: Rc<dyn TreeDataGridColumn>,
});

// The declarative columns of a grid (`TreeDataGrid.ColumnDefinitions`).
ferro_markup_type!(class TreeDataGridColumns {
    namespace: "FerroUI.Controls",
    handles: [TreeDataGridColumns, Option<TreeDataGridColumns>],
    constructors: [() => TreeDataGridColumns::new],
    properties: [
        Count: i32 { get: |list: &TreeDataGridColumns| list.count() as i32 },
    ],
    methods: [fn Add(Rc<dyn TreeDataGridColumn>) => |list: &TreeDataGridColumns, item: Rc<dyn TreeDataGridColumn>| list.add(item)],
});

/// `Enum.Parse` of the flags of the selection mode.
fn parse_selection_mode(text: &str) -> Result<TreeDataGridSelectionMode, String> {
    TreeDataGridSelectionMode::parse(text).ok_or_else(|| format!("'{text}' is not a TreeDataGridSelectionMode"))
}

ferro_markup_type!(struct TreeDataGridSelectionMode {
    namespace: "FerroUI.Controls",
    handles: [TreeDataGridSelectionMode],
    parse: parse_selection_mode,
});

/// `Enum.Parse` of the flags of the gestures that start editing a cell.
fn parse_begin_edit_gestures(text: &str) -> Result<BeginEditGestures, String> {
    BeginEditGestures::parse(text).ok_or_else(|| format!("'{text}' is not a BeginEditGestures"))
}

ferro_markup_type!(struct BeginEditGestures {
    namespace: "FerroUI.Controls.Models.TreeDataGrid",
    handles: [BeginEditGestures],
    parse: parse_begin_edit_gestures,
});

ferro_markup_type!(class TreeDataGridSelectionChangedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [TreeDataGridSelectionChangedEventArgs],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <TreeDataGridTextColumn as MarkupTyped>::MARKUP,
    <TreeDataGridCheckBoxColumn as MarkupTyped>::MARKUP,
    <TreeDataGridTemplateColumn as MarkupTyped>::MARKUP,
    <TreeDataGridRowHeaderColumn as MarkupTyped>::MARKUP,
    <TreeDataGridHierarchicalExpanderColumn as MarkupTyped>::MARKUP,
    <dyn TreeDataGridColumn as MarkupTyped>::MARKUP,
    <TreeDataGridColumns as MarkupTyped>::MARKUP,
    <TreeDataGridSelectionMode as MarkupTyped>::MARKUP,
    <BeginEditGestures as MarkupTyped>::MARKUP,
    <TreeDataGridSelectionChangedEventArgs as MarkupTyped>::MARKUP,
];

/// Registers the nullable handles of the columns and their casts to the column contract (one line
/// per type: the scan of the sources reads the types of the casts as they are written).
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<TreeDataGridTextColumn>>();
    ValueTypes::register_cast::<Rc<TreeDataGridTextColumn>, Rc<dyn TreeDataGridColumn>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<TreeDataGridCheckBoxColumn>>();
    ValueTypes::register_cast::<Rc<TreeDataGridCheckBoxColumn>, Rc<dyn TreeDataGridColumn>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<TreeDataGridTemplateColumn>>();
    ValueTypes::register_cast::<Rc<TreeDataGridTemplateColumn>, Rc<dyn TreeDataGridColumn>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<TreeDataGridRowHeaderColumn>>();
    ValueTypes::register_cast::<Rc<TreeDataGridRowHeaderColumn>, Rc<dyn TreeDataGridColumn>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<TreeDataGridHierarchicalExpanderColumn>>();
    ValueTypes::register_cast::<Rc<TreeDataGridHierarchicalExpanderColumn>, Rc<dyn TreeDataGridColumn>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<dyn TreeDataGridColumn>>();
    ValueTypes::register_nullable::<TreeDataGridColumns>();
}
