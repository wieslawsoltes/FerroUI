//! `Primitives/TreeDataGridCell.cs`: the base class of the cell controls: binding to a cell model,
//! the selection and editing state, and the gestures that start editing.
//!
//! The edits are those of the cell model (`IEditableObject`, [`ICell::as_editable`]).

use super::{INaturalWidthMeasureCache, ITreeDataGridCell, TreeDataGridElementFactory, TreeDataGridRow};
use crate::models::{BeginEditGestures, CellRef, ColumnRef};
use crate::selection::ITreeDataGridSelectionInteraction;
use crate::TreeDataGrid;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, TappedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_property, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl, StyledElementImplExt, Thickness,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::metadata::PseudoClassesAttribute;
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::ControlImpl;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

const INVALID_POINT: Point = Point::new(f64::NAN, f64::NAN);

/// The base class of the cell controls of a [`TreeDataGrid`].
#[repr(C)]
pub struct TreeDataGridCell {
    base: TemplatedControl,
    is_selected: Cell<bool>,
    tree_data_grid: RefCell<Option<WeakRef<TreeDataGrid>>>,
    pressed_point: Cell<Point>,
    column_index: Cell<i32>,
    row_index: Cell<i32>,
    is_editing: Cell<bool>,
    model: RefCell<Option<CellRef>>,
    recycling_column: RefCell<Option<ColumnRef>>,
    natural_desired_size: Cell<Option<Size>>,
    /// The model whose property changes the cell follows, and the token of the handler.
    model_subscription: RefCell<Option<(CellRef, u64)>>,
    /// The handler the cell subscribes to the property changes of its models with, made once and
    /// added again at every subscription.
    model_changed_handler: OnceCell<Rc<dyn Fn(&str)>>,
}

ferro_class! {
    TreeDataGridCell: TemplatedControl, virtuals TreeDataGridCellImpl: TemplatedControlImpl {
        /// Begins a synchronous replacement of this cell's row. The presenter still calls
        /// `unrealize` and `realize`, including their normal clearing/prepared events. This hook is
        /// not called when a cell is merely returned to the recycle pool.
        fn begin_rebind(this);
        /// Ends the synchronous replacement, including when a lifecycle callback panics.
        /// `realized` is true only when `realize` completed successfully. On false, release any
        /// presentation state retained by `begin_rebind`. Do not assume the model or indexes were
        /// reset if a callback failed partway through.
        fn end_rebind(this, realized: bool);
        /// Binds the cell to `model`, the cell of the column at `column_index` in the row at
        /// `row_index`.
        fn realize(
            this,
            factory: &Rc<TreeDataGridElementFactory>,
            selection: Option<&dyn ITreeDataGridSelectionInteraction>,
            model: &CellRef,
            column_index: i32,
            row_index: i32
        );
        /// Releases the cell's model.
        fn unrealize(this);
        /// Reads the value of the model again (`UpdateValue`).
        fn update_value(this);
        /// A property of the model changed (`OnModelPropertyChanged`); `property_name` is the name
        /// of the property, empty when all properties changed.
        fn on_model_property_changed(this, property_name: &str);
    }
}
ferroui_base::ferro_impl_classes!(TreeDataGridCell: InteractiveImpl, TemplatedControlImpl);

ferroui_base::ferro_properties! { impl TreeDataGridCell {
    ferro_property!(
        /// Defines the `IsSelected` property.
        pub fn is_selected_property() -> DirectProperty<TreeDataGridCell, bool> {
            FerroProperty::register_direct::<TreeDataGridCell, _>("IsSelected", |o| o.is_selected(), None, false)
        }
    );
} }

impl FerroObjectImpl for TreeDataGridCell {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::is_selected_property().as_property() {
            this.pseudo_classes().set(":selected", this.is_selected());
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl StyledElementImpl for TreeDataGridCell {
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

impl VisualImpl for TreeDataGridCell {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        // The cell may be realized before being parented. In this case raise the CellPrepared
        // event here.
        if let Some(tree_data_grid) = this.tree_data_grid() {
            if this.column_index.get() >= 0 && this.row_index.get() >= 0 {
                tree_data_grid.raise_cell_prepared(this, this.column_index.get(), this.row_index.get());
            }
        }
    }
}

impl LayoutableImpl for TreeDataGridCell {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let result = Self::parent_measure_override(this, available_size);

        // HACKFIX for #83 of the reference. Seems that cells are getting truncated at times due to
        // DPI scaling; a pixel is added to the cell size.
        result.inflate(Thickness::new(1.0, 0.0, 1.0, 0.0))
    }

    fn on_measure_invalidated(this: &Self) {
        this.natural_desired_size.set(None);
        Self::parent_on_measure_invalidated(this);
    }
}

impl InputElementImpl for TreeDataGridCell {
    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);

        if !this.is_keyboard_focus_within() && this.is_editing.get() {
            this.end_edit();
        }
    }

    fn on_double_tapped(this: &Self, e: &TappedEventArgs) {
        Self::parent_on_double_tapped(this, e);

        if let Some(model) = this.model() {
            if !e.handled()
                && !this.is_editing.get()
                && model.can_edit()
                && this.is_enabled_edit_gesture(BeginEditGestures::DOUBLE_TAP, model.edit_gestures())
            {
                this.begin_edit();
                e.set_handled(true);
            }
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        let Some(model) = this.model() else { return };
        if e.handled() {
            return;
        }

        if e.key == Key::F2
            && !this.is_editing.get()
            && model.can_edit()
            && this.is_enabled_edit_gesture(BeginEditGestures::F2, model.edit_gestures())
        {
            this.begin_edit();
            e.set_handled(true);
        } else if e.key == Key::Return && this.is_editing.get() {
            this.end_edit();
            e.set_handled(true);
        } else if e.key == Key::Escape && this.is_editing.get() {
            this.cancel_edit();
            e.set_handled(true);
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        let model = this.model();
        if model.as_ref().is_some_and(|model| {
            !e.handled()
                && !this.is_editing.get()
                && model.can_edit()
                && this.is_enabled_edit_gesture(BeginEditGestures::TAP, model.edit_gestures())
        }) {
            this.pressed_point.set(e.get_current_point(None).position);
            e.set_handled(true);
        } else {
            this.pressed_point.set(INVALID_POINT);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        let model = this.model();
        if model.as_ref().is_some_and(|model| {
            !e.handled()
                && !this.is_editing.get()
                && !this.pressed_point.get().x.is_nan()
                && model.can_edit()
                && this.is_enabled_edit_gesture(BeginEditGestures::TAP, model.edit_gestures())
        }) {
            let point = e.get_current_point(Some(this));
            let settings = this.get_platform_settings();
            let tap_size = settings.map_or(Size::new(4.0, 4.0), |settings| settings.get_tap_size(point.pointer.type_()));
            let tap_rect = Rect::from_position_size(this.pressed_point.get(), Size::default())
                .inflate_thickness(Thickness::new(tap_size.width, tap_size.height, tap_size.width, tap_size.height));

            if Rect::from_size(this.bounds().size()).contains_exclusive(point.position)
                && tap_rect.contains_exclusive(e.get_current_point(None).position)
            {
                this.begin_edit();
                e.set_handled(true);
            }
        }

        this.pressed_point.set(INVALID_POINT);
    }
}

impl ControlImpl for TreeDataGridCell {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        crate::automation::peers::TreeDataGridCellAutomationPeer::new(this).upcast()
    }
}

impl TreeDataGridCellImpl for TreeDataGridCell {
    fn begin_rebind(_this: &Self) {}

    fn end_rebind(_this: &Self, _realized: bool) {}

    fn realize(
        this: &Self,
        _factory: &Rc<TreeDataGridElementFactory>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        model: &CellRef,
        column_index: i32,
        row_index: i32,
    ) {
        if this.column_index.get() >= 0 || this.row_index.get() >= 0 {
            panic!("Cell is already realized.");
        }
        if column_index < 0 {
            panic!("Invalid column index.");
        }
        if row_index < 0 {
            panic!("Invalid row index.");
        }

        this.column_index.set(column_index);
        this.row_index.set(row_index);
        *this.model.borrow_mut() = Some(model.clone());
        this.natural_desired_size.set(None);
        this.set_is_selected(selection.is_some_and(|selection| selection.is_cell_selected(column_index, row_index)));

        if let Some(tree_data_grid) = this.tree_data_grid() {
            tree_data_grid.raise_cell_prepared(this, column_index, this.row_index.get());
        }
    }

    fn unrealize(this: &Self) {
        if let Some(tree_data_grid) = this.tree_data_grid() {
            tree_data_grid.raise_cell_clearing(this, this.column_index.get(), this.row_index.get());
        }
        this.column_index.set(-1);
        this.row_index.set(-1);
        *this.model.borrow_mut() = None;
        *this.recycling_column.borrow_mut() = None;
        this.natural_desired_size.set(None);
    }

    fn update_value(_this: &Self) {}

    fn on_model_property_changed(_this: &Self, _property_name: &str) {}
}

impl INaturalWidthMeasureCache for TreeDataGridCell {
    fn natural_desired_size(&self) -> Option<Size> {
        self.natural_desired_size.get()
    }

    fn set_natural_desired_size(&self, value: Option<Size>) {
        self.natural_desired_size.set(value);
    }
}

impl ITreeDataGridCell for TreeDataGridCell {
    fn column_index(&self) -> i32 {
        self.column_index.get()
    }

    fn realize(
        &self,
        factory: &Rc<TreeDataGridElementFactory>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        model: &CellRef,
        column_index: i32,
        row_index: i32,
    ) {
        TreeDataGridCell::realize(self, factory, selection, model, column_index, row_index);
    }

    fn unrealize(&self) {
        TreeDataGridCell::unrealize(self);
    }
}

impl TreeDataGridCell {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":editing", ":selected"]);

    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<TreeDataGridCell>(true);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            is_selected: Cell::new(false),
            tree_data_grid: RefCell::new(None),
            pressed_point: Cell::new(INVALID_POINT),
            column_index: Cell::new(-1),
            row_index: Cell::new(-1),
            is_editing: Cell::new(false),
            model: RefCell::new(None),
            recycling_column: RefCell::new(None),
            natural_desired_size: Cell::new(None),
            model_subscription: RefCell::new(None),
            model_changed_handler: OnceCell::new(),
        }
    }

    /// The index of the cell's column, or -1 when the cell is not realized.
    pub fn column_index(&self) -> i32 {
        self.column_index.get()
    }

    /// The index of the cell's row, or -1 when the cell is not realized.
    pub fn row_index(&self) -> i32 {
        self.row_index.get()
    }

    /// Whether the cell is being edited.
    pub fn is_editing(&self) -> bool {
        self.is_editing.get()
    }

    /// The cell model the cell is bound to.
    pub fn model(&self) -> Option<CellRef> {
        self.model.borrow().clone()
    }

    /// The column the cell was realized for, which its model is returned to when it is recycled
    /// (`RecyclingColumn`).
    pub(crate) fn recycling_column(&self) -> Option<ColumnRef> {
        self.recycling_column.borrow().clone()
    }

    pub(crate) fn set_recycling_column(&self, value: Option<ColumnRef>) {
        *self.recycling_column.borrow_mut() = value;
    }

    /// Whether the cell is selected.
    pub fn is_selected(&self) -> bool {
        self.is_selected.get()
    }

    fn set_is_selected(&self, value: bool) {
        self.set_and_raise_cell(Self::is_selected_property(), &self.is_selected, value);
    }

    /// Whether the cell or its row is selected.
    pub fn is_effectively_selected(&self) -> bool {
        self.is_selected() || self.find_ancestor_of_type::<TreeDataGridRow>(false).is_some_and(|row| row.is_selected())
    }

    fn tree_data_grid(&self) -> Option<Ref<TreeDataGrid>> {
        self.tree_data_grid.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Begins editing the cell.
    pub fn begin_edit(&self) {
        if !self.is_editing.get() {
            self.is_editing.set(true);
            if let Some(model) = self.model() {
                if let Some(editable) = model.as_editable() {
                    editable.begin_edit();
                }
            }
            self.pseudo_classes().add_pseudo(":editing");
        }
    }

    /// Ends editing the cell, discarding the edit.
    pub fn cancel_edit(&self) {
        if self.end_edit_core() {
            if let Some(model) = self.model() {
                if let Some(editable) = model.as_editable() {
                    editable.cancel_edit();
                }
            }
        }
    }

    /// Ends editing the cell, committing the edit.
    pub fn end_edit(&self) {
        if self.end_edit_core() {
            if let Some(model) = self.model() {
                if let Some(editable) = model.as_editable() {
                    editable.end_edit();
                    self.update_value();
                    self.raise_cell_value_changed();
                }
            }
        }
    }

    /// Follows the property changes of the model (`SubscribeToModelChanges`).
    pub fn subscribe_to_model_changes(&self) {
        let Some(model) = self.model() else { return };
        let Some(notifier) = model.as_notify_property_changed() else { return };

        let handler = self.model_changed_handler.get_or_init(|| {
            let weak = self.to_ref().downgrade();
            Rc::new(move |property_name: &str| {
                if let Some(this) = weak.upgrade() {
                    this.on_model_property_changed(property_name);
                }
            })
        });
        let token = notifier.property_changed().add(handler.clone());
        let previous = self.model_subscription.borrow_mut().replace((model.clone(), token));
        if let Some((previous_model, previous_token)) = previous {
            if let Some(notifier) = previous_model.as_notify_property_changed() {
                notifier.property_changed().remove(previous_token);
            }
        }
    }

    /// Stops following the property changes of the model (`UnsubscribeFromModelChanges`).
    pub fn unsubscribe_from_model_changes(&self) {
        let subscription = self.model_subscription.borrow_mut().take();
        if let Some((model, token)) = subscription {
            if let Some(notifier) = model.as_notify_property_changed() {
                notifier.property_changed().remove(token);
            }
        }
    }

    /// Raises the control's `CellValueChanged` for the cell when it is realized and not being
    /// edited (`RaiseCellValueChanged`).
    pub fn raise_cell_value_changed(&self) {
        if !self.is_editing.get() && self.column_index.get() != -1 && self.row_index.get() != -1 {
            if let Some(tree_data_grid) = self.tree_data_grid() {
                tree_data_grid.raise_cell_value_changed(self, self.column_index.get(), self.row_index.get());
            }
        }
    }

    /// The row of the realized cell changed (`UpdateRowIndex`).
    pub fn update_row_index(&self, index: i32) {
        if self.row_index.get() == -1 {
            panic!("Cell is not realized.");
        }
        self.row_index.set(index);
    }

    /// Updates the selection state of the cell (`UpdateSelection`).
    pub fn update_selection(&self, selection: Option<&dyn ITreeDataGridSelectionInteraction>) {
        self.set_is_selected(selection.is_some_and(|selection| {
            selection.is_cell_selected(self.column_index.get(), self.row_index.get())
        }));
    }

    fn end_edit_core(&self) -> bool {
        if self.is_editing.get() {
            let restore_focus = self.is_keyboard_focus_within();
            self.is_editing.set(false);
            self.pseudo_classes().remove_pseudo(":editing");
            if restore_focus {
                self.focus();
            }
            return true;
        }

        false
    }

    fn is_enabled_edit_gesture(&self, gesture: BeginEditGestures, enabled_gestures: BeginEditGestures) -> bool {
        if !enabled_gestures.has_flag(gesture) {
            return false;
        }

        if enabled_gestures.has_flag(BeginEditGestures::WHEN_SELECTED) {
            self.is_effectively_selected()
        } else {
            true
        }
    }
}
