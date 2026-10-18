//! The helpers of the model tests: the reference's `LayoutTestColumn.cs` and `LayoutTestCell.cs`,
//! the models of the tests, a stand-in of `BehaviorSubject<BindingValue<T>>` and a deterministic
//! random number generator (the reference's `System.Random` with a seed).

use crate::models::column_base::impl_view_column_base;
use crate::models::{
    CellRef, ColumnBase, ColumnOptions, ICellBinding, ICellColumn, IColumnOptions, TextCell, TextColumn,
};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{ColumnKind, Comparison, IColumnOf, IValueColumn, ListSortDirection, RowRef};
use ferroui_treedatagrid_core::Accessor;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The model of the column tests: a name and a country.
#[derive(Default)]
pub(crate) struct Model {
    pub name: RefCell<Option<String>>,
    pub country: RefCell<Option<String>>,
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Model {
    /// `x => x.Name`.
    pub fn name_accessor() -> Accessor<Model, Option<String>> {
        Accessor::member("Name", |m: &Rc<Model>| m.name.borrow().clone())
    }

    /// `x => x.Country`.
    pub fn country_accessor() -> Accessor<Model, Option<String>> {
        Accessor::member("Country", |m: &Rc<Model>| m.country.borrow().clone())
    }
}

/// A text column of the model's name with the width `width` and the options `options`.
pub(crate) fn name_column(
    width: GridLength,
    options: Option<Rc<crate::models::TextColumnOptions<Model>>>,
) -> Rc<TextColumn<Model, Option<String>>> {
    TextColumn::new(None, Model::name_accessor(), Some(width), options)
}

/// A text column of the model's country with the width `width` and the options `options`.
pub(crate) fn country_column(
    width: GridLength,
    options: Option<Rc<crate::models::TextColumnOptions<Model>>>,
) -> Rc<TextColumn<Model, Option<String>>> {
    TextColumn::new(None, Model::country_accessor(), Some(width), options)
}

/// A length in pixels.
pub(crate) fn pixels(value: f64) -> GridLength {
    GridLength::new(value, GridUnitType::Pixel)
}

/// A length in stars.
pub(crate) fn stars(value: f64) -> GridLength {
    GridLength::new(value, GridUnitType::Star)
}

/// `LayoutTestCell`: a read-only text cell of a constant text.
pub(crate) fn layout_test_cell(value: String) -> CellRef {
    TextCell::from_value(Some(value))
}

/// `LayoutTestColumn<TModel>`: a column of the layout tests whose cells show the header and the
/// row's index, with a minimum width of 0 by default.
pub(crate) struct LayoutTestColumn<M: TreeModel> {
    weak_self: Weak<Self>,
    base: ColumnBase<M>,
}

impl<M: TreeModel> LayoutTestColumn<M> {
    pub fn new(header: &str, width: Option<GridLength>, options: Option<Rc<ColumnOptions<M>>>) -> Rc<Self> {
        let options: Rc<dyn IColumnOptions<M>> = options.unwrap_or_else(|| {
            let options = ColumnOptions::default();
            options.set_min_width(GridLength::new(0.0, GridUnitType::Pixel));
            Rc::new(options)
        });
        let header = Some(Rc::new(header.to_string()) as BoxedValue);
        Rc::new_cyclic(|weak_self| Self { weak_self: weak_self.clone(), base: ColumnBase::new(header, width, options) })
    }
}

impl_view_column_base!(impl[M: TreeModel] LayoutTestColumn<M>, model M);

impl<M: TreeModel> ICellColumn for LayoutTestColumn<M> {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        let indexable = row.as_model_indexable().expect("a row of an indexed source");
        let header = self.base.header().as_ref().and_then(|h| h.downcast_ref::<String>().cloned()).unwrap_or_default();
        layout_test_cell(format!("{header} Row {}", indexable.model_index()))
    }
}

impl<M: TreeModel> IColumnOf<M> for LayoutTestColumn<M> {
    fn get_comparison(&self, _direction: ListSortDirection) -> Option<Comparison<M>> {
        panic!("The method or operation is not implemented.");
    }

    fn kind(&self) -> ColumnKind<'_, M> {
        ColumnKind::Value(self)
    }
}

impl<M: TreeModel> IValueColumn<M> for LayoutTestColumn<M> {
    fn get_boxed_value(&self, _model: &Rc<M>) -> Option<BoxedValue> {
        None
    }

    fn set_boxed_value(&self, _model: &Rc<M>, _value: Option<&BoxedValue>) -> bool {
        false
    }

    fn can_set(&self) -> bool {
        false
    }

    fn property_name(&self) -> Option<&'static str> {
        None
    }

    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<String>()
    }

    fn as_value_any(&self) -> &dyn Any {
        self
    }
}

/// `BehaviorSubject<BindingValue<T>>`: a binding that holds a value, pushes it to each new
/// subscriber and pushes every value written to it to all subscribers.
pub(crate) struct BehaviorSubject<T: Clone + 'static> {
    value: RefCell<T>,
    observers: HandlerList<dyn Fn(&T)>,
}

impl<T: Clone + 'static> BehaviorSubject<T> {
    pub fn new(value: T) -> Rc<Self> {
        Rc::new(Self { value: RefCell::new(value), observers: HandlerList::new() })
    }
}

impl<T: Clone + 'static> ICellBinding<T> for BehaviorSubject<T> {
    fn subscribe(&self, observer: Rc<dyn Fn(&T)>) -> u64 {
        let token = self.observers.add(observer.clone());
        let value = self.value.borrow().clone();
        observer(&value);
        token
    }

    fn unsubscribe(&self, token: u64) {
        self.observers.remove(token);
    }

    fn on_next(&self, value: T) {
        *self.value.borrow_mut() = value.clone();
        for (_, observer) in self.observers.snapshot().iter() {
            observer(&value);
        }
    }

    fn can_write(&self) -> bool {
        true
    }
}

/// A deterministic random number generator (the reference's `new Random(seed)`; the sequence is
/// not the runtime's, which the tests do not depend on).
pub(crate) struct Random {
    state: Cell<u64>,
}

impl Random {
    pub fn new(seed: u64) -> Self {
        Self { state: Cell::new(seed ^ 0x9E37_79B9_7F4A_7C15) }
    }

    fn next_u64(&self) -> u64 {
        // SplitMix64.
        let mut z = self.state.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.state.set(z);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `[min, max)` (`Next(minValue, maxValue)`).
    pub fn next_range(&self, min: i32, max: i32) -> i32 {
        if max <= min {
            return min;
        }
        let range = (max - min) as u64;
        min + (self.next_u64() % range) as i32
    }

    /// A number in `[0, max)` (`Next(maxValue)`).
    pub fn next(&self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_u64() % max as u64) as usize
    }
}
