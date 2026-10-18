//! `Models/TreeDataGrid/ExpanderCell.cs`: the cell of an expander column: the cell of the inner
//! column with an expander that expands and collapses the row.

use super::{BeginEditGestures, CellRef, ICell, ICellBinding, IExpanderCellPresentation, ShowExpanderObservable};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{IExpander, IExpanderRow, IRow, NotifyingBase, RowRef};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `ExpanderCell<TModel>`.
pub struct ExpanderCell<M: TreeModel> {
    notifying: NotifyingBase,
    inner: CellRef,
    row: Rc<dyn IExpanderRow<M>>,
    row_token: Cell<Option<u64>>,
    show_expander: RefCell<Option<Rc<ShowExpanderObservable<M>>>>,
    is_expanded: RefCell<Option<(Rc<dyn ICellBinding<bool>>, u64)>>,
}

impl<M: TreeModel> ExpanderCell<M> {
    /// The expander cell of `row` around the inner column's cell `inner`; `show_expander`
    /// publishes whether the expander is shown, `is_expanded` the model's expansion state.
    pub fn new(
        inner: CellRef,
        row: Rc<dyn IExpanderRow<M>>,
        show_expander: Rc<ShowExpanderObservable<M>>,
        is_expanded: Option<Rc<dyn ICellBinding<bool>>>,
    ) -> Rc<Self> {
        let this = Rc::new(Self {
            notifying: NotifyingBase::new(),
            inner,
            row: row.clone(),
            row_token: Cell::new(None),
            show_expander: RefCell::new(None),
            is_expanded: RefCell::new(None),
        });

        let weak = Rc::downgrade(&this);
        let token = row.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                this.row_property_changed(name);
            }
        }));
        this.row_token.set(Some(token));

        *this.show_expander.borrow_mut() = Some(show_expander.clone());
        let expander_row = Rc::downgrade(&row);
        show_expander.subscribe(Rc::new(move |value: bool| {
            if let Some(row) = expander_row.upgrade() {
                row.update_show_expander(value);
            }
        }));

        if let Some(is_expanded) = is_expanded {
            let weak = Rc::downgrade(&this);
            let token = is_expanded.subscribe(Rc::new(move |value: &bool| {
                if let Some(this) = weak.upgrade() {
                    this.set_is_expanded(*value);
                }
            }));
            *this.is_expanded.borrow_mut() = Some((is_expanded, token));
        }

        this
    }

    /// The cell of the inner column.
    pub fn content(&self) -> &CellRef {
        &self.inner
    }

    /// The row of the cell.
    pub fn row(&self) -> &Rc<dyn IExpanderRow<M>> {
        &self.row
    }

    /// Whether the expander is shown.
    pub fn show_expander(&self) -> bool {
        self.row.show_expander()
    }

    /// Whether the row is expanded.
    pub fn is_expanded(&self) -> bool {
        self.row.is_expanded()
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.row.set_is_expanded(value);
    }

    fn row_property_changed(&self, name: &str) {
        if name == "IsExpanded" || name == "ShowExpander" {
            self.notifying.raise_property_changed(name);
        }
    }
}

impl<M: TreeModel> INotifyPropertyChanged for ExpanderCell<M> {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl<M: TreeModel> ICell for ExpanderCell<M> {
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
            self.row.property_changed().remove(token);
        }
        let show_expander = self.show_expander.borrow_mut().take();
        if let Some(show_expander) = show_expander {
            show_expander.unsubscribe();
        }
        let is_expanded = self.is_expanded.borrow_mut().take();
        if let Some((binding, token)) = is_expanded {
            binding.unsubscribe(token);
        }
        self.inner.dispose();
    }
}

impl<M: TreeModel> IExpander for ExpanderCell<M> {
    fn is_expanded(&self) -> bool {
        ExpanderCell::is_expanded(self)
    }

    fn set_is_expanded(&self, value: bool) {
        ExpanderCell::set_is_expanded(self, value)
    }

    fn show_expander(&self) -> bool {
        ExpanderCell::show_expander(self)
    }
}

impl<M: TreeModel> IExpanderCellPresentation for ExpanderCell<M> {
    fn content(&self) -> Option<CellRef> {
        Some(self.inner.clone())
    }

    fn row(&self) -> RowRef {
        let row: Rc<dyn IRow> = self.row.clone();
        row
    }
}
