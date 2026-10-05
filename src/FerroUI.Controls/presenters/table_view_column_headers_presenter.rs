use super::table_view_layout_helper::TableViewLayoutHelper;
use crate::{ControlImpl, Panel, PanelImpl, TableView, TableViewColumnHeader};
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, LayoutableImpl};
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl,
    StyledElementImplExt, VisualImpl, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Displays the column headers for a [`TableView`]. Computes column widths
/// (pixel and star) and exposes them via the actual width of the columns so
/// that the presenters of the cells can align cells with the headers.
#[repr(C)]
pub struct TableViewColumnHeadersPresenter {
    base: Panel,
    /// The table view the presenter belongs to: an ancestor, held weakly.
    table_view: RefCell<Option<WeakRef<TableView>>>,
    /// The scrollable whose property changes the presenter listens to, and
    /// the subscription.
    scroll_subscription: RefCell<Option<(WeakRef<InputElement>, Rc<dyn IDisposable>)>>,
}

ferro_class!(TableViewColumnHeadersPresenter: Panel);
ferro_class_info!(TableViewColumnHeadersPresenter { new: TableViewColumnHeadersPresenter::new });
ferro_impl_classes!(
    TableViewColumnHeadersPresenter: FerroObjectImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl StyledElementImpl for TableViewColumnHeadersPresenter {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_logical_tree(this, e);

        let mut table_view = None;
        let mut current = this.parent();
        while let Some(ancestor) = current {
            if let Some(found) = ancestor.clone().cast::<TableView>() {
                table_view = Some(found);
                break;
            }
            current = ancestor.parent();
        }

        this.set_table_view(table_view.as_ref());

        this.rebuild_headers();

        if let Some(table_view) = table_view {
            table_view.set_headers_presenter(Some(&this.to_ref()));

            if let Some(scroll) = table_view.scroll() {
                let weak = this.to_ref().downgrade();
                let subscription = scroll.property_changed(move |e| {
                    if e.property().name() == "Offset" {
                        if let Some(this) = weak.upgrade() {
                            this.invalidate_arrange();
                        }
                    }
                });
                *this.scroll_subscription.borrow_mut() = Some((scroll.downgrade(), subscription));
            }
        }
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        if let Some(table_view) = this.table_view() {
            table_view.set_headers_presenter(None);

            // As in the reference, only the scrollable the table view has
            // now is unsubscribed from.
            if let Some(scroll) = table_view.scroll() {
                let subscribed = this
                    .scroll_subscription
                    .borrow()
                    .as_ref()
                    .is_some_and(|(element, _)| element.upgrade().is_some_and(|element| element == scroll));
                if subscribed {
                    if let Some((_, subscription)) = this.scroll_subscription.take() {
                        subscription.dispose();
                    }
                }
            }

            this.set_table_view(None);
            this.rebuild_headers();
        }

        Self::parent_on_detached_from_logical_tree(this, e);
    }
}

impl LayoutableImpl for TableViewColumnHeadersPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let Some(table_view) = this.table_view() else { return Size::default() };

        let columns = table_view.columns();

        if TableViewLayoutHelper::update_actual_widths(
            &columns,
            available_size.width,
            this.use_layout_rounding(),
            LayoutHelper::get_layout_scale(this),
        ) {
            table_view.invalidate_cells_measure();
        }

        TableViewLayoutHelper::measure_row(&columns, &this.children(), available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let Some(table_view) = this.table_view() else { return final_size };

        let columns = table_view.columns();

        if TableViewLayoutHelper::update_actual_widths(
            &columns,
            final_size.width,
            this.use_layout_rounding(),
            LayoutHelper::get_layout_scale(this),
        ) {
            table_view.invalidate_cells_measure();
        }

        let offset = table_view
            .scroll()
            .and_then(|scroll| scroll.as_scrollable().map(|scrollable| scrollable.offset().x))
            .unwrap_or(0.0);
        TableViewLayoutHelper::arrange_row(&columns, &this.children(), final_size, -offset)
    }
}

impl TableViewColumnHeadersPresenter {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct(), table_view: RefCell::new(None), scroll_subscription: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub(crate) fn table_view(&self) -> Option<Ref<TableView>> {
        self.table_view.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_table_view(&self, value: Option<&Ref<TableView>>) {
        *self.table_view.borrow_mut() = value.map(Ref::downgrade);
    }

    pub(crate) fn rebuild_headers(&self) {
        let children = self.children();
        children.clear();

        let Some(table_view) = self.table_view() else { return };

        for column in table_view.columns().snapshot().iter() {
            let header = TableViewColumnHeader::new();
            header.set_column(Some(column.clone()));
            children.add(header);
        }

        self.invalidate_measure();
    }

    pub(crate) fn refresh_header(&self, column_index: usize) {
        let child = self.children().get(column_index);
        match child.downcast_ref::<TableViewColumnHeader>() {
            Some(header) => header.refresh(),
            None => panic!(
                "Unable to cast object of type '{}' to type '{}'.",
                child.get_type().name(),
                <TableViewColumnHeader as ferroui_base::StaticType>::TYPE.name()
            ),
        }
    }
}
