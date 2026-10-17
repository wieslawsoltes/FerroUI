//! `Models/Columns.cs` and `Models/IExpanderColumn.cs`: the neutral columns of a source.

use super::{IExpanderRow, NotifyingBase, NotifyingListBase};
use crate::accessor::{Accessor, PropertyPathObserver, TreeModel};
use crate::model_collection::Models;
use crate::GridLength;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

/// `System.ComponentModel.ListSortDirection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListSortDirection {
    Ascending,
    Descending,
}

/// A comparison of two models (`Comparison<TModel?>`).
pub type Comparison<M> = Rc<dyn Fn(&Rc<M>, &Rc<M>) -> Ordering>;

/// A column of a source (`IColumn`).
pub trait IColumn: INotifyPropertyChanged + Any {
    fn id(&self) -> String;
    fn header(&self) -> Option<BoxedValue>;
    fn width(&self) -> GridLength;
    fn set_width(&self, value: GridLength);
    fn is_visible(&self) -> bool;
    fn set_is_visible(&self, value: bool);
    /// The key that selects how the control presents the column.
    fn presentation_key(&self) -> Option<String>;
    fn set_presentation_key(&self, value: Option<String>);
    fn sort_direction(&self) -> Option<ListSortDirection>;
    fn set_sort_direction(&self, value: Option<ListSortDirection>);
    fn tag(&self) -> Option<BoxedValue>;
    fn set_tag(&self, value: Option<BoxedValue>);
    fn as_any(&self) -> &dyn Any;
    /// The model type of the column (`is IColumn<TModel>`).
    fn model_type(&self) -> std::any::TypeId;

    /// The view of a column that presents itself (a column of the control crate, which creates its
    /// cells), as a handle the control downcasts; `None` for a neutral column, which the control
    /// presents through its presentation options.
    fn view(&self) -> Option<Rc<dyn Any>> {
        None
    }

    /// The erased access to the value of a value column, for a presentation that holds models as
    /// untyped values.
    fn untyped_value(&self) -> Option<&dyn IUntypedValueColumn> {
        None
    }

    /// The erased access to an expander column.
    fn untyped_expander(&self) -> Option<&dyn IUntypedExpanderColumn> {
        None
    }
}

/// A subscription of [`IUntypedValueColumn::observe`]: dropping it ends it.
pub type UntypedSubscription = Box<dyn Any>;

/// The value of a [`ValueColumn`] read and written through untyped models.
pub trait IUntypedValueColumn {
    /// The value of `model` (`None` when the model is not of the column's model type).
    fn get_value(&self, model: &BoxedValue) -> Option<BoxedValue>;
    /// Writes a value; false when the column is read only or a type does not match.
    fn set_value(&self, model: &BoxedValue, value: Option<&BoxedValue>) -> bool;
    fn can_set(&self) -> bool;
    /// The member the value is read from.
    fn property_name(&self) -> Option<&'static str>;
    /// Observes the objects below the model on the value's path; `None` when the value reads only
    /// members of the model.
    fn observe(&self, model: &BoxedValue, changed: Rc<dyn Fn()>) -> Option<UntypedSubscription>;
    /// Whether the column is a template column.
    fn is_template(&self) -> bool;
    /// Whether a check box column has three states.
    fn is_three_state(&self) -> bool;
    /// The column's options as untyped values: (can user resize, can user sort, min width, max width).
    fn layout_options(&self) -> (Option<bool>, Option<bool>, GridLength, Option<GridLength>);
}

/// An expander column read through untyped models.
pub trait IUntypedExpanderColumn {
    /// The column the expander shows.
    fn inner(&self) -> Rc<dyn IColumn>;
    fn has_children(&self, model: &BoxedValue) -> bool;
    fn get_model_is_expanded(&self, model: &BoxedValue) -> Option<bool>;
}

fn typed_model<M: 'static>(model: &BoxedValue) -> Option<Rc<M>> {
    let any: Rc<dyn Any> = model.clone();
    any.downcast::<M>().ok()
}

/// What a column of models of `M` is, the dispatch of the reference's `IColumnVisitor`
/// (`Accept<TResult>`), whose generic method cannot be called through a trait object.
pub enum ColumnKind<'a, M: 'static> {
    /// A [`ValueColumn`] (and its text, check box and template forms), with its value erased.
    Value(&'a dyn IValueColumn<M>),
    /// A [`HierarchicalExpanderColumn`].
    HierarchicalExpander(&'a HierarchicalExpanderColumn<M>),
}

/// A column of models of `M` (`IColumn<TModel>`).
pub trait IColumnOf<M: 'static>: IColumn {
    /// The comparison that sorts the models by the column, or `None` when the column does not sort.
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>>;

    /// What the column is (`Accept`).
    fn kind(&self) -> ColumnKind<'_, M>;

    /// The column as an expander column (`as IExpanderColumn<TModel>`).
    fn as_expander_column(&self) -> Option<&dyn IExpanderColumn<M>>
    where
        M: TreeModel,
    {
        None
    }

    /// Whether the column is an expander column (`is IExpanderColumn<TModel>`).
    fn as_expander_column_any(&self) -> bool {
        false
    }
}

/// A shared column.
pub type ColumnRef<M> = Rc<dyn IColumnOf<M>>;

/// A [`ValueColumn`] with its value type erased: what the presentation reads.
pub trait IValueColumn<M: 'static>: IColumnOf<M> {
    /// The value of `model`, boxed.
    fn get_boxed_value(&self, model: &Rc<M>) -> Option<BoxedValue>;
    /// Writes a boxed value; false when the column is read only or the value has another type.
    fn set_boxed_value(&self, model: &Rc<M>, value: Option<&BoxedValue>) -> bool;
    /// Whether the column has a setter.
    fn can_set(&self) -> bool;
    /// The member the value is read from, for change notifications.
    fn property_name(&self) -> Option<&'static str>;
    /// The observer of the path of the value below the model, if it reads below it.
    fn value_type_name(&self) -> &'static str;
    /// The column as `Any`, to reach the typed [`ValueColumn`].
    fn as_value_any(&self) -> &dyn Any;
}

/// `ColumnOptions<TModel>`.
pub struct ColumnOptions<M: 'static> {
    pub can_user_resize_column: Option<bool>,
    pub can_user_sort_column: Option<bool>,
    pub min_width: GridLength,
    pub max_width: Option<GridLength>,
    pub compare_ascending: Option<Comparison<M>>,
    pub compare_descending: Option<Comparison<M>>,
}

impl<M: 'static> Default for ColumnOptions<M> {
    fn default() -> Self {
        Self {
            can_user_resize_column: None,
            can_user_sort_column: None,
            min_width: GridLength::pixels(30.0),
            max_width: None,
            compare_ascending: None,
            compare_descending: None,
        }
    }
}

impl<M: 'static> Clone for ColumnOptions<M> {
    fn clone(&self) -> Self {
        Self {
            can_user_resize_column: self.can_user_resize_column,
            can_user_sort_column: self.can_user_sort_column,
            min_width: self.min_width,
            max_width: self.max_width,
            compare_ascending: self.compare_ascending.clone(),
            compare_descending: self.compare_descending.clone(),
        }
    }
}

const NOT_UNIQUE: &str = "A column instance can only appear once.";

fn same_column<M: 'static>(a: &ColumnRef<M>, b: &ColumnRef<M>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `ColumnList<TModel>`: the columns of a source, each instance at most once.
pub struct ColumnList<M: 'static> {
    list: NotifyingListBase<ColumnRef<M>>,
    // The columns of a hierarchical source (`HierarchicalColumnList`): one expander column, which
    // can be neither removed nor replaced.
    hierarchical: bool,
    applying_reset: Cell<bool>,
}

impl<M: 'static> Default for ColumnList<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> ColumnList<M> {
    pub fn new() -> Self {
        Self { list: NotifyingListBase::new(), hierarchical: false, applying_reset: Cell::new(false) }
    }

    /// The columns of a hierarchical source (`HierarchicalColumnList`): at most one expander
    /// column, which can be neither removed nor replaced once it is added.
    pub fn new_hierarchical() -> Self {
        Self { list: NotifyingListBase::new(), hierarchical: true, applying_reset: Cell::new(false) }
    }

    fn is_expander(column: &ColumnRef<M>) -> bool {
        column.as_expander_column_any()
    }

    fn expander_count(&self) -> usize {
        self.list.with_items(|items| items.iter().filter(|c| Self::is_expander(c)).count())
    }

    /// The notifying list (its `CollectionChanged` and `PropertyChanged`).
    pub fn list(&self) -> &NotifyingListBase<ColumnRef<M>> {
        &self.list
    }

    pub fn count(&self) -> usize {
        self.list.count()
    }

    pub fn get(&self, index: usize) -> ColumnRef<M> {
        self.list.get(index)
    }

    pub fn to_vec(&self) -> Vec<ColumnRef<M>> {
        self.list.to_vec()
    }

    /// The index of `column`, compared by identity.
    pub fn index_of(&self, column: &ColumnRef<M>) -> Option<usize> {
        self.list.position(|c| same_column(c, column))
    }

    /// The index of a column given as an untyped column.
    pub fn index_of_untyped(&self, column: &dyn IColumn) -> Option<usize> {
        self.list.position(|c| std::ptr::addr_eq(Rc::as_ptr(c) as *const dyn IColumn, column as *const dyn IColumn))
    }

    fn contains_reference(&self, column: &ColumnRef<M>) -> bool {
        self.index_of(column).is_some()
    }

    pub fn add(&self, column: ColumnRef<M>) {
        self.insert(self.count(), column);
    }

    /// Inserts a column.
    ///
    /// # Panics
    /// When the column is in the list already.
    pub fn insert(&self, index: usize, column: ColumnRef<M>) {
        if self.contains_reference(&column) {
            panic!("{NOT_UNIQUE}");
        }
        if self.hierarchical && !self.applying_reset.get() && Self::is_expander(&column) && self.expander_count() > 0 {
            panic!("Only one expander column is allowed.");
        }
        self.list.insert(index, column);
    }

    /// Replaces the column at `index`.
    ///
    /// # Panics
    /// When the column is elsewhere in the list.
    pub fn set(&self, index: usize, column: ColumnRef<M>) {
        if self.hierarchical && !self.applying_reset.get() && (Self::is_expander(&self.get(index)) || Self::is_expander(&column)) {
            panic!("The expander column cannot be replaced.");
        }
        for i in 0..self.count() {
            if i != index && same_column(&self.get(i), &column) {
                panic!("{NOT_UNIQUE}");
            }
        }
        self.list.set(index, column);
    }

    pub fn remove_at(&self, index: usize) {
        if self.hierarchical && !self.applying_reset.get() && Self::is_expander(&self.get(index)) {
            panic!("The expander column cannot be removed.");
        }
        self.list.remove_at(index);
    }

    /// Removes `column`; false when it is not in the list.
    pub fn remove(&self, column: &ColumnRef<M>) -> bool {
        match self.index_of(column) {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    pub fn clear(&self) {
        if self.hierarchical && !self.applying_reset.get() && self.expander_count() > 0 {
            panic!("The expander column cannot be removed.");
        }
        self.list.clear();
    }

    pub fn remove_range(&self, index: usize, count: usize) {
        if self.hierarchical {
            for i in 0..count {
                if Self::is_expander(&self.get(index + i)) {
                    panic!("The expander column cannot be removed.");
                }
            }
        }
        self.list.remove_range(index, count);
    }

    /// Inserts the columns `action` hands over, with one notification.
    ///
    /// # Panics
    /// When a column appears twice or is in the list already.
    pub fn insert_range(&self, index: usize, action: impl FnOnce(&mut dyn FnMut(ColumnRef<M>))) {
        let mut proposed = Vec::new();
        action(&mut |column| proposed.push(column));
        if self.hierarchical && self.expander_count() + proposed.iter().filter(|c| Self::is_expander(c)).count() > 1 {
            panic!("Only one expander column is allowed.");
        }
        self.validate_unique(&proposed, true);
        self.list.insert_range(index, |add| {
            for column in proposed {
                add(column);
            }
        });
    }

    /// Replaces the columns with the list `action` leaves, with one reset notification.
    ///
    /// # Panics
    /// When a column appears twice.
    pub fn reset(&self, action: impl FnOnce(&mut Vec<ColumnRef<M>>)) {
        let current_expander = self.list.with_items(|items| items.iter().find(|c| Self::is_expander(c)).cloned());
        let mut proposed = self.to_vec();
        action(&mut proposed);
        if self.hierarchical {
            let proposed_expanders: Vec<&ColumnRef<M>> = proposed.iter().filter(|c| Self::is_expander(c)).collect();
            if proposed_expanders.len() > 1
                || current_expander
                    .as_ref()
                    .is_some_and(|current| proposed_expanders.first().is_none_or(|p| !same_column(current, p)))
            {
                panic!("A reset cannot remove or replace the expander column.");
            }
        }
        self.validate_unique(&proposed, false);
        self.applying_reset.set(true);
        self.list.reset(|columns| {
            columns.clear();
            for column in proposed {
                columns.add(column);
            }
        });
        self.applying_reset.set(false);
    }

    /// Sets the width of the column at `column_index`.
    pub fn set_column_width(&self, column_index: usize, width: GridLength) {
        self.get(column_index).set_width(width);
    }

    /// Moves a column.
    ///
    /// # Panics
    /// When an index is out of range.
    pub fn move_column(&self, old_index: usize, new_index: usize) {
        if old_index >= self.count() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'oldIndex')");
        }
        if new_index >= self.count() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'newIndex')");
        }
        if old_index != new_index {
            self.list.move_item(old_index, new_index);
        }
    }

    fn validate_unique(&self, proposed: &[ColumnRef<M>], include_existing: bool) {
        for i in 0..proposed.len() {
            if include_existing && self.contains_reference(&proposed[i]) {
                panic!("{NOT_UNIQUE}");
            }
            for j in 0..i {
                if same_column(&proposed[i], &proposed[j]) {
                    panic!("{NOT_UNIQUE}");
                }
            }
        }
    }
}

/// The state every column has (`ColumnBase<TModel>`), held by the column types.
pub struct ColumnBase<M: 'static> {
    notifying: NotifyingBase,
    id: String,
    header: Option<BoxedValue>,
    options: ColumnOptions<M>,
    width: Cell<GridLength>,
    is_visible: Cell<bool>,
    presentation_key: RefCell<Option<String>>,
    sort_direction: Cell<Option<ListSortDirection>>,
    tag: RefCell<Option<BoxedValue>>,
}

impl<M: 'static> ColumnBase<M> {
    pub fn new(header: Option<BoxedValue>, width: Option<GridLength>, options: Option<ColumnOptions<M>>, id: Option<String>) -> Self {
        let id = id.unwrap_or_else(|| header.as_ref().map(|h| ValueTypes::to_display_string(Some(h))).unwrap_or_default());
        Self {
            notifying: NotifyingBase::new(),
            id,
            header,
            options: options.unwrap_or_default(),
            width: Cell::new(width.unwrap_or(GridLength::AUTO)),
            is_visible: Cell::new(true),
            presentation_key: RefCell::new(None),
            sort_direction: Cell::new(None),
            tag: RefCell::new(None),
        }
    }

    pub fn options(&self) -> &ColumnOptions<M> {
        &self.options
    }

    pub fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn header(&self) -> Option<BoxedValue> {
        self.header.clone()
    }

    pub fn width(&self) -> GridLength {
        self.width.get()
    }

    pub fn set_width(&self, value: GridLength) {
        self.notifying.raise_and_set_cell_if_changed(&self.width, value, "Width");
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible.get()
    }

    pub fn set_is_visible(&self, value: bool) {
        self.notifying.raise_and_set_cell_if_changed(&self.is_visible, value, "IsVisible");
    }

    pub fn presentation_key(&self) -> Option<String> {
        self.presentation_key.borrow().clone()
    }

    pub fn set_presentation_key(&self, value: Option<String>) {
        self.notifying.raise_and_set_if_changed(&self.presentation_key, value, "PresentationKey");
    }

    pub fn sort_direction(&self) -> Option<ListSortDirection> {
        self.sort_direction.get()
    }

    pub fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.notifying.raise_and_set_cell_if_changed(&self.sort_direction, value, "SortDirection");
    }

    pub fn tag(&self) -> Option<BoxedValue> {
        self.tag.borrow().clone()
    }

    pub fn set_tag(&self, value: Option<BoxedValue>) {
        *self.tag.borrow_mut() = value;
    }
}

/// Implements [`IColumn`] for a type that holds a [`ColumnBase`] in the field `base`.
#[macro_export]
macro_rules! impl_column_base {
    ($ty:ident < $($param:ident),+ > where $($bounds:tt)*) => {
        impl<$($param),+> ::ferroui_base::data::model::INotifyPropertyChanged for $ty<$($param),+> where $($bounds)* {
            fn property_changed(&self) -> &::ferroui_base::data::model::Event<str> {
                self.base.property_changed()
            }
        }

        impl<$($param),+> $crate::models::IColumn for $ty<$($param),+> where $($bounds)* {
            fn id(&self) -> String { self.base.id() }
            fn header(&self) -> Option<::ferroui_base::BoxedValue> { self.base.header() }
            fn width(&self) -> $crate::GridLength { self.base.width() }
            fn set_width(&self, value: $crate::GridLength) { self.base.set_width(value) }
            fn is_visible(&self) -> bool { self.base.is_visible() }
            fn set_is_visible(&self, value: bool) { self.base.set_is_visible(value) }
            fn presentation_key(&self) -> Option<String> { self.base.presentation_key() }
            fn set_presentation_key(&self, value: Option<String>) { self.base.set_presentation_key(value) }
            fn sort_direction(&self) -> Option<$crate::models::ListSortDirection> { self.base.sort_direction() }
            fn set_sort_direction(&self, value: Option<$crate::models::ListSortDirection>) { self.base.set_sort_direction(value) }
            fn tag(&self) -> Option<::ferroui_base::BoxedValue> { self.base.tag() }
            fn set_tag(&self, value: Option<::ferroui_base::BoxedValue>) { self.base.set_tag(value) }
            fn as_any(&self) -> &dyn ::std::any::Any { self }
            fn model_type(&self) -> ::std::any::TypeId { ::std::any::TypeId::of::<M>() }
        }
    };
}

/// How the values of a column compare: `Comparer<TValue>.Default`.
pub type ValueComparer<V> = Rc<dyn Fn(&V, &V) -> Ordering>;

/// The default comparer of a value that is partially ordered: values that do not compare (a NaN)
/// compare as the reference's `Comparer<double>.Default` does, a NaN before every number.
pub fn default_comparer<V: PartialOrd + 'static>() -> ValueComparer<V> {
    Rc::new(|a: &V, b: &V| {
        a.partial_cmp(b).unwrap_or_else(|| {
            // Only a value that is not equal to itself fails to compare.
            #[allow(clippy::eq_op)]
            let a_nan = a.partial_cmp(a).is_none();
            #[allow(clippy::eq_op)]
            let b_nan = b.partial_cmp(b).is_none();
            match (a_nan, b_nan) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Less,
                _ => Ordering::Greater,
            }
        })
    })
}

/// `ValueColumn<TModel, TValue>`: a column whose cells show a value of the model.
pub struct ValueColumn<M: 'static, V: 'static> {
    base: ColumnBase<M>,
    accessor: Accessor<M, V>,
    comparer: Option<ValueComparer<V>>,
    boxer: Rc<dyn Fn(&V) -> Option<BoxedValue>>,
    unboxer: Rc<dyn Fn(Option<&BoxedValue>) -> Option<V>>,
    // A template column: its own comparison (the options only), its value the model.
    is_template: Cell<bool>,
    // A check box column of a `bool` (the reference's `BooleanGetter`): two states.
    is_boolean: Cell<bool>,
    weak_self: std::rc::Weak<Self>,
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> INotifyPropertyChanged for ValueColumn<M, V> {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> IColumn for ValueColumn<M, V> {
    fn id(&self) -> String {
        self.base.id()
    }
    fn header(&self) -> Option<BoxedValue> {
        self.base.header()
    }
    fn width(&self) -> GridLength {
        self.base.width()
    }
    fn set_width(&self, value: GridLength) {
        self.base.set_width(value)
    }
    fn is_visible(&self) -> bool {
        self.base.is_visible()
    }
    fn set_is_visible(&self, value: bool) {
        self.base.set_is_visible(value)
    }
    fn presentation_key(&self) -> Option<String> {
        self.base.presentation_key()
    }
    fn set_presentation_key(&self, value: Option<String>) {
        self.base.set_presentation_key(value)
    }
    fn sort_direction(&self) -> Option<ListSortDirection> {
        self.base.sort_direction()
    }
    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.base.set_sort_direction(value)
    }
    fn tag(&self) -> Option<BoxedValue> {
        self.base.tag()
    }
    fn set_tag(&self, value: Option<BoxedValue>) {
        self.base.set_tag(value)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn model_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<M>()
    }
    fn untyped_value(&self) -> Option<&dyn IUntypedValueColumn> {
        Some(self)
    }
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> IUntypedValueColumn for ValueColumn<M, V> {
    fn get_value(&self, model: &BoxedValue) -> Option<BoxedValue> {
        typed_model::<M>(model).and_then(|m| (self.boxer)(&self.get_value(&m)))
    }

    fn set_value(&self, model: &BoxedValue, value: Option<&BoxedValue>) -> bool {
        match typed_model::<M>(model) {
            Some(m) => IValueColumn::set_boxed_value(self, &m, value),
            None => false,
        }
    }

    fn can_set(&self) -> bool {
        self.accessor.setter().is_some()
    }

    fn property_name(&self) -> Option<&'static str> {
        self.accessor.property_name()
    }

    fn observe(&self, model: &BoxedValue, changed: Rc<dyn Fn()>) -> Option<UntypedSubscription> {
        let model = typed_model::<M>(model)?;
        let observer = PropertyPathObserver::create(&self.accessor)?;
        Some(Box::new(observer.subscribe(&model, changed)) as UntypedSubscription)
    }

    fn is_template(&self) -> bool {
        self.is_template.get()
    }

    fn is_three_state(&self) -> bool {
        !self.is_boolean.get()
    }

    fn layout_options(&self) -> (Option<bool>, Option<bool>, GridLength, Option<GridLength>) {
        let options = self.base.options();
        (options.can_user_resize_column, options.can_user_sort_column, options.min_width, options.max_width)
    }
}

impl<M: TreeModel, V: Clone + PartialEq + PartialOrd + 'static> ValueColumn<M, V> {
    /// A column of the value `accessor` reads (`ValueColumn(header, getter, setter, ...)`); the
    /// accessor's setter makes the column writable.
    pub fn new(
        header: Option<BoxedValue>,
        accessor: Accessor<M, V>,
        width: Option<GridLength>,
        options: Option<ColumnOptions<M>>,
        id: Option<String>,
    ) -> Rc<Self> {
        Self::with_comparer(header, accessor, Some(default_comparer::<V>()), width, options, id)
    }

    /// A column of a getter without a member name (`FromDelegate`): only the model is observed.
    pub fn from_delegate(
        header: Option<BoxedValue>,
        getter: impl Fn(&Rc<M>) -> V + 'static,
        property_name: Option<&'static str>,
        setter: Option<Rc<dyn Fn(&Rc<M>, V)>>,
        width: Option<GridLength>,
        options: Option<ColumnOptions<M>>,
        id: Option<String>,
    ) -> Rc<Self> {
        let mut accessor = Accessor::new(getter);
        if let Some(name) = property_name {
            accessor = accessor.named(name);
        }
        if let Some(setter) = setter {
            accessor = accessor.with_setter(move |m, v| setter(m, v));
        }
        Self::new(header, accessor, width, options, id)
    }
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> ValueColumn<M, V> {
    /// A column whose values compare with `comparer` (`None`: the values do not sort).
    pub fn with_comparer(
        header: Option<BoxedValue>,
        accessor: Accessor<M, V>,
        comparer: Option<ValueComparer<V>>,
        width: Option<GridLength>,
        options: Option<ColumnOptions<M>>,
        id: Option<String>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            base: ColumnBase::new(header, width, options, id),
            accessor,
            comparer,
            boxer: Rc::new(|v: &V| Some(Rc::new(v.clone()) as BoxedValue)),
            unboxer: Rc::new(|b: Option<&BoxedValue>| b.and_then(|b| b.downcast_ref::<V>().cloned())),
            is_template: Cell::new(false),
            is_boolean: Cell::new(false),
            weak_self: weak_self.clone(),
        })
    }
}

impl<M: 'static, V: 'static> ValueColumn<M, V> {
    pub fn base(&self) -> &ColumnBase<M> {
        &self.base
    }

    pub fn options(&self) -> &ColumnOptions<M> {
        self.base.options()
    }

    /// The accessor of the value (the reference's `GetterExpression`, `Getter` and `Setter`).
    pub fn accessor(&self) -> &Accessor<M, V> {
        &self.accessor
    }

    pub fn property_name(&self) -> Option<&'static str> {
        self.accessor.property_name()
    }

    pub fn get_value(&self, model: &Rc<M>) -> V {
        self.accessor.get(model)
    }

    /// Writes the value.
    ///
    /// # Panics
    /// When the column is read only (`InvalidOperationException`).
    pub fn set_value(&self, model: &Rc<M>, value: V) {
        if !self.accessor.set(model, value) {
            panic!("Column is read-only.");
        }
    }

    pub fn setter(&self) -> Option<&Rc<dyn Fn(&Rc<M>, V)>> {
        self.accessor.setter()
    }

    /// The shared column (the `this` the reference's visitors receive).
    pub fn this(&self) -> Rc<Self> {
        self.weak_self.upgrade().expect("a column that is alive")
    }

    fn value_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        let options = self.base.options();
        if options.can_user_sort_column == Some(false) {
            return None;
        }
        let custom = match direction {
            ListSortDirection::Ascending => options.compare_ascending.clone(),
            ListSortDirection::Descending => options.compare_descending.clone(),
        };
        if custom.is_some() {
            return custom;
        }
        let comparer = self.comparer.clone()?;
        let getter = self.accessor.getter().clone();
        Some(Rc::new(move |x: &Rc<M>, y: &Rc<M>| match direction {
            ListSortDirection::Ascending => comparer(&getter(x), &getter(y)),
            ListSortDirection::Descending => comparer(&getter(y), &getter(x)),
        }))
    }
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> IColumnOf<M> for ValueColumn<M, V> {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        if self.is_template.get() {
            return template_comparison(self.base.options(), direction);
        }
        self.value_comparison(direction)
    }

    fn kind(&self) -> ColumnKind<'_, M> {
        ColumnKind::Value(self)
    }
}

impl<M: 'static, V: 'static> ValueColumn<M, V> {
    /// Whether the column is a [`TemplateColumn`].
    pub fn is_template(&self) -> bool {
        self.is_template.get()
    }

    /// For a [`CheckBoxColumn`]: whether its value has three states (`IsThreeState`: a column of a
    /// `bool?`, not of a `bool`).
    pub fn is_three_state(&self) -> bool {
        !self.is_boolean.get()
    }
}

fn template_comparison<M: 'static>(options: &ColumnOptions<M>, direction: ListSortDirection) -> Option<Comparison<M>> {
    if options.can_user_sort_column == Some(false) {
        return None;
    }
    match direction {
        ListSortDirection::Ascending => options.compare_ascending.clone(),
        ListSortDirection::Descending => options.compare_descending.clone(),
    }
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> IValueColumn<M> for ValueColumn<M, V> {
    fn get_boxed_value(&self, model: &Rc<M>) -> Option<BoxedValue> {
        (self.boxer)(&self.get_value(model))
    }

    fn set_boxed_value(&self, model: &Rc<M>, value: Option<&BoxedValue>) -> bool {
        if self.accessor.setter().is_none() {
            return false;
        }
        match (self.unboxer)(value) {
            Some(value) => self.accessor.set(model, value),
            None => false,
        }
    }

    fn can_set(&self) -> bool {
        self.accessor.setter().is_some()
    }

    fn property_name(&self) -> Option<&'static str> {
        self.accessor.property_name()
    }

    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<V>()
    }

    fn as_value_any(&self) -> &dyn Any {
        self
    }
}

/// `TextColumn<TModel, TValue>`: a value column presented as text.
pub struct TextColumn;

impl TextColumn {
    /// A text column (`TextColumn(header, getter, [setter,] width, options, id)`).
    pub fn new<M: TreeModel, V: Clone + PartialEq + PartialOrd + 'static>(
        header: Option<BoxedValue>,
        accessor: Accessor<M, V>,
        width: Option<GridLength>,
        options: Option<ColumnOptions<M>>,
        id: Option<String>,
    ) -> Rc<ValueColumn<M, V>> {
        ValueColumn::new(header, accessor, width, options, id)
    }
}

/// `CheckBoxColumn<TModel>`: a value column of `bool?` presented as a check box.
pub struct CheckBoxColumn;

impl CheckBoxColumn {
    /// A two-state check box column of a `bool` (`CheckBoxColumn(header, Expression<Func<TModel, bool>>, ...)`):
    /// the value is the boolean as `Some`, and a cleared value writes `false`.
    pub fn new<M: TreeModel>(
        header: Option<BoxedValue>,
        accessor: Accessor<M, bool>,
        width: Option<GridLength>,
        options: Option<ColumnOptions<M>>,
        id: Option<String>,
    ) -> Rc<ValueColumn<M, Option<bool>>> {
        let column = ValueColumn::new(header, accessor.map(Some, |v: Option<bool>| v.unwrap_or(false)), width, options, id);
        column.base.set_presentation_key(Some("CheckBox".to_string()));
        column.is_boolean.set(true);
        column
    }

    /// A three-state check box column of a `bool?`.
    pub fn new_three_state<M: TreeModel>(
        header: Option<BoxedValue>,
        accessor: Accessor<M, Option<bool>>,
        width: Option<GridLength>,
        options: Option<ColumnOptions<M>>,
        id: Option<String>,
    ) -> Rc<ValueColumn<M, Option<bool>>> {
        let column = ValueColumn::new(header, accessor, width, options, id);
        column.base.set_presentation_key(Some("CheckBox".to_string()));
        column
    }
}

/// `TemplateColumn<TModel>`: a column whose cells the presentation builds for the model, selected
/// by its presentation key; it sorts only with the comparisons of its options.
pub struct TemplateColumn;

impl TemplateColumn {
    pub fn new<M: TreeModel>(
        header: Option<BoxedValue>,
        presentation_key: &str,
        width: Option<GridLength>,
        options: Option<ColumnOptions<M>>,
        id: Option<String>,
    ) -> Rc<ValueColumn<M, Rc<M>>> {
        let column = ValueColumn::with_comparer(header, Accessor::new(|m: &Rc<M>| m.clone()), None, width, options, id);
        column.base.set_presentation_key(Some(presentation_key.to_string()));
        column.is_template.set(true);
        column
    }
}

/// `IExpanderColumn<TModel>`: a column whose cells show an expander that reveals nested data.
pub trait IExpanderColumn<M: TreeModel>: IColumnOf<M> {
    /// Whether the model has nested data.
    fn has_children(&self, model: &Rc<M>) -> bool;

    /// The bound expansion state of the model, or `None` when expansion belongs only to the source.
    fn get_model_is_expanded(&self, _model: &Rc<M>) -> Option<bool> {
        None
    }

    /// The child models.
    fn get_child_models(&self, model: &Rc<M>) -> Option<Models<M>>;

    /// Writes the row's expansion state to its model.
    fn set_model_is_expanded(&self, row: &dyn IExpanderRow<M>);

    /// The observer of the path of the expansion state, if it is bound below the model.
    fn expansion_observer(&self) -> Option<Rc<PropertyPathObserver<M>>> {
        None
    }
}

/// `HierarchicalExpanderColumn<TModel>`: wraps a column and gives it an expander.
pub struct HierarchicalExpanderColumn<M: 'static> {
    inner: ColumnRef<M>,
    children: Rc<dyn Fn(&Rc<M>) -> Option<Models<M>>>,
    has_children: Option<Accessor<M, bool>>,
    is_expanded: Option<Accessor<M, bool>>,
    set_expanded: Option<Rc<dyn Fn(&Rc<M>, bool)>>,
    expanded_observer: Option<Rc<PropertyPathObserver<M>>>,
}

impl<M: TreeModel> HierarchicalExpanderColumn<M> {
    /// A column that shows `inner` with an expander; `child_selector` gives the children of a
    /// model, `has_children_selector` whether it has any (without it: whether the children are not
    /// empty), `is_expanded_selector` a bound expansion state, written back with its setter or with
    /// `set_is_expanded`.
    pub fn new(
        inner: ColumnRef<M>,
        child_selector: impl Fn(&Rc<M>) -> Option<Models<M>> + 'static,
        has_children_selector: Option<Accessor<M, bool>>,
        is_expanded_selector: Option<Accessor<M, bool>>,
        set_is_expanded: Option<Rc<dyn Fn(&Rc<M>, bool)>>,
    ) -> Rc<Self> {
        let expanded_observer = is_expanded_selector.as_ref().and_then(PropertyPathObserver::create);
        let set_expanded = set_is_expanded.or_else(|| is_expanded_selector.as_ref().and_then(|s| s.setter().cloned()));
        Rc::new(Self {
            inner,
            children: Rc::new(child_selector),
            has_children: has_children_selector,
            is_expanded: is_expanded_selector,
            set_expanded,
            expanded_observer,
        })
    }

    /// The column the expander column shows.
    pub fn inner(&self) -> &ColumnRef<M> {
        &self.inner
    }

    pub fn has_children_selector(&self) -> Option<&Accessor<M, bool>> {
        self.has_children.as_ref()
    }

    pub fn is_expanded_selector(&self) -> Option<&Accessor<M, bool>> {
        self.is_expanded.as_ref()
    }
}

impl<M: TreeModel> INotifyPropertyChanged for HierarchicalExpanderColumn<M> {
    fn property_changed(&self) -> &Event<str> {
        self.inner.property_changed()
    }
}

impl<M: TreeModel> IColumn for HierarchicalExpanderColumn<M> {
    fn id(&self) -> String {
        self.inner.id()
    }
    fn header(&self) -> Option<BoxedValue> {
        self.inner.header()
    }
    fn width(&self) -> GridLength {
        self.inner.width()
    }
    fn set_width(&self, value: GridLength) {
        self.inner.set_width(value)
    }
    fn is_visible(&self) -> bool {
        self.inner.is_visible()
    }
    fn set_is_visible(&self, value: bool) {
        self.inner.set_is_visible(value)
    }
    fn presentation_key(&self) -> Option<String> {
        self.inner.presentation_key()
    }
    fn set_presentation_key(&self, value: Option<String>) {
        self.inner.set_presentation_key(value)
    }
    fn sort_direction(&self) -> Option<ListSortDirection> {
        self.inner.sort_direction()
    }
    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.inner.set_sort_direction(value)
    }
    fn tag(&self) -> Option<BoxedValue> {
        self.inner.tag()
    }
    fn set_tag(&self, value: Option<BoxedValue>) {
        self.inner.set_tag(value)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn model_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<M>()
    }
    fn untyped_expander(&self) -> Option<&dyn IUntypedExpanderColumn> {
        Some(self)
    }
}

impl<M: TreeModel> IUntypedExpanderColumn for HierarchicalExpanderColumn<M> {
    fn inner(&self) -> Rc<dyn IColumn> {
        self.inner.clone() as Rc<dyn IColumn>
    }

    fn has_children(&self, model: &BoxedValue) -> bool {
        typed_model::<M>(model).is_some_and(|m| IExpanderColumn::has_children(self, &m))
    }

    fn get_model_is_expanded(&self, model: &BoxedValue) -> Option<bool> {
        typed_model::<M>(model).and_then(|m| IExpanderColumn::get_model_is_expanded(self, &m))
    }
}

impl<M: TreeModel> IColumnOf<M> for HierarchicalExpanderColumn<M> {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        self.inner.get_comparison(direction)
    }

    fn kind(&self) -> ColumnKind<'_, M> {
        ColumnKind::HierarchicalExpander(self)
    }

    fn as_expander_column(&self) -> Option<&dyn IExpanderColumn<M>> {
        Some(self)
    }

    fn as_expander_column_any(&self) -> bool {
        true
    }
}

impl<M: TreeModel> IExpanderColumn<M> for HierarchicalExpanderColumn<M> {
    fn has_children(&self, model: &Rc<M>) -> bool {
        match &self.has_children {
            Some(selector) => selector.get(model),
            None => (self.children)(model).is_some_and(|children| children.count() > 0),
        }
    }

    fn get_model_is_expanded(&self, model: &Rc<M>) -> Option<bool> {
        self.is_expanded.as_ref().map(|selector| selector.get(model))
    }

    fn get_child_models(&self, model: &Rc<M>) -> Option<Models<M>> {
        (self.children)(model)
    }

    fn set_model_is_expanded(&self, row: &dyn IExpanderRow<M>) {
        if let Some(set_expanded) = &self.set_expanded {
            set_expanded(&row.model_of(), row.is_expanded());
        }
    }

    fn expansion_observer(&self) -> Option<Rc<PropertyPathObserver<M>>> {
        self.expanded_observer.clone()
    }
}
