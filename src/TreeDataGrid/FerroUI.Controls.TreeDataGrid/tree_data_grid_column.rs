//! `TreeDataGridColumn.cs`: the declarative columns of a grid whose rows are the items of its
//! `ItemsSource` (the columns of markup): text, check box, template, row header and hierarchical
//! expander columns, which create the view columns of the source the grid generates.
//!
//! The reference's abstract class and its subclasses are a trait, [`TreeDataGridColumn`], and one
//! type per subclass; the members common to every column are [`TreeDataGridColumnBase`], which
//! each column type dereferences to. The models of a generated source are untyped values, so the
//! view columns are columns of models of type `BoxedValue`.

use crate::models::{
    BeginEditGestures, CheckBoxColumnOptions, ColumnOptions, IViewColumn, ObjectHierarchicalExpanderColumn,
    ReflectionCheckBoxColumn, ReflectionTextColumn, TemplateColumn, TemplateColumnOptions, TextColumnOptions,
    TreeDataGridRowHeaderColumnInternal,
};
use crate::TreeDataGridBindingAccessor;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::BindingBase;
use ferroui_base::media::{TextAlignment, TextTrimming, TextWrapping};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_treedatagrid_core::models::Comparison;
use ferroui_treedatagrid_core::Models;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::ops::Deref;
use std::rc::Rc;

/// A comparison of two untyped models (`Comparison<object?>`).
pub type ObjectComparison = Rc<dyn Fn(Option<&BoxedValue>, Option<&BoxedValue>) -> Ordering>;

/// The comparison of the models of a generated source from a comparison of untyped values.
fn to_model_comparison(comparison: &ObjectComparison) -> Comparison<BoxedValue> {
    let comparison = comparison.clone();
    Rc::new(move |a: &Rc<BoxedValue>, b: &Rc<BoxedValue>| comparison(Some(&**a), Some(&**b)))
}

/// The members every declarative column has (`TreeDataGridColumn`).
pub struct TreeDataGridColumnBase {
    header: RefCell<Option<BoxedValue>>,
    width: Cell<GridLength>,
    can_user_resize: Cell<Option<bool>>,
    can_user_sort_column: Cell<Option<bool>>,
    allow_tri_state_sorting: Cell<bool>,
    min_width: Cell<GridLength>,
    max_width: Cell<Option<GridLength>>,
    compare_ascending: RefCell<Option<ObjectComparison>>,
    compare_descending: RefCell<Option<ObjectComparison>>,
    begin_edit_gestures: Cell<BeginEditGestures>,
}

impl Default for TreeDataGridColumnBase {
    fn default() -> Self {
        Self {
            header: RefCell::new(None),
            width: Cell::new(GridLength::AUTO),
            can_user_resize: Cell::new(None),
            can_user_sort_column: Cell::new(None),
            allow_tri_state_sorting: Cell::new(false),
            min_width: Cell::new(GridLength::new(30.0, GridUnitType::Pixel)),
            max_width: Cell::new(None),
            compare_ascending: RefCell::new(None),
            compare_descending: RefCell::new(None),
            begin_edit_gestures: Cell::new(BeginEditGestures::DEFAULT),
        }
    }
}

impl TreeDataGridColumnBase {
    pub fn header(&self) -> Option<BoxedValue> {
        self.header.borrow().clone()
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        *self.header.borrow_mut() = value;
    }

    /// Sets the header to a text.
    pub fn set_header_text(&self, value: &str) {
        self.set_header(Some(Rc::new(value.to_string()) as BoxedValue));
    }

    pub fn width(&self) -> GridLength {
        self.width.get()
    }

    pub fn set_width(&self, value: GridLength) {
        self.width.set(value);
    }

    pub fn can_user_resize(&self) -> Option<bool> {
        self.can_user_resize.get()
    }

    pub fn set_can_user_resize(&self, value: Option<bool>) {
        self.can_user_resize.set(value);
    }

    pub fn can_user_sort_column(&self) -> Option<bool> {
        self.can_user_sort_column.get()
    }

    pub fn set_can_user_sort_column(&self, value: Option<bool>) {
        self.can_user_sort_column.set(value);
    }

    pub fn allow_tri_state_sorting(&self) -> bool {
        self.allow_tri_state_sorting.get()
    }

    pub fn set_allow_tri_state_sorting(&self, value: bool) {
        self.allow_tri_state_sorting.set(value);
    }

    pub fn min_width(&self) -> GridLength {
        self.min_width.get()
    }

    pub fn set_min_width(&self, value: GridLength) {
        self.min_width.set(value);
    }

    pub fn max_width(&self) -> Option<GridLength> {
        self.max_width.get()
    }

    pub fn set_max_width(&self, value: Option<GridLength>) {
        self.max_width.set(value);
    }

    pub fn compare_ascending(&self) -> Option<ObjectComparison> {
        self.compare_ascending.borrow().clone()
    }

    pub fn set_compare_ascending(&self, value: Option<ObjectComparison>) {
        *self.compare_ascending.borrow_mut() = value;
    }

    pub fn compare_descending(&self) -> Option<ObjectComparison> {
        self.compare_descending.borrow().clone()
    }

    pub fn set_compare_descending(&self, value: Option<ObjectComparison>) {
        *self.compare_descending.borrow_mut() = value;
    }

    pub fn begin_edit_gestures(&self) -> BeginEditGestures {
        self.begin_edit_gestures.get()
    }

    pub fn set_begin_edit_gestures(&self, value: BeginEditGestures) {
        self.begin_edit_gestures.set(value);
    }

    /// Applies the common options to the options of a view column (`CreateCommonOptions` and
    /// the initializers of the typed options).
    fn apply_to(&self, options: &ColumnOptions<BoxedValue>) {
        options.set_can_user_resize_column(self.can_user_resize());
        options.set_can_user_sort_column(self.can_user_sort_column());
        options.set_min_width(self.min_width());
        options.set_max_width(self.max_width());
        options.set_begin_edit_gestures(self.begin_edit_gestures());
        options.set_compare_ascending(self.compare_ascending().as_ref().map(to_model_comparison));
        options.set_compare_descending(self.compare_descending().as_ref().map(to_model_comparison));
    }

    /// The options of a view column (`CreateCommonOptions`).
    pub fn create_common_options(&self) -> ColumnOptions<BoxedValue> {
        let options = ColumnOptions::new();
        self.apply_to(&options);
        options
    }
}

/// A declarative column (`abstract class TreeDataGridColumn`).
pub trait TreeDataGridColumn: Any {
    /// The members every column has.
    fn base(&self) -> &TreeDataGridColumnBase;

    /// The view column of the generated source, of untyped models (`CreateUntypedColumn`).
    ///
    /// # Panics
    /// When the column lacks what it needs (a binding, a template; `InvalidOperationException`).
    fn create_untyped_column(&self) -> Rc<dyn IViewColumn<BoxedValue>>;

    /// Tells the column a model of the items, before the source is generated (`InitializeFromSample`).
    fn initialize_from_sample(&self, _sample_model: Option<&BoxedValue>) {}

    /// Whether the column is a hierarchical expander column, which makes the generated source
    /// hierarchical (`is TreeDataGridHierarchicalExpanderColumn`).
    fn is_hierarchical_expander(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any;
}

impl PartialEq for dyn TreeDataGridColumn {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

macro_rules! deref_to_base {
    ($ty:ty) => {
        impl Deref for $ty {
            type Target = TreeDataGridColumnBase;

            fn deref(&self) -> &TreeDataGridColumnBase {
                &self.base
            }
        }
    };
}

/// A text column (`TreeDataGridTextColumn`): the value its binding selects, as text.
pub struct TreeDataGridTextColumn {
    base: TreeDataGridColumnBase,
    sample_model: RefCell<Option<BoxedValue>>,
    binding: RefCell<Option<Rc<dyn BindingBase>>>,
    is_read_only: Cell<bool>,
    is_text_search_enabled: Cell<bool>,
    string_format: RefCell<String>,
    culture: RefCell<CultureInfo>,
    text_alignment: Cell<TextAlignment>,
    text_trimming: RefCell<Rc<dyn TextTrimming>>,
    text_wrapping: Cell<TextWrapping>,
}

deref_to_base!(TreeDataGridTextColumn);

impl Default for TreeDataGridTextColumn {
    fn default() -> Self {
        Self {
            base: TreeDataGridColumnBase::default(),
            sample_model: RefCell::new(None),
            binding: RefCell::new(None),
            is_read_only: Cell::new(false),
            is_text_search_enabled: Cell::new(true),
            string_format: RefCell::new("{0}".to_string()),
            culture: RefCell::new(CultureInfo::current_culture()),
            text_alignment: Cell::new(TextAlignment::Left),
            text_trimming: RefCell::new(<dyn TextTrimming>::character_ellipsis()),
            text_wrapping: Cell::new(TextWrapping::NoWrap),
        }
    }
}

impl TreeDataGridTextColumn {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The binding of the value (`[AssignBinding] Binding`).
    pub fn binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.binding.borrow().clone()
    }

    pub fn set_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        *self.binding.borrow_mut() = value;
    }

    pub fn is_read_only(&self) -> bool {
        self.is_read_only.get()
    }

    pub fn set_is_read_only(&self, value: bool) {
        self.is_read_only.set(value);
    }

    pub fn is_text_search_enabled(&self) -> bool {
        self.is_text_search_enabled.get()
    }

    pub fn set_is_text_search_enabled(&self, value: bool) {
        self.is_text_search_enabled.set(value);
    }

    pub fn string_format(&self) -> String {
        self.string_format.borrow().clone()
    }

    pub fn set_string_format(&self, value: impl Into<String>) {
        *self.string_format.borrow_mut() = value.into();
    }

    pub fn culture(&self) -> CultureInfo {
        self.culture.borrow().clone()
    }

    pub fn set_culture(&self, value: CultureInfo) {
        *self.culture.borrow_mut() = value;
    }

    pub fn text_alignment(&self) -> TextAlignment {
        self.text_alignment.get()
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.text_alignment.set(value);
    }

    pub fn text_trimming(&self) -> Rc<dyn TextTrimming> {
        self.text_trimming.borrow().clone()
    }

    pub fn set_text_trimming(&self, value: Rc<dyn TextTrimming>) {
        *self.text_trimming.borrow_mut() = value;
    }

    pub fn text_wrapping(&self) -> TextWrapping {
        self.text_wrapping.get()
    }

    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.text_wrapping.set(value);
    }
}

impl TreeDataGridColumn for TreeDataGridTextColumn {
    fn base(&self) -> &TreeDataGridColumnBase {
        &self.base
    }

    fn create_untyped_column(&self) -> Rc<dyn IViewColumn<BoxedValue>> {
        let Some(binding) = self.binding() else {
            panic!("TreeDataGridTextColumn requires Binding.");
        };
        let accessor = TreeDataGridBindingAccessor::new(&binding, self.sample_model.borrow().as_ref());
        let getter = {
            let accessor = accessor.clone();
            Rc::new(move |model: &BoxedValue| accessor.read(model)) as Rc<dyn Fn(&BoxedValue) -> Option<BoxedValue>>
        };
        let setter = if !self.is_read_only() && accessor.can_write() {
            let accessor = accessor.clone();
            Some(Rc::new(move |model: &BoxedValue, value: Option<BoxedValue>| accessor.write(model, value))
                as Rc<dyn Fn(&BoxedValue, Option<BoxedValue>)>)
        } else {
            None
        };

        let options = TextColumnOptions::<BoxedValue>::new();
        self.base.apply_to(&options);
        options.set_is_text_search_enabled(self.is_text_search_enabled());
        options.set_string_format(self.string_format());
        options.set_culture(self.culture());
        options.set_text_alignment(self.text_alignment());
        options.set_text_trimming(self.text_trimming());
        options.set_text_wrapping(self.text_wrapping());

        ReflectionTextColumn::new(self.header(), getter, setter, accessor.links(), self.width(), Rc::new(options))
    }

    fn initialize_from_sample(&self, sample_model: Option<&BoxedValue>) {
        *self.sample_model.borrow_mut() = sample_model.cloned();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A check box column (`TreeDataGridCheckBoxColumn`): the boolean its binding selects.
pub struct TreeDataGridCheckBoxColumn {
    base: TreeDataGridColumnBase,
    sample_model: RefCell<Option<BoxedValue>>,
    binding: RefCell<Option<Rc<dyn BindingBase>>>,
    is_read_only: Cell<bool>,
    is_three_state: Cell<Option<bool>>,
    // Whether the last column created has three states (`_isThreeState`).
    created_three_state: Cell<bool>,
}

deref_to_base!(TreeDataGridCheckBoxColumn);

impl Default for TreeDataGridCheckBoxColumn {
    fn default() -> Self {
        Self {
            base: TreeDataGridColumnBase::default(),
            sample_model: RefCell::new(None),
            binding: RefCell::new(None),
            is_read_only: Cell::new(false),
            is_three_state: Cell::new(None),
            created_three_state: Cell::new(false),
        }
    }
}

impl TreeDataGridCheckBoxColumn {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The binding of the value (`[AssignBinding] Binding`).
    pub fn binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.binding.borrow().clone()
    }

    pub fn set_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        *self.binding.borrow_mut() = value;
    }

    pub fn is_read_only(&self) -> bool {
        self.is_read_only.get()
    }

    pub fn set_is_read_only(&self, value: bool) {
        self.is_read_only.set(value);
    }

    /// Whether the check boxes have three states; `None` infers it from the type of the value or
    /// a null value of the sample model.
    pub fn is_three_state(&self) -> Option<bool> {
        self.is_three_state.get()
    }

    pub fn set_is_three_state(&self, value: Option<bool>) {
        self.is_three_state.set(value);
    }
}

impl TreeDataGridColumn for TreeDataGridCheckBoxColumn {
    fn base(&self) -> &TreeDataGridColumnBase {
        &self.base
    }

    fn create_untyped_column(&self) -> Rc<dyn IViewColumn<BoxedValue>> {
        let Some(binding) = self.binding() else {
            panic!("TreeDataGridCheckBoxColumn requires Binding.");
        };
        let accessor = TreeDataGridBindingAccessor::new(&binding, self.sample_model.borrow().as_ref());
        let getter = {
            let accessor = accessor.clone();
            Rc::new(move |model: &BoxedValue| accessor.read_as_nullable_boolean(model)) as Rc<dyn Fn(&BoxedValue) -> Option<bool>>
        };
        let setter = if !self.is_read_only() && accessor.can_write() {
            let accessor = accessor.clone();
            Some(Rc::new(move |model: &BoxedValue, value: Option<bool>| {
                accessor.write(model, value.map(|value| Rc::new(value) as BoxedValue))
            }) as Rc<dyn Fn(&BoxedValue, Option<bool>)>)
        } else {
            None
        };

        let nullable_type = accessor.value_type().is_some_and(|value_type| {
            value_type.is::<Option<bool>>()
                || ferroui_base::data::core::ValueTypes::nullable_inner(value_type).is_some_and(|inner| inner.is::<bool>())
        });
        let is_three_state = self.is_three_state().unwrap_or(nullable_type || accessor.sample_value_was_null());
        self.created_three_state.set(is_three_state);

        let options = CheckBoxColumnOptions::<BoxedValue>::new();
        self.base.apply_to(&options);

        ReflectionCheckBoxColumn::new(
            self.header(),
            getter,
            setter,
            accessor.links(),
            self.width(),
            Rc::new(options),
            is_three_state,
        )
    }

    fn initialize_from_sample(&self, sample_model: Option<&BoxedValue>) {
        *self.sample_model.borrow_mut() = sample_model.cloned();
        self.created_three_state.set(false);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A template column (`TreeDataGridTemplateColumn`): cells built from a data template, given
/// directly or as resource keys.
pub struct TreeDataGridTemplateColumn {
    base: TreeDataGridColumnBase,
    cell_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    cell_editing_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    cell_template_resource_key: RefCell<Option<ResourceKey>>,
    cell_editing_template_resource_key: RefCell<Option<ResourceKey>>,
    text_search_binding: RefCell<Option<Rc<dyn BindingBase>>>,
}

deref_to_base!(TreeDataGridTemplateColumn);

impl Default for TreeDataGridTemplateColumn {
    fn default() -> Self {
        Self {
            base: TreeDataGridColumnBase::default(),
            cell_template: RefCell::new(None),
            cell_editing_template: RefCell::new(None),
            cell_template_resource_key: RefCell::new(None),
            cell_editing_template_resource_key: RefCell::new(None),
            text_search_binding: RefCell::new(None),
        }
    }
}

impl TreeDataGridTemplateColumn {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// A column of the templates of the resource keys
    /// (`TreeDataGridTemplateColumn(header, cellTemplateResourceKey, cellEditingTemplateResourceKey)`).
    pub fn with_resource_keys(
        header: Option<BoxedValue>,
        cell_template_resource_key: ResourceKey,
        cell_editing_template_resource_key: Option<ResourceKey>,
    ) -> Rc<Self> {
        let this = Self::default();
        this.set_header(header);
        *this.cell_template_resource_key.borrow_mut() = Some(cell_template_resource_key);
        *this.cell_editing_template_resource_key.borrow_mut() = cell_editing_template_resource_key;
        Rc::new(this)
    }

    pub fn cell_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.cell_template.borrow().clone()
    }

    pub fn set_cell_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        *self.cell_template.borrow_mut() = value;
    }

    pub fn cell_editing_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.cell_editing_template.borrow().clone()
    }

    pub fn set_cell_editing_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        *self.cell_editing_template.borrow_mut() = value;
    }

    pub fn cell_template_resource_key(&self) -> Option<ResourceKey> {
        self.cell_template_resource_key.borrow().clone()
    }

    pub fn set_cell_template_resource_key(&self, value: Option<ResourceKey>) {
        *self.cell_template_resource_key.borrow_mut() = value;
    }

    pub fn cell_editing_template_resource_key(&self) -> Option<ResourceKey> {
        self.cell_editing_template_resource_key.borrow().clone()
    }

    pub fn set_cell_editing_template_resource_key(&self, value: Option<ResourceKey>) {
        *self.cell_editing_template_resource_key.borrow_mut() = value;
    }

    /// The binding of the text a type-ahead search reads (`[AssignBinding] TextSearchBinding`).
    pub fn text_search_binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.text_search_binding.borrow().clone()
    }

    pub fn set_text_search_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        *self.text_search_binding.borrow_mut() = value;
    }

    /// The options of the typed template column of models of `M`, whose text search reads the
    /// text search binding through `to_untyped` (`CreateTyped<TModel>`'s options).
    fn create_options<M: ferroui_treedatagrid_core::accessor::TreeModel>(
        &self,
        to_untyped: fn(&Rc<M>) -> BoxedValue,
    ) -> TemplateColumnOptions<M> {
        let options = TemplateColumnOptions::<M>::new();
        options.set_can_user_resize_column(self.can_user_resize());
        options.set_can_user_sort_column(self.can_user_sort_column());
        options.set_min_width(self.min_width());
        options.set_max_width(self.max_width());
        options.set_begin_edit_gestures(self.begin_edit_gestures());
        if let Some(compare) = self.compare_ascending() {
            options.set_compare_ascending(Some(Rc::new(move |a: &Rc<M>, b: &Rc<M>| {
                compare(Some(&to_untyped(a)), Some(&to_untyped(b)))
            })));
        }
        if let Some(compare) = self.compare_descending() {
            options.set_compare_descending(Some(Rc::new(move |a: &Rc<M>, b: &Rc<M>| {
                compare(Some(&to_untyped(a)), Some(&to_untyped(b)))
            })));
        }
        let selector = TreeDataGridBindingAccessor::try_create_text_selector(self.text_search_binding().as_ref());
        options.set_is_text_search_enabled(selector.is_some());
        options.set_text_search_value_selector(
            selector.map(|selector| Rc::new(move |model: &Rc<M>| selector(&to_untyped(model))) as Rc<dyn Fn(&Rc<M>) -> Option<String>>),
        );
        options
    }

    /// The template column of a typed source of models of `M` (`CreateTyped<TModel>`).
    ///
    /// # Panics
    /// When the column has neither a template nor a template resource key.
    pub fn create_typed<M: ferroui_treedatagrid_core::accessor::TreeModel>(&self) -> Rc<TemplateColumn<M>> {
        let options = Rc::new(self.create_options::<M>(|model| model.clone() as BoxedValue));
        if let Some(template) = self.cell_template() {
            return TemplateColumn::new(self.header(), template, self.cell_editing_template(), Some(self.width()), Some(options));
        }
        if let Some(key) = self.cell_template_resource_key() {
            return TemplateColumn::from_resource_keys(
                self.header(),
                key,
                self.cell_editing_template_resource_key(),
                Some(self.width()),
                Some(options),
            );
        }
        panic!("TreeDataGridTemplateColumn requires CellTemplate or CellTemplateResourceKey.");
    }
}

impl TreeDataGridColumn for TreeDataGridTemplateColumn {
    fn base(&self) -> &TreeDataGridColumnBase {
        &self.base
    }

    fn create_untyped_column(&self) -> Rc<dyn IViewColumn<BoxedValue>> {
        // The models of a generated source are the untyped items themselves.
        let options = Rc::new(self.create_options::<BoxedValue>(|model| (**model).clone()));
        if let Some(template) = self.cell_template() {
            return TemplateColumn::new(self.header(), template, self.cell_editing_template(), Some(self.width()), Some(options));
        }
        if let Some(key) = self.cell_template_resource_key() {
            return TemplateColumn::from_resource_keys(
                self.header(),
                key,
                self.cell_editing_template_resource_key(),
                Some(self.width()),
                Some(options),
            );
        }
        panic!("TreeDataGridTemplateColumn requires CellTemplate or CellTemplateResourceKey.");
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A column of row headers (`TreeDataGridRowHeaderColumn`): the number of each row.
#[derive(Default)]
pub struct TreeDataGridRowHeaderColumn {
    base: TreeDataGridColumnBase,
}

deref_to_base!(TreeDataGridRowHeaderColumn);

impl TreeDataGridRowHeaderColumn {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}

impl TreeDataGridColumn for TreeDataGridRowHeaderColumn {
    fn base(&self) -> &TreeDataGridColumnBase {
        &self.base
    }

    fn create_untyped_column(&self) -> Rc<dyn IViewColumn<BoxedValue>> {
        let options = Rc::new(self.base.create_common_options());
        TreeDataGridRowHeaderColumnInternal::<BoxedValue>::new(self.header(), Some(self.width()), Some(options))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A hierarchical expander column (`TreeDataGridHierarchicalExpanderColumn`): its inner column
/// with an expander whose children, whether there are any and whether the row is expanded are
/// selected by bindings. Its presence makes the generated source hierarchical.
#[derive(Default)]
pub struct TreeDataGridHierarchicalExpanderColumn {
    base: TreeDataGridColumnBase,
    sample_model: RefCell<Option<BoxedValue>>,
    children_binding: RefCell<Option<Rc<dyn BindingBase>>>,
    has_children_binding: RefCell<Option<Rc<dyn BindingBase>>>,
    is_expanded_binding: RefCell<Option<Rc<dyn BindingBase>>>,
    inner_column: RefCell<Option<Rc<dyn TreeDataGridColumn>>>,
}

deref_to_base!(TreeDataGridHierarchicalExpanderColumn);

impl TreeDataGridHierarchicalExpanderColumn {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The binding of the children of a model (`[AssignBinding] ChildrenBinding`).
    pub fn children_binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.children_binding.borrow().clone()
    }

    pub fn set_children_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        *self.children_binding.borrow_mut() = value;
    }

    /// The binding of whether a model has children (`[AssignBinding] HasChildrenBinding`).
    pub fn has_children_binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.has_children_binding.borrow().clone()
    }

    pub fn set_has_children_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        *self.has_children_binding.borrow_mut() = value;
    }

    /// The binding of whether a model's row is expanded (`[AssignBinding] IsExpandedBinding`).
    pub fn is_expanded_binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.is_expanded_binding.borrow().clone()
    }

    pub fn set_is_expanded_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        *self.is_expanded_binding.borrow_mut() = value;
    }

    /// The column the expander shows (`[Content] InnerColumn`).
    pub fn inner_column(&self) -> Option<Rc<dyn TreeDataGridColumn>> {
        self.inner_column.borrow().clone()
    }

    pub fn set_inner_column(&self, value: Option<Rc<dyn TreeDataGridColumn>>) {
        *self.inner_column.borrow_mut() = value;
    }
}

/// The children a binding selected, as the models of a generated source: a collection of the
/// framework's items, read item by item (`items.Cast<object>()`).
fn children_models(value: Option<BoxedValue>) -> Option<Models<BoxedValue>> {
    let value = value?;
    if let Some(models) = value.downcast_ref::<Models<BoxedValue>>() {
        return Some(models.clone());
    }
    let items = match value.downcast_ref::<ferroui_controls::ItemsSource>() {
        Some(items) => items.clone(),
        None => ferroui_base::data::core::ValueTypes::try_convert(
            Some(&value),
            ferroui_base::data::core::ValueType::of::<ferroui_controls::ItemsSource>(),
        )
        .flatten()
        .and_then(|converted| converted.downcast_ref::<ferroui_controls::ItemsSource>().cloned())?,
    };
    Some(crate::ItemsSourceModels::new(items) as Models<BoxedValue>)
}

impl TreeDataGridColumn for TreeDataGridHierarchicalExpanderColumn {
    fn base(&self) -> &TreeDataGridColumnBase {
        &self.base
    }

    fn create_untyped_column(&self) -> Rc<dyn IViewColumn<BoxedValue>> {
        let sample = self.sample_model.borrow().clone();
        let Some(children_binding) = self.children_binding() else {
            panic!("TreeDataGridHierarchicalExpanderColumn requires ChildrenBinding.");
        };
        let children_accessor = TreeDataGridBindingAccessor::new(&children_binding, sample.as_ref());
        let has_children_accessor =
            self.has_children_binding().map(|binding| TreeDataGridBindingAccessor::new(&binding, sample.as_ref()));
        let is_expanded_accessor =
            self.is_expanded_binding().map(|binding| TreeDataGridBindingAccessor::new(&binding, sample.as_ref()));
        let Some(inner) = self.inner_column().map(|inner| inner.create_untyped_column()) else {
            panic!("TreeDataGridHierarchicalExpanderColumn requires an inner column.");
        };

        let has_children_links = has_children_accessor.as_ref().map(|accessor| accessor.links());
        let is_expanded_links = is_expanded_accessor.as_ref().map(|accessor| accessor.links());
        let has_children = has_children_accessor.map(|accessor| {
            Rc::new(move |model: &BoxedValue| accessor.read_as_boolean(model)) as Rc<dyn Fn(&BoxedValue) -> bool>
        });
        let is_expanded_setter = is_expanded_accessor.as_ref().filter(|accessor| accessor.can_write()).map(|accessor| {
            let accessor = accessor.clone();
            Rc::new(move |model: &BoxedValue, value: bool| accessor.write(model, Some(Rc::new(value) as BoxedValue)))
                as Rc<dyn Fn(&BoxedValue, bool)>
        });
        let is_expanded = is_expanded_accessor.map(|accessor| {
            Rc::new(move |model: &BoxedValue| accessor.read_as_boolean(model)) as Rc<dyn Fn(&BoxedValue) -> bool>
        });

        ObjectHierarchicalExpanderColumn::new(
            inner,
            move |model: &BoxedValue| children_models(children_accessor.read(model)),
            has_children,
            is_expanded,
            is_expanded_setter,
            has_children_links,
            is_expanded_links,
        )
    }

    fn initialize_from_sample(&self, sample_model: Option<&BoxedValue>) {
        *self.sample_model.borrow_mut() = sample_model.cloned();
        if let Some(inner) = self.inner_column() {
            inner.initialize_from_sample(sample_model);
        }
    }

    fn is_hierarchical_expander(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
