use super::table_view_layout_helper::TableViewLayoutHelper;
use crate::{ControlImpl, Panel, PanelImpl, TableViewCell, TableViewRow};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElement,
    StyledElementImpl, VisualImpl, WeakRef,
};
use std::cell::RefCell;

/// Lays out the cells of a [`TableViewRow`] according to the column
/// definitions of the parent table view.
///
/// The cells are recycled alongside their owning row in a virtualizing stack
/// panel, but are not virtualized in a single row (i.e. there is no column
/// virtualization).
#[repr(C)]
pub struct TableViewCellsPresenter {
    base: Panel,
    /// The row the presenter belongs to: an ancestor, held weakly.
    row: RefCell<Option<WeakRef<TableViewRow>>>,
}

ferro_class!(TableViewCellsPresenter: Panel);
ferro_class_info!(TableViewCellsPresenter { new: TableViewCellsPresenter::new });
ferro_impl_classes!(
    TableViewCellsPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl LayoutableImpl for TableViewCellsPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let Some(columns) = this.row().and_then(|row| row.columns()) else { return Size::default() };

        // In a standard template, the column widths should have been
        // computed by the headers' presenter. If for some reason they
        // weren't, do it now.
        if TableViewLayoutHelper::needs_actual_widths(&columns) {
            TableViewLayoutHelper::update_actual_widths(
                &columns,
                available_size.width,
                this.use_layout_rounding(),
                LayoutHelper::get_layout_scale(this),
            );
        }

        TableViewLayoutHelper::measure_row(&columns, &this.children(), available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let Some(columns) = this.row().and_then(|row| row.columns()) else { return final_size };

        if TableViewLayoutHelper::needs_actual_widths(&columns) {
            TableViewLayoutHelper::update_actual_widths(
                &columns,
                final_size.width,
                this.use_layout_rounding(),
                LayoutHelper::get_layout_scale(this),
            );
        }

        TableViewLayoutHelper::arrange_row(&columns, &this.children(), final_size, 0.0)
    }
}

impl TableViewCellsPresenter {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct(), row: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub(crate) fn row(&self) -> Option<Ref<TableViewRow>> {
        self.row.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_row(&self, value: Option<&Ref<TableViewRow>>) {
        *self.row.borrow_mut() = value.map(Ref::downgrade);
    }

    pub(crate) fn clear_cells(&self) {
        for child in self.children().snapshot().iter() {
            if let Some(cell) = child.downcast_ref::<TableViewCell>() {
                cell.set_column(None);
            }
        }
    }

    pub(crate) fn remove_cells(&self) {
        self.clear_cells();
        self.children().clear();
        if let Some(row) = self.row() {
            StyledElement::logical_children(&row).clear();
        }
    }

    pub(crate) fn rebuild_cells(&self) {
        let row = self.row();
        let Some((row, columns)) = row.and_then(|row| row.columns().map(|columns| (row, columns))) else {
            self.remove_cells();
            return;
        };

        let children = self.children();
        let columns = columns.snapshot();

        if columns.len() != children.count() {
            self.remove_cells();

            for column in columns.iter() {
                let cell = TableViewCell::new();
                cell.set_column(Some(column.clone()));
                children.add(cell);
            }

            StyledElement::logical_children(&row)
                .add_range(children.snapshot().iter().map(|child| child.clone().upcast::<StyledElement>()));
        } else {
            for (child, column) in children.snapshot().iter().zip(columns.iter()) {
                Self::cell(child).set_column(Some(column.clone()));
            }
        }

        self.invalidate_measure();
    }

    pub(crate) fn refresh_cell(&self, column_index: usize) {
        Self::cell(&self.children().get(column_index)).refresh();
    }

    /// A child of the presenter as the cell it has to be.
    fn cell(child: &Ref<crate::Control>) -> &TableViewCell {
        child.downcast_ref::<TableViewCell>().unwrap_or_else(|| {
            panic!(
                "Unable to cast object of type '{}' to type '{}'.",
                child.get_type().name(),
                <TableViewCell as ferroui_base::StaticType>::TYPE.name()
            )
        })
    }
}
