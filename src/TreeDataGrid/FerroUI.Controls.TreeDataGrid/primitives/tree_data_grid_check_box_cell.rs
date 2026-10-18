//! `Primitives/TreeDataGridCheckBoxCell.cs`: a cell that shows and edits the value of a check box
//! cell model.

use super::{TreeDataGridCell, TreeDataGridCellImpl, TreeDataGridCellImplExt, TreeDataGridElementFactory};
use crate::models::{CellRef, CheckBoxCell};
use crate::selection::ITreeDataGridSelectionInteraction;
use ferroui_base::input::{InputElementImpl, InputElementImplExt, PointerPressedEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::ControlImpl;
use std::cell::Cell;
use std::rc::Rc;

/// A cell that shows a [`CheckBoxCell`] model.
#[repr(C)]
pub struct TreeDataGridCheckBoxCell {
    base: TreeDataGridCell,
    is_read_only: Cell<bool>,
    is_three_state: Cell<bool>,
    value: Cell<Option<bool>>,
}

ferro_class!(TreeDataGridCheckBoxCell: TreeDataGridCell);
ferroui_base::ferro_class_info!(TreeDataGridCheckBoxCell { new: TreeDataGridCheckBoxCell::new });
ferroui_base::ferro_impl_classes!(
    TreeDataGridCheckBoxCell: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    TemplatedControlImpl
);

ferroui_base::ferro_properties! { impl TreeDataGridCheckBoxCell {
    ferro_property!(
        /// Defines the `IsReadOnly` property.
        pub fn is_read_only_property() -> DirectProperty<TreeDataGridCheckBoxCell, bool> {
            FerroProperty::register_direct::<TreeDataGridCheckBoxCell, _>(
                "IsReadOnly",
                |o| o.is_read_only(),
                Some(|o, v| o.set_is_read_only(v)),
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `IsThreeState` property.
        pub fn is_three_state_property() -> DirectProperty<TreeDataGridCheckBoxCell, bool> {
            FerroProperty::register_direct::<TreeDataGridCheckBoxCell, _>(
                "IsThreeState",
                |o| o.is_three_state(),
                Some(|o, v| o.set_is_three_state(v)),
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `Value` property.
        pub fn value_property() -> DirectProperty<TreeDataGridCheckBoxCell, Option<bool>> {
            FerroProperty::register_direct::<TreeDataGridCheckBoxCell, _>(
                "Value",
                |o| o.value(),
                Some(|o, v| o.set_value(v)),
                None,
            )
        }
    );
} }

impl InputElementImpl for TreeDataGridCheckBoxCell {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
    }
}

impl ControlImpl for TreeDataGridCheckBoxCell {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        crate::automation::peers::TreeDataGridCheckBoxCellAutomationPeer::new(this).upcast()
    }
}

impl TreeDataGridCellImpl for TreeDataGridCheckBoxCell {
    fn realize(
        this: &Self,
        factory: &Rc<TreeDataGridElementFactory>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        model: &CellRef,
        column_index: i32,
        row_index: i32,
    ) {
        match model.as_any().downcast_ref::<CheckBoxCell>() {
            Some(cell) => {
                this.set_is_read_only(cell.is_read_only());
                this.set_is_three_state(cell.is_three_state());
                this.set_value(cell.value());
            }
            None => panic!("Invalid cell model."),
        }

        Self::parent_realize(this, factory, selection, model, column_index, row_index);
        this.subscribe_to_model_changes();
    }

    fn unrealize(this: &Self) {
        this.unsubscribe_from_model_changes();
        Self::parent_unrealize(this);
    }

    fn on_model_property_changed(this: &Self, property_name: &str) {
        Self::parent_on_model_property_changed(this, property_name);

        if property_name == "Value" {
            if let Some(model) = this.model() {
                if let Some(cell) = model.as_any().downcast_ref::<CheckBoxCell>() {
                    this.set_value(cell.value());
                }
            }
        }
    }
}

impl TreeDataGridCheckBoxCell {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TreeDataGridCell::construct(),
            is_read_only: Cell::new(false),
            is_three_state: Cell::new(false),
            value: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the value cannot be changed.
    pub fn is_read_only(&self) -> bool {
        self.is_read_only.get()
    }

    pub fn set_is_read_only(&self, value: bool) {
        self.set_and_raise_cell(Self::is_read_only_property(), &self.is_read_only, value);
    }

    /// Whether the check box has an indeterminate state.
    pub fn is_three_state(&self) -> bool {
        self.is_three_state.get()
    }

    pub fn set_is_three_state(&self, value: bool) {
        self.set_and_raise_cell(Self::is_three_state_property(), &self.is_three_state, value);
    }

    /// The value of the cell. Setting it writes the value to the model.
    pub fn value(&self) -> Option<bool> {
        self.value.get()
    }

    pub fn set_value(&self, value: Option<bool>) {
        if self.set_and_raise_cell(Self::value_property(), &self.value, value) {
            if let Some(model) = self.model() {
                if let Some(cell) = model.as_any().downcast_ref::<CheckBoxCell>() {
                    cell.set_value(value);
                }
            }
            self.raise_cell_value_changed();
        }
    }
}
