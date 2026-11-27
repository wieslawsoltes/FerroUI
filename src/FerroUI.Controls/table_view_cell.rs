use crate::primitives::TemplatedControlImpl;
use crate::{ContentControl, ContentControlImpl, ControlImpl, TableViewColumn, ToolTip};
use ferroui_base::data::CompiledBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObjectImpl,
    FerroProperty, Ref, StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// The binding of the content of a cell to the item of its row (the
    /// data context of the cell).
    static ROW_BINDING: Rc<CompiledBinding> = CompiledBinding::empty();
}

/// Represents a single cell in a [`TableViewRow`](crate::TableViewRow).
#[repr(C)]
pub struct TableViewCell {
    base: ContentControl,
    column: RefCell<Option<Ref<TableViewColumn>>>,
    /// Whether the tool tip is bound to the `CellToolTipBinding` of the column (WinUI).
    has_column_tool_tip: Cell<bool>,
}

ferro_class!(TableViewCell: ContentControl);
ferro_class_info!(TableViewCell { new: TableViewCell::new });
ferro_impl_classes!(
    TableViewCell: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

ferro_properties! {
    impl TableViewCell {
        /// Defines the `Column` property.
        pub fn column_property() -> DirectProperty<TableViewCell, Option<Ref<TableViewColumn>>> {
            FerroProperty::register_direct::<TableViewCell, _>("Column", |o| o.column(), None, None)
        }
    }
}

impl TableViewCell {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct(), column: RefCell::new(None), has_column_tool_tip: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the column associated with this cell.
    pub fn column(&self) -> Option<Ref<TableViewColumn>> {
        self.column.borrow().clone()
    }

    pub(crate) fn set_column(&self, value: Option<Ref<TableViewColumn>>) {
        let old_value = self.column();
        if !self.set_and_raise(Self::column_property(), &self.column, value.clone()) {
            return;
        }

        if old_value.is_some() {
            self.clear_properties();
        }

        if let Some(value) = value {
            self.set_properties(&value);
        }
    }

    fn clear_properties(&self) {
        if self.has_column_tool_tip.replace(false) {
            self.clear_value(ToolTip::tip_property());
        }
        self.clear_value(Visual::is_visible_property());
        self.clear_value(StyledElement::theme_property());
        self.clear_value(ContentControl::horizontal_content_alignment_property());
        self.clear_value(ContentControl::content_template_property());
        self.clear_value(ContentControl::content_property());
    }

    fn set_properties(&self, column: &TableViewColumn) {
        // We don't bind the various properties here.
        // First, it's pretty rare for a column's properties to change after
        // the initial setup.
        // Second, we have additional logic depending on whether a cell
        // template is specified.
        // Instead, values are updated manually via `refresh`.

        self.set_value(Visual::is_visible_property(), column.is_visible());
        self.set_value(StyledElement::theme_property(), column.cell_theme());
        self.set_value(ContentControl::horizontal_content_alignment_property(), column.horizontal_content_alignment());

        let content_property = ContentControl::content_property().as_property();
        let row_binding = ROW_BINDING.with(Rc::clone);

        if let Some(cell_template) = column.cell_template() {
            self.set_value(ContentControl::content_template_property(), Some(cell_template));
            self.bind_binding(content_property, &row_binding);
        } else {
            self.set_value(ContentControl::content_template_property(), None);
            match column.binding() {
                Some(binding) => self.bind_binding(content_property, &*binding),
                None => self.bind_binding(content_property, &row_binding),
            };
        }

        match column.cell_tool_tip_binding() {
            Some(binding) => {
                self.bind_binding(ToolTip::tip_property().as_property(), &*binding);
                self.has_column_tool_tip.set(true);
            }
            None if self.has_column_tool_tip.replace(false) => self.clear_value(ToolTip::tip_property()),
            None => {}
        }
    }

    pub(crate) fn refresh(&self) {
        if let Some(column) = self.column() {
            self.set_properties(&column);
        }
    }
}
