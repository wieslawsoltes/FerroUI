use crate::presenters::TableViewCellsPresenter;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{ContentControlImpl, ControlImpl, ListBoxItem, TableViewColumn};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use std::cell::RefCell;

const PART_CELLS_PRESENTER: &str = "PART_CellsPresenter";

/// A row container in a [`TableView`](crate::TableView).
#[repr(C)]
pub struct TableViewRow {
    base: ListBoxItem,
    cells_presenter: RefCell<Option<Ref<TableViewCellsPresenter>>>,
    columns: RefCell<Option<FerroList<Ref<TableViewColumn>>>>,
}

ferro_class!(TableViewRow: ListBoxItem);
ferro_class_info!(TableViewRow {
    new: TableViewRow::new,
    markup: {
        attributes: [TemplatePart("PART_CellsPresenter", type(Ref<TableViewCellsPresenter>))],
    },
});
ferro_impl_classes!(
    TableViewRow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    ContentControlImpl
);

impl TemplatedControlImpl for TableViewRow {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        if let Some(cells_presenter) = this.cells_presenter.take() {
            debug_assert!(cells_presenter.row().is_some_and(|row| row.ptr_eq(&this.to_ref())));
            cells_presenter.remove_cells();
            cells_presenter.set_row(None);
        }

        let cells_presenter = e.name_scope().find_as::<TableViewCellsPresenter>(PART_CELLS_PRESENTER);
        *this.cells_presenter.borrow_mut() = cells_presenter.clone();

        if let Some(cells_presenter) = cells_presenter {
            debug_assert!(cells_presenter.row().is_none());
            cells_presenter.set_row(Some(&this.to_ref()));
            cells_presenter.rebuild_cells();
        }
    }
}

impl TableViewRow {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListBoxItem::construct(), cells_presenter: RefCell::new(None), columns: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The columns of the table view the row belongs to.
    pub(crate) fn columns(&self) -> Option<FerroList<Ref<TableViewColumn>>> {
        self.columns.borrow().clone()
    }

    pub(crate) fn set_columns(&self, value: Option<FerroList<Ref<TableViewColumn>>>) {
        *self.columns.borrow_mut() = value;
    }

    fn cells_presenter(&self) -> Option<Ref<TableViewCellsPresenter>> {
        self.cells_presenter.borrow().clone()
    }

    pub(crate) fn clear_cells(&self) {
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.clear_cells();
        }
    }

    pub(crate) fn invalidate_cells_measure(&self) {
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.invalidate_measure();
        }
    }

    pub(crate) fn rebuild_cells(&self) {
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.rebuild_cells();
        }
    }

    pub(crate) fn refresh_cell(&self, column_index: usize) {
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.refresh_cell(column_index);
        }
    }
}
