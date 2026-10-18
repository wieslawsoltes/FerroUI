//! `Primitives/TreeDataGridRow.cs`: a row of the control, which hosts the cells presenter of its
//! template, shows the selection of the row and starts row drags.

use super::{TreeDataGridCellsPresenter, TreeDataGridElementFactory, TreeDataGridRowUnrealizeReason};
use crate::models::{IColumns, ITreeDataGridRows};
use crate::selection::ITreeDataGridSelectionInteraction;
use crate::TreeDataGrid;
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Point, Ref, StyledElementImpl, StyledElementImplExt, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::metadata::PseudoClassesAttribute;
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const DRAG_DISTANCE: f64 = 3.0;
const INVALID_POINT: Point = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);

/// A row of a [`TreeDataGrid`].
#[repr(C)]
pub struct TreeDataGridRow {
    base: TemplatedControl,
    columns: RefCell<Option<Rc<dyn IColumns>>>,
    element_factory: RefCell<Option<Rc<TreeDataGridElementFactory>>>,
    is_selected: Cell<bool>,
    rows: RefCell<Option<Rc<dyn ITreeDataGridRows>>>,
    mouse_down_position: Cell<Point>,
    pressed_event_args: RefCell<Option<PointerPressedEventArgs>>,
    tree_data_grid: RefCell<Option<WeakRef<TreeDataGrid>>>,
    cells_presenter: RefCell<Option<Ref<TreeDataGridCellsPresenter>>>,
    row_index: Cell<i32>,
}

ferro_class! {
    TreeDataGridRow: TemplatedControl, virtuals TreeDataGridRowImpl: TemplatedControlImpl {
        /// Called after this row has been realized and its primary cells presenter has been
        /// updated. `row_index` is the realized row index.
        fn on_realized(this, row_index: i32);
        /// Called after the index of this realized row and its primary cells has changed.
        fn on_row_index_changed(this, old_row_index: i32, new_row_index: i32);
        /// Called before this row and its primary cells presenter are unrealized. `row_index` is
        /// the row index that is about to be cleared and `reason` the reason the row is being
        /// unrealized.
        fn on_unrealizing(this, row_index: i32, reason: TreeDataGridRowUnrealizeReason);
    }
}
ferroui_base::ferro_class_info!(TreeDataGridRow { new: TreeDataGridRow::new });
ferroui_base::ferro_impl_classes!(TreeDataGridRow: LayoutableImpl, InteractiveImpl);

ferroui_base::ferro_properties! { impl TreeDataGridRow {
    ferro_property!(
        /// Defines the `Columns` property.
        pub fn columns_property() -> DirectProperty<TreeDataGridRow, Option<Rc<dyn IColumns>>> {
            FerroProperty::register_direct::<TreeDataGridRow, _>("Columns", |o| o.columns(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `ElementFactory` property.
        pub fn element_factory_property() -> DirectProperty<TreeDataGridRow, Option<Rc<TreeDataGridElementFactory>>> {
            FerroProperty::register_direct::<TreeDataGridRow, _>(
                "ElementFactory",
                |o| o.element_factory(),
                Some(|o, v| o.set_element_factory(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `IsSelected` property.
        pub fn is_selected_property() -> DirectProperty<TreeDataGridRow, bool> {
            FerroProperty::register_direct::<TreeDataGridRow, _>("IsSelected", |o| o.is_selected(), None, false)
        }
    );

    ferro_property!(
        /// Defines the `Rows` property.
        pub fn rows_property() -> DirectProperty<TreeDataGridRow, Option<Rc<dyn ITreeDataGridRows>>> {
            FerroProperty::register_direct::<TreeDataGridRow, _>("Rows", |o| o.rows(), None, None)
        }
    );
} }

impl FerroObjectImpl for TreeDataGridRow {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::is_selected_property().as_property() {
            this.pseudo_classes().set(":selected", this.is_selected());
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl StyledElementImpl for TreeDataGridRow {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        *this.tree_data_grid.borrow_mut() =
            this.find_logical_ancestor_of_type::<TreeDataGrid>(false).map(|tree_data_grid| tree_data_grid.downgrade());
        Self::parent_on_attached_to_logical_tree(this, e);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        *this.tree_data_grid.borrow_mut() = None;
        Self::parent_on_detached_from_logical_tree(this, e);
    }
}

impl VisualImpl for TreeDataGridRow {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        // The row may be realized before being parented. In this case raise the RowPrepared event
        // here.
        if let Some(tree_data_grid) = this.tree_data_grid() {
            if this.row_index.get() >= 0 {
                tree_data_grid.raise_row_prepared(this, this.row_index.get());
            }
        }
    }
}

impl InputElementImpl for TreeDataGridRow {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if !e.handled() {
            this.mouse_down_position.set(e.get_position(Some(this)));
            *this.pressed_event_args.borrow_mut() = Some(e.clone());
        } else {
            this.mouse_down_position.set(INVALID_POINT);
            *this.pressed_event_args.borrow_mut() = None;
        }
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let current_point = e.get_current_point(Some(this));
        let mouse_down_position = this.mouse_down_position.get();
        let delta_x = current_point.position.x - mouse_down_position.x;
        let delta_y = current_point.position.y - mouse_down_position.y;

        let pointer_supports_drag = match current_point.pointer.type_() {
            PointerType::Mouse => current_point.properties.is_left_button_pressed,
            PointerType::Pen => current_point.properties.is_right_button_pressed,
            _ => false,
        };

        if !pointer_supports_drag
            || e.handled()
            || (delta_x.abs() < DRAG_DISTANCE && delta_y.abs() < DRAG_DISTANCE)
            || mouse_down_position == INVALID_POINT
            || this.pressed_event_args.borrow().is_none()
        {
            return;
        }

        let pressed_event_args = this.pressed_event_args.borrow_mut().take();
        this.mouse_down_position.set(INVALID_POINT);

        let presenter = this.parent().and_then(|parent| parent.cast::<super::TreeDataGridRowsPresenter>());
        let owner = presenter
            .and_then(|presenter| presenter.templated_parent())
            .and_then(|templated_parent| templated_parent.cast::<TreeDataGrid>());
        if let (Some(owner), Some(pressed_event_args)) = (owner, pressed_event_args) {
            owner.raise_row_drag_started(&pressed_event_args);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        this.mouse_down_position.set(INVALID_POINT);
        *this.pressed_event_args.borrow_mut() = None;
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        this.mouse_down_position.set(INVALID_POINT);
        *this.pressed_event_args.borrow_mut() = None;
    }
}

impl ControlImpl for TreeDataGridRow {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        crate::automation::peers::TreeDataGridRowAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for TreeDataGridRow {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let old_cells_presenter = this.cells_presenter.borrow_mut().take();
        if let Some(old_cells_presenter) = old_cells_presenter {
            this.logical_children().remove(&old_cells_presenter.upcast());
        }

        Self::parent_on_apply_template(this, e);
        let cells_presenter = e.name_scope().find_as::<TreeDataGridCellsPresenter>("PART_CellsPresenter");
        *this.cells_presenter.borrow_mut() = cells_presenter.clone();

        if let Some(cells_presenter) = &cells_presenter {
            this.logical_children().add(cells_presenter.clone().upcast());
        }

        if this.row_index.get() >= 0 {
            if let Some(cells_presenter) = &cells_presenter {
                cells_presenter.realize(this.row_index.get());
            }
        }
    }
}

impl TreeDataGridRowImpl for TreeDataGridRow {
    fn on_realized(_this: &Self, _row_index: i32) {}

    fn on_row_index_changed(_this: &Self, _old_row_index: i32, _new_row_index: i32) {}

    fn on_unrealizing(_this: &Self, _row_index: i32, _reason: TreeDataGridRowUnrealizeReason) {}
}

impl TreeDataGridRow {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":selected"]);

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            columns: RefCell::new(None),
            element_factory: RefCell::new(None),
            is_selected: Cell::new(false),
            rows: RefCell::new(None),
            mouse_down_position: Cell::new(INVALID_POINT),
            pressed_event_args: RefCell::new(None),
            tree_data_grid: RefCell::new(None),
            cells_presenter: RefCell::new(None),
            row_index: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The columns of the row.
    pub fn columns(&self) -> Option<Rc<dyn IColumns>> {
        self.columns.borrow().clone()
    }

    fn set_columns(&self, value: Option<Rc<dyn IColumns>>) {
        self.set_and_raise(Self::columns_property(), &self.columns, value);
    }

    /// The factory of the cells of the row.
    pub fn element_factory(&self) -> Option<Rc<TreeDataGridElementFactory>> {
        self.element_factory.borrow().clone()
    }

    pub fn set_element_factory(&self, value: Option<Rc<TreeDataGridElementFactory>>) {
        self.set_and_raise(Self::element_factory_property(), &self.element_factory, value);
    }

    /// Whether the row is selected.
    pub fn is_selected(&self) -> bool {
        self.is_selected.get()
    }

    fn set_is_selected(&self, value: bool) {
        self.set_and_raise_cell(Self::is_selected_property(), &self.is_selected, value);
    }

    /// The model of the row: its data context.
    pub fn model(&self) -> Option<BoxedValue> {
        self.data_context()
    }

    /// The rows the row belongs to.
    pub fn rows(&self) -> Option<Rc<dyn ITreeDataGridRows>> {
        self.rows.borrow().clone()
    }

    fn set_rows(&self, value: Option<Rc<dyn ITreeDataGridRows>>) {
        self.set_and_raise(Self::rows_property(), &self.rows, value);
    }

    /// The cells presenter of the row's template.
    pub fn cells_presenter(&self) -> Option<Ref<TreeDataGridCellsPresenter>> {
        self.cells_presenter.borrow().clone()
    }

    /// The index of the row, or -1 when it is not realized.
    pub fn row_index(&self) -> i32 {
        self.row_index.get()
    }

    fn tree_data_grid(&self) -> Option<Ref<TreeDataGrid>> {
        self.tree_data_grid.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Realizes the row for the row at `row_index` of `rows`.
    pub fn realize(
        &self,
        element_factory: Option<&Rc<TreeDataGridElementFactory>>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        columns: Option<&Rc<dyn IColumns>>,
        rows: Option<&Rc<dyn ITreeDataGridRows>>,
        row_index: i32,
    ) {
        self.set_element_factory(element_factory.cloned());
        self.set_columns(columns.cloned());
        self.set_rows(rows.cloned());
        self.set_data_context(rows.and_then(|rows| rows.get(row_index as usize).model()));
        self.set_is_selected(selection.is_some_and(|selection| selection.is_row_selected_at(row_index)));
        self.row_index.set(row_index);
        self.update_selection(selection);
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.realize(row_index);
        }
        self.on_realized(row_index);
        if let Some(tree_data_grid) = self.tree_data_grid() {
            tree_data_grid.raise_row_prepared(self, self.row_index.get());
        }
    }

    /// The realized cell of the column at `column_index`, if any.
    pub fn try_get_cell(&self, column_index: i32) -> Option<Ref<Control>> {
        self.cells_presenter().and_then(|cells_presenter| cells_presenter.try_get_element(column_index))
    }

    /// The index of the realized row changed.
    pub fn update_index(&self, index: i32) {
        if self.row_index.get() == -1 {
            panic!("Row is not realized.");
        }

        let old_index = self.row_index.get();
        self.row_index.set(index);
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.update_row_index(index);
        }
        self.on_row_index_changed(old_index, index);
    }

    /// Unrealizes the row, to be recycled for another row.
    pub fn unrealize(&self) {
        if let Some(tree_data_grid) = self.tree_data_grid() {
            tree_data_grid.raise_row_clearing(self, self.row_index.get());
        }
        self.on_unrealizing(self.row_index.get(), TreeDataGridRowUnrealizeReason::Recycle);
        self.row_index.set(-1);
        self.set_data_context(None);
        self.set_is_selected(false);
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.unrealize();
        }
    }

    /// Unrealizes the row because its item was removed from the source.
    pub fn unrealize_on_item_removed(&self) {
        if let Some(tree_data_grid) = self.tree_data_grid() {
            tree_data_grid.raise_row_clearing(self, self.row_index.get());
        }
        self.on_unrealizing(self.row_index.get(), TreeDataGridRowUnrealizeReason::ItemRemoved);
        self.row_index.set(-1);
        self.set_data_context(None);
        self.set_is_selected(false);
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.unrealize_on_row_removed();
        }
    }

    /// Releases the cells kept for a rebind when the row was not rebound (`FinalizeUnrealize`).
    pub fn finalize_unrealize(&self) {
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.finalize_unrealize();
        }
    }

    /// Updates the selection state of the row and its cells (`UpdateSelection`).
    pub fn update_selection(&self, selection: Option<&dyn ITreeDataGridSelectionInteraction>) {
        self.set_is_selected(selection.is_some_and(|selection| selection.is_row_selected_at(self.row_index.get())));
        if let Some(cells_presenter) = self.cells_presenter() {
            cells_presenter.update_selection(selection);
        }
    }
}
