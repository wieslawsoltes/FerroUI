//! `Presentation/ExpanderColumnPresentation.cs`: the presentation of a core expander column, and
//! the expander cells it creates (`CoreExpanderCell`).

use crate::models::{
    BeginEditGestures, CellRef, ColumnRef, ICell, ICellColumn, IColumn, IExpanderCellPresentation, IUpdateColumnLayout,
};
use crate::selection::{clear_show_expander, update_show_expander};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::accessor::as_notifier;
use ferroui_treedatagrid_core::models::{IExpander, ListSortDirection, NotifyingBase, RowRef};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The presentation of a core expander column (`ExpanderColumnPresentation<T>`): the inner
/// column's cells with an expander that follows the row.
pub struct ExpanderColumnPresentation {
    weak_self: Weak<Self>,
    model: Rc<dyn ferroui_treedatagrid_core::models::IColumn>,
    inner: ColumnRef,
    disposed: Cell<bool>,
}

impl ExpanderColumnPresentation {
    /// The presentation of the expander column `model`, showing the cells of `inner`.
    ///
    /// # Panics
    /// When `model` is not an expander column (`untyped_expander`).
    pub fn new(model: Rc<dyn ferroui_treedatagrid_core::models::IColumn>, inner: ColumnRef) -> Rc<Self> {
        assert!(model.untyped_expander().is_some(), "The column is not an expander column.");
        Rc::new_cyclic(|weak_self| Self { weak_self: weak_self.clone(), model, inner, disposed: Cell::new(false) })
    }

    /// The core column presented.
    pub fn model(&self) -> &Rc<dyn ferroui_treedatagrid_core::models::IColumn> {
        &self.model
    }

    /// The presentation of the inner column.
    pub fn inner(&self) -> &ColumnRef {
        &self.inner
    }

    fn layout(&self) -> &dyn IUpdateColumnLayout {
        self.inner.as_update_column_layout().expect("The inner column of an expander column lays itself out.")
    }

    /// Disposes the inner column's presentation (`Dispose`).
    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        super::dispose_column(&self.inner);
    }
}

impl INotifyPropertyChanged for ExpanderColumnPresentation {
    fn property_changed(&self) -> &Event<str> {
        // `PropertyChanged { add => _inner.PropertyChanged += value; ... }`.
        self.inner.property_changed()
    }
}

impl IColumn for ExpanderColumnPresentation {
    fn actual_width(&self) -> f64 {
        self.inner.actual_width()
    }

    fn can_user_resize(&self) -> Option<bool> {
        self.inner.can_user_resize()
    }

    fn header(&self) -> Option<BoxedValue> {
        self.model.header()
    }

    fn width(&self) -> GridLength {
        self.inner.width()
    }

    fn sort_direction(&self) -> Option<ListSortDirection> {
        self.inner.sort_direction()
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.inner.set_sort_direction(value);
    }

    fn tag(&self) -> Option<BoxedValue> {
        self.model.tag()
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        self.model.set_tag(value);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_update_column_layout(&self) -> Option<&dyn IUpdateColumnLayout> {
        Some(self)
    }

    fn as_cell_column(&self) -> Option<&dyn ICellColumn> {
        Some(self)
    }

    fn requires_unconstrained_width_measurement(&self) -> bool {
        // `_inner is not IColumnMeasurementOptions options || options.RequiresUnconstrainedWidthMeasurement`:
        // a column without layout of its own is measured unconstrained.
        self.inner.as_update_column_layout().is_none() || self.inner.requires_unconstrained_width_measurement()
    }

    fn core_column(&self) -> Option<Rc<dyn ferroui_treedatagrid_core::models::IColumn>> {
        Some(self.model.clone())
    }
}

impl IUpdateColumnLayout for ExpanderColumnPresentation {
    fn min_actual_width(&self) -> f64 {
        self.layout().min_actual_width()
    }

    fn max_actual_width(&self) -> f64 {
        self.layout().max_actual_width()
    }

    fn star_width_was_constrained(&self) -> bool {
        self.layout().star_width_was_constrained()
    }

    fn cell_measured(&self, width: f64, row_index: i32) -> f64 {
        self.layout().cell_measured(width, row_index)
    }

    fn calculate_star_width(&self, available_width: f64, total_stars: f64) {
        self.layout().calculate_star_width(available_width, total_stars);
    }

    fn commit_actual_width(&self) -> bool {
        self.layout().commit_actual_width()
    }

    fn set_width(&self, width: GridLength) {
        self.layout().set_width(width);
    }
}

impl ICellColumn for ExpanderColumnPresentation {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        let inner = self.inner.as_cell_column().expect("The inner column of an expander column creates cells.");
        let content = inner.create_cell(row);
        let this = self.weak_self.upgrade().expect("an expander column that is alive");
        CoreExpanderCell::new(content, row.clone(), this) as CellRef
    }

    fn select_value(&self, model: &BoxedValue) -> Option<String> {
        self.inner.as_cell_column().and_then(|inner| inner.select_value(model))
    }

    fn is_text_search_enabled(&self) -> bool {
        self.inner.as_cell_column().is_some_and(|inner| inner.is_text_search_enabled())
    }
}

/// An expander cell of a core row (`CoreExpanderCell<T>`): the inner column's cell, the row's
/// expansion state, and whether the row shows its expander, which follows the model.
pub struct CoreExpanderCell {
    notifying: NotifyingBase,
    inner: CellRef,
    row: RowRef,
    row_token: Cell<Option<u64>>,
    // The subscription of the "show expander" observation (`ShowExpanderObservable`): the model's
    // property changes, which can change whether it has children.
    show_expander: RefCell<Option<(BoxedValue, u64)>>,
}

impl CoreExpanderCell {
    fn new(inner: CellRef, row: RowRef, column: Rc<ExpanderColumnPresentation>) -> Rc<Self> {
        let this = Rc::new(Self {
            notifying: NotifyingBase::new(),
            inner,
            row: row.clone(),
            row_token: Cell::new(None),
            show_expander: RefCell::new(None),
        });

        if let Some(notifier) = row.as_notify_property_changed() {
            let weak = Rc::downgrade(&this);
            let token = notifier.property_changed().add(Rc::new(move |name: &str| {
                if let Some(this) = weak.upgrade() {
                    this.notifying.raise_property_changed(name);
                }
            }));
            this.row_token.set(Some(token));
        }

        // `showExpander.Subscribe(row.UpdateShowExpander)`: publish whether the model has
        // children now and whenever the model changes.
        if let Some(model) = row.model() {
            let publish = {
                let row = Rc::downgrade(&row);
                let column = Rc::downgrade(&column);
                let model = model.clone();
                move || {
                    if let (Some(row), Some(column)) = (row.upgrade(), column.upgrade()) {
                        if let Some(expander) = column.model.untyped_expander() {
                            update_show_expander(&row, expander.has_children(&model));
                        }
                    }
                }
            };
            publish();
            if let Some(notifier) = as_notifier(&*model) {
                let token = notifier.property_changed().add(Rc::new(move |_: &str| publish()));
                *this.show_expander.borrow_mut() = Some((model, token));
            }
        }

        this
    }

    fn expander(&self) -> Option<&dyn IExpander> {
        self.row.as_expander()
    }
}

impl INotifyPropertyChanged for CoreExpanderCell {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl ICell for CoreExpanderCell {
    fn can_edit(&self) -> bool {
        self.inner.can_edit()
    }

    fn edit_gestures(&self) -> BeginEditGestures {
        self.inner.edit_gestures()
    }

    fn value(&self) -> Option<BoxedValue> {
        self.inner.value()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_expander_cell(&self) -> Option<&dyn IExpanderCellPresentation> {
        Some(self)
    }

    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        Some(self)
    }

    fn dispose(&self) {
        if let Some(token) = self.row_token.take() {
            if let Some(notifier) = self.row.as_notify_property_changed() {
                notifier.property_changed().remove(token);
            }
        }
        let show_expander = self.show_expander.borrow_mut().take();
        if let Some((model, token)) = show_expander {
            if let Some(notifier) = as_notifier(&*model) {
                notifier.property_changed().remove(token);
            }
        }
        clear_show_expander(&self.row);
        self.inner.dispose();
    }
}

impl IExpander for CoreExpanderCell {
    fn is_expanded(&self) -> bool {
        self.expander().is_some_and(|e| e.is_expanded())
    }

    fn set_is_expanded(&self, value: bool) {
        if let Some(expander) = self.expander() {
            expander.set_is_expanded(value);
        }
    }

    fn show_expander(&self) -> bool {
        self.expander().is_some_and(|e| e.show_expander())
    }
}

impl IExpanderCellPresentation for CoreExpanderCell {
    fn content(&self) -> Option<CellRef> {
        Some(self.inner.clone())
    }

    fn row(&self) -> RowRef {
        self.row.clone()
    }
}
