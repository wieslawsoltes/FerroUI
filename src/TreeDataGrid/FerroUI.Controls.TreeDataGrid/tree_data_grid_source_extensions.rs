//! `TreeDataGridSourceExtensions.cs`: fluent helpers that add the control's view columns to a
//! source (`WithTextColumn`, `WithCheckBoxColumn`, ...), with the options of the columns they
//! create (`ColumnCreateOptions` and its forms).
//!
//! The reference takes expressions, reads the member name for a default header and synthesizes a
//! setter from the expression; the port takes accessors, which carry the member name and the
//! setter (`TreeDataGridExpressionHelper.cs` is replaced by them: `TryGetMemberName` is
//! `Accessor::property_name`, `TryCreateSetter` the accessor's setter). A column created read only
//! drops the accessor's setter.

use crate::models::{
    BeginEditGestures, CheckBoxColumn, CheckBoxColumnOptions, ColumnOptions, HierarchicalExpanderColumn, IViewColumn,
    TemplateColumn, TemplateColumnOptions, TextColumn, TextColumnOptions, TreeDataGridRowHeaderColumnInternal,
};
use crate::{TreeDataGridBindingAccessor, TreeDataGridTemplateColumn};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::BindingBase;
use ferroui_base::media::{TextAlignment, TextTrimming, TextWrapping};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{ColumnList, ColumnRef, Comparison};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, Models};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// The options of a column created by the helpers (`ColumnCreateOptions<TModel>`).
pub struct ColumnCreateOptions<M: 'static> {
    pub width: GridLength,
    pub can_user_resize: Option<bool>,
    pub can_user_sort_column: Option<bool>,
    pub allow_tri_state_sorting: bool,
    pub min_width: GridLength,
    pub max_width: Option<GridLength>,
    pub compare_ascending: Option<Comparison<M>>,
    pub compare_descending: Option<Comparison<M>>,
    pub begin_edit_gestures: BeginEditGestures,
}

impl<M: 'static> Default for ColumnCreateOptions<M> {
    fn default() -> Self {
        Self {
            width: GridLength::AUTO,
            can_user_resize: None,
            can_user_sort_column: None,
            allow_tri_state_sorting: false,
            min_width: GridLength::new(30.0, GridUnitType::Pixel),
            max_width: None,
            compare_ascending: None,
            compare_descending: None,
            begin_edit_gestures: BeginEditGestures::DEFAULT,
        }
    }
}

macro_rules! deref_to_common {
    ($ty:ident) => {
        impl<M: 'static> Deref for $ty<M> {
            type Target = ColumnCreateOptions<M>;

            fn deref(&self) -> &ColumnCreateOptions<M> {
                &self.common
            }
        }

        impl<M: 'static> DerefMut for $ty<M> {
            fn deref_mut(&mut self) -> &mut ColumnCreateOptions<M> {
                &mut self.common
            }
        }
    };
}

/// The options of a text column created by the helpers (`TextColumnCreateOptions<TModel>`).
pub struct TextColumnCreateOptions<M: 'static> {
    pub common: ColumnCreateOptions<M>,
    pub is_read_only: bool,
    pub is_text_search_enabled: bool,
    pub string_format: String,
    pub culture: Option<CultureInfo>,
    pub text_alignment: TextAlignment,
    pub text_trimming: Rc<dyn TextTrimming>,
    pub text_wrapping: TextWrapping,
}

impl<M: 'static> Default for TextColumnCreateOptions<M> {
    fn default() -> Self {
        Self {
            common: ColumnCreateOptions::default(),
            is_read_only: false,
            is_text_search_enabled: true,
            string_format: "{0}".to_string(),
            culture: None,
            text_alignment: TextAlignment::Left,
            text_trimming: <dyn TextTrimming>::character_ellipsis(),
            text_wrapping: TextWrapping::NoWrap,
        }
    }
}

deref_to_common!(TextColumnCreateOptions);

/// The options of a check box column created by the helpers (`CheckBoxColumnCreateOptions<TModel>`).
pub struct CheckBoxColumnCreateOptions<M: 'static> {
    pub common: ColumnCreateOptions<M>,
    pub is_read_only: bool,
}

impl<M: 'static> Default for CheckBoxColumnCreateOptions<M> {
    fn default() -> Self {
        Self { common: ColumnCreateOptions::default(), is_read_only: false }
    }
}

deref_to_common!(CheckBoxColumnCreateOptions);

/// The options of a template column created by the helpers (`TemplateColumnCreateOptions<TModel>`).
pub struct TemplateColumnCreateOptions<M: 'static> {
    pub common: ColumnCreateOptions<M>,
    /// The binding of the text a type-ahead search reads.
    pub text_search_binding: Option<Rc<dyn BindingBase>>,
}

impl<M: 'static> Default for TemplateColumnCreateOptions<M> {
    fn default() -> Self {
        Self { common: ColumnCreateOptions::default(), text_search_binding: None }
    }
}

deref_to_common!(TemplateColumnCreateOptions);

/// The options of a hierarchical expander column created by the helpers
/// (`HierarchicalExpanderColumnCreateOptions<TModel>`).
pub struct HierarchicalExpanderColumnCreateOptions<M: 'static> {
    pub common: ColumnCreateOptions<M>,
    /// Whether a model has children.
    pub has_children: Option<Accessor<M, bool>>,
    /// Whether a model's row is expanded.
    pub is_expanded: Option<Accessor<M, bool>>,
}

impl<M: 'static> Default for HierarchicalExpanderColumnCreateOptions<M> {
    fn default() -> Self {
        Self { common: ColumnCreateOptions::default(), has_children: None, is_expanded: None }
    }
}

deref_to_common!(HierarchicalExpanderColumnCreateOptions);

/// `ApplyCommonOptions`.
fn apply_common_options<M: 'static>(target: &ColumnOptions<M>, source: &ColumnCreateOptions<M>) {
    target.set_can_user_resize_column(source.can_user_resize);
    target.set_can_user_sort_column(source.can_user_sort_column);
    target.set_min_width(source.min_width);
    target.set_max_width(source.max_width);
    target.set_begin_edit_gestures(source.begin_edit_gestures);
    target.set_compare_ascending(source.compare_ascending.clone());
    target.set_compare_descending(source.compare_descending.clone());
}

/// The accessor without its setter (a column created read only).
fn read_only<M: 'static, V: 'static>(accessor: &Accessor<M, V>) -> Accessor<M, V> {
    let getter = accessor.getter().clone();
    let mut result = Accessor::new(move |model: &Rc<M>| getter(model));
    if let Some(name) = accessor.property_name() {
        result = result.named(name);
    }
    for link in accessor.links() {
        let owner = link.owner.clone();
        result = result.through(move |model: &Rc<M>| owner(model), link.property);
    }
    result
}

/// The header of a column: the given header, or the name of the member the accessor reads.
fn header_or_member<M: 'static, V: 'static>(header: Option<BoxedValue>, accessor: &Accessor<M, V>) -> Option<BoxedValue> {
    header.or_else(|| Some(Rc::new(accessor.property_name().unwrap_or_default().to_string()) as BoxedValue))
}

fn add_row_header_column<M: TreeModel>(
    columns: &ColumnList<M>,
    header: Option<BoxedValue>,
    configure: impl FnOnce(&mut ColumnCreateOptions<M>),
) {
    let mut options = ColumnCreateOptions::<M>::default();
    configure(&mut options);
    let internal_options = ColumnOptions::<M>::new();
    apply_common_options(&internal_options, &options);
    columns.add(TreeDataGridRowHeaderColumnInternal::new(header, Some(options.width), Some(Rc::new(internal_options))) as ColumnRef<M>);
}

fn add_text_column<M: TreeModel, V: Clone + PartialEq + PartialOrd + 'static>(
    columns: &ColumnList<M>,
    header: Option<BoxedValue>,
    getter: Accessor<M, V>,
    configure: impl FnOnce(&mut TextColumnCreateOptions<M>),
) {
    let mut options = TextColumnCreateOptions::<M>::default();
    configure(&mut options);
    let header = header_or_member(header, &getter);

    let internal_options = TextColumnOptions::<M>::new();
    internal_options.set_is_text_search_enabled(options.is_text_search_enabled);
    internal_options.set_string_format(options.string_format.clone());
    internal_options.set_text_alignment(options.text_alignment);
    internal_options.set_text_trimming(options.text_trimming.clone());
    internal_options.set_text_wrapping(options.text_wrapping);

    if let Some(culture) = &options.culture {
        internal_options.set_culture(culture.clone());
    }

    apply_common_options(&internal_options, &options);

    let accessor = if options.is_read_only { read_only(&getter) } else { getter };
    columns.add(TextColumn::new(header, accessor, Some(options.width), Some(Rc::new(internal_options))) as ColumnRef<M>);
}

fn add_check_box_column<M: TreeModel>(
    columns: &ColumnList<M>,
    header: Option<BoxedValue>,
    getter: Accessor<M, bool>,
    configure: impl FnOnce(&mut CheckBoxColumnCreateOptions<M>),
) {
    let mut options = CheckBoxColumnCreateOptions::<M>::default();
    configure(&mut options);
    let header = header_or_member(header, &getter);

    let internal_options = CheckBoxColumnOptions::<M>::new();
    apply_common_options(&internal_options, &options);

    let accessor = if options.is_read_only { read_only(&getter) } else { getter };
    columns.add(CheckBoxColumn::new(header, accessor, Some(options.width), Some(Rc::new(internal_options))) as ColumnRef<M>);
}

fn add_three_state_check_box_column<M: TreeModel>(
    columns: &ColumnList<M>,
    header: Option<BoxedValue>,
    getter: Accessor<M, Option<bool>>,
    configure: impl FnOnce(&mut CheckBoxColumnCreateOptions<M>),
) {
    let mut options = CheckBoxColumnCreateOptions::<M>::default();
    configure(&mut options);
    let header = header_or_member(header, &getter);

    let internal_options = CheckBoxColumnOptions::<M>::new();
    apply_common_options(&internal_options, &options);

    let accessor = if options.is_read_only { read_only(&getter) } else { getter };
    columns.add(
        CheckBoxColumn::new_three_state(header, accessor, Some(options.width), Some(Rc::new(internal_options))) as ColumnRef<M>,
    );
}

fn template_options<M: TreeModel>(options: &TemplateColumnCreateOptions<M>) -> TemplateColumnOptions<M> {
    let internal_options = TemplateColumnOptions::<M>::new();
    apply_common_options(&internal_options, options);
    let selector = TreeDataGridBindingAccessor::try_create_text_selector(options.text_search_binding.as_ref());
    internal_options.set_text_search_value_selector(selector.map(|selector| {
        Rc::new(move |model: &Rc<M>| selector(&(model.clone() as BoxedValue))) as Rc<dyn Fn(&Rc<M>) -> Option<String>>
    }));
    internal_options
}

fn add_template_column<M: TreeModel>(
    columns: &ColumnList<M>,
    header: Option<BoxedValue>,
    cell_template: Rc<dyn IDataTemplate>,
    cell_editing_template: Option<Rc<dyn IDataTemplate>>,
    configure: impl FnOnce(&mut TemplateColumnCreateOptions<M>),
) {
    let mut options = TemplateColumnCreateOptions::<M>::default();
    configure(&mut options);
    let internal_options = template_options(&options);
    columns.add(TemplateColumn::new(header, cell_template, cell_editing_template, Some(options.width), Some(Rc::new(internal_options)))
        as ColumnRef<M>);
}

fn add_template_column_from_resource_keys<M: TreeModel>(
    columns: &ColumnList<M>,
    header: Option<BoxedValue>,
    cell_template_resource_key: ResourceKey,
    cell_editing_template_resource_key: Option<ResourceKey>,
    configure: impl FnOnce(&mut TemplateColumnCreateOptions<M>),
) {
    let mut options = TemplateColumnCreateOptions::<M>::default();
    configure(&mut options);
    let internal_options = template_options(&options);
    columns.add(TemplateColumn::from_resource_keys(
        header,
        cell_template_resource_key,
        cell_editing_template_resource_key,
        Some(options.width),
        Some(Rc::new(internal_options)),
    ) as ColumnRef<M>);
}

fn add_hierarchical_expander_text_column<M: TreeModel, V: Clone + PartialEq + PartialOrd + 'static>(
    columns: &ColumnList<M>,
    header: Option<BoxedValue>,
    getter: Accessor<M, V>,
    child_selector: impl Fn(&Rc<M>) -> Option<Models<M>> + 'static,
    configure: impl FnOnce(&mut HierarchicalExpanderColumnCreateOptions<M>),
) {
    let mut options = HierarchicalExpanderColumnCreateOptions::<M>::default();
    configure(&mut options);
    let inner_options = TextColumnCreateOptions::<M> {
        common: ColumnCreateOptions { width: options.width, ..ColumnCreateOptions::default() },
        ..TextColumnCreateOptions::default()
    };
    let internal_options = TextColumnOptions::<M>::new();
    apply_common_options(&internal_options, &inner_options);
    let inner_header = header_or_member(header, &getter);
    let inner = TextColumn::new(inner_header, getter, Some(options.width), Some(Rc::new(internal_options)));
    columns.add(HierarchicalExpanderColumn::new(
        inner as Rc<dyn IViewColumn<M>>,
        child_selector,
        options.has_children.take(),
        options.is_expanded.take(),
    ) as ColumnRef<M>);
}

/// The fluent helpers of the sources (`TreeDataGridSourceExtensions`): each adds a column of the
/// control crate to the source and returns it.
pub trait TreeDataGridSourceExtensions<M: TreeModel>: Sized {
    /// The columns of the source.
    fn column_list(&self) -> &ColumnList<M>;

    /// Adds a column of row numbers (`WithRowHeaderColumn`).
    fn with_row_header_column(self, header: Option<BoxedValue>, configure: impl FnOnce(&mut ColumnCreateOptions<M>)) -> Self {
        add_row_header_column(self.column_list(), header, configure);
        self
    }

    /// Adds a text column of the value `getter` reads (`WithTextColumn`); without a header, the
    /// header is the name of the member read.
    fn with_text_column<V: Clone + PartialEq + PartialOrd + 'static>(
        self,
        header: Option<BoxedValue>,
        getter: Accessor<M, V>,
        configure: impl FnOnce(&mut TextColumnCreateOptions<M>),
    ) -> Self {
        add_text_column(self.column_list(), header, getter, configure);
        self
    }

    /// Adds a two-state check box column (`WithCheckBoxColumn`).
    fn with_check_box_column(
        self,
        header: Option<BoxedValue>,
        getter: Accessor<M, bool>,
        configure: impl FnOnce(&mut CheckBoxColumnCreateOptions<M>),
    ) -> Self {
        add_check_box_column(self.column_list(), header, getter, configure);
        self
    }

    /// Adds a three-state check box column (`WithThreeStateCheckBoxColumn`).
    fn with_three_state_check_box_column(
        self,
        header: Option<BoxedValue>,
        getter: Accessor<M, Option<bool>>,
        configure: impl FnOnce(&mut CheckBoxColumnCreateOptions<M>),
    ) -> Self {
        add_three_state_check_box_column(self.column_list(), header, getter, configure);
        self
    }

    /// Adds a template column (`WithTemplateColumn`).
    fn with_template_column(
        self,
        header: Option<BoxedValue>,
        cell_template: Rc<dyn IDataTemplate>,
        cell_editing_template: Option<Rc<dyn IDataTemplate>>,
        configure: impl FnOnce(&mut TemplateColumnCreateOptions<M>),
    ) -> Self {
        add_template_column(self.column_list(), header, cell_template, cell_editing_template, configure);
        self
    }

    /// Adds a template column whose templates are resources (`WithTemplateColumnFromResourceKeys`).
    fn with_template_column_from_resource_keys(
        self,
        header: Option<BoxedValue>,
        cell_template_resource_key: ResourceKey,
        cell_editing_template_resource_key: Option<ResourceKey>,
        configure: impl FnOnce(&mut TemplateColumnCreateOptions<M>),
    ) -> Self {
        add_template_column_from_resource_keys(
            self.column_list(),
            header,
            cell_template_resource_key,
            cell_editing_template_resource_key,
            configure,
        );
        self
    }
}

impl<M: TreeModel> TreeDataGridSourceExtensions<M> for Rc<FlatTreeDataGridSource<M>> {
    fn column_list(&self) -> &ColumnList<M> {
        self.columns()
    }
}

impl<M: TreeModel> TreeDataGridSourceExtensions<M> for Rc<HierarchicalTreeDataGridSource<M>> {
    fn column_list(&self) -> &ColumnList<M> {
        self.columns()
    }
}

/// The helpers of hierarchical sources (`WithHierarchicalExpanderTextColumn`,
/// `WithHierarchicalExpanderColumn`).
pub trait HierarchicalTreeDataGridSourceExtensions<M: TreeModel>: TreeDataGridSourceExtensions<M> {
    /// Adds an expander column whose cells show the text of the value `getter` reads, and whose
    /// rows have the children `child_selector` gives (`WithHierarchicalExpanderTextColumn`).
    fn with_hierarchical_expander_text_column<V: Clone + PartialEq + PartialOrd + 'static>(
        self,
        header: Option<BoxedValue>,
        getter: Accessor<M, V>,
        child_selector: impl Fn(&Rc<M>) -> Option<Models<M>> + 'static,
        configure: impl FnOnce(&mut HierarchicalExpanderColumnCreateOptions<M>),
    ) -> Self {
        add_hierarchical_expander_text_column(self.column_list(), header, getter, child_selector, configure);
        self
    }

    /// Adds an expander column whose cells show the declarative template column `inner_column`
    /// (`WithHierarchicalExpanderColumn`).
    fn with_hierarchical_expander_column(
        self,
        header: Option<BoxedValue>,
        inner_column: &TreeDataGridTemplateColumn,
        child_selector: impl Fn(&Rc<M>) -> Option<Models<M>> + 'static,
        configure: impl FnOnce(&mut HierarchicalExpanderColumnCreateOptions<M>),
    ) -> Self {
        let mut options = HierarchicalExpanderColumnCreateOptions::<M>::default();
        configure(&mut options);
        let inner = inner_column.create_typed::<M>();
        apply_common_options(inner.options(), &options);
        if header.is_some() {
            inner.set_header(header);
        }
        self.column_list().add(HierarchicalExpanderColumn::new(
            inner as Rc<dyn IViewColumn<M>>,
            child_selector,
            options.has_children.take(),
            options.is_expanded.take(),
        ) as ColumnRef<M>);
        self
    }
}

impl<M: TreeModel> HierarchicalTreeDataGridSourceExtensions<M> for Rc<HierarchicalTreeDataGridSource<M>> {}
