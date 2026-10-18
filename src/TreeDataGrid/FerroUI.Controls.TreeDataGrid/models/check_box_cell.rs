//! `Models/TreeDataGrid/CheckBoxCell.cs`: a cell that shows a two- or three-state check box.

use super::{BeginEditGestures, ICell, ICellBinding, IRecyclableCell};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::models::NotifyingBase;
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

/// `CheckBoxCell`.
pub struct CheckBoxCell {
    notifying: NotifyingBase,
    binding: Option<Rc<dyn ICellBinding<Option<bool>>>>,
    subscription: Cell<Option<u64>>,
    value: Cell<Option<bool>>,
    is_read_only: bool,
    is_three_state: bool,
}

impl CheckBoxCell {
    /// A read-only cell of a constant value (`CheckBoxCell(bool? value)`).
    pub fn from_value(value: Option<bool>) -> Rc<Self> {
        Rc::new(Self {
            notifying: NotifyingBase::new(),
            binding: None,
            subscription: Cell::new(None),
            value: Cell::new(value),
            is_read_only: true,
            is_three_state: false,
        })
    }

    /// A cell of the value `binding` pushes; a cell that is not read only writes its changes back.
    pub fn new(binding: Rc<dyn ICellBinding<Option<bool>>>, is_read_only: bool, is_three_state: bool) -> Rc<Self> {
        let this = Rc::new(Self {
            notifying: NotifyingBase::new(),
            binding: Some(binding.clone()),
            subscription: Cell::new(None),
            value: Cell::new(None),
            is_read_only,
            is_three_state,
        });
        let weak = Rc::downgrade(&this);
        let token = binding.subscribe(Rc::new(move |value: &Option<bool>| {
            if let Some(this) = weak.upgrade() {
                this.set_value(*value);
            }
        }));
        this.subscription.set(Some(token));
        this
    }

    /// Whether a single tap edits the cell (`SingleTapEdit`).
    pub fn single_tap_edit(&self) -> bool {
        false
    }

    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// Whether the check box has an indeterminate state.
    pub fn is_three_state(&self) -> bool {
        self.is_three_state
    }

    /// The state of the check box.
    pub fn value(&self) -> Option<bool> {
        self.value.get()
    }

    /// Sets the state; a cell that is not read only writes it to its binding.
    pub fn set_value(&self, value: Option<bool>) {
        if self.notifying.raise_and_set_cell_if_changed(&self.value, value, "Value") && !self.is_read_only {
            if let Some(binding) = &self.binding {
                binding.on_next(value);
            }
        }
    }

    /// Points the cell's binding at another model (`TrySetSource`).
    pub fn try_set_source(&self, source: Option<&BoxedValue>) -> bool {
        self.binding
            .as_ref()
            .and_then(|binding| binding.as_retargetable())
            .is_some_and(|retargetable| retargetable.try_set_root(source))
    }
}

impl INotifyPropertyChanged for CheckBoxCell {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl ICell for CheckBoxCell {
    fn can_edit(&self) -> bool {
        false
    }

    fn edit_gestures(&self) -> BeginEditGestures {
        BeginEditGestures::NONE
    }

    fn value(&self) -> Option<BoxedValue> {
        self.value.get().map(|value| Rc::new(value) as BoxedValue)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        Some(self)
    }

    fn as_recyclable(&self) -> Option<&dyn IRecyclableCell> {
        Some(self)
    }

    fn dispose(&self) {
        if let (Some(binding), Some(token)) = (&self.binding, self.subscription.take()) {
            binding.unsubscribe(token);
        }
    }
}

impl IRecyclableCell for CheckBoxCell {
    fn try_suspend(&self) -> bool {
        let suspended = self
            .binding
            .as_ref()
            .and_then(|binding| binding.as_retargetable())
            .is_some_and(|retargetable| retargetable.try_suspend_root());
        if !suspended {
            return false;
        }

        self.value.set(None);
        true
    }
}
