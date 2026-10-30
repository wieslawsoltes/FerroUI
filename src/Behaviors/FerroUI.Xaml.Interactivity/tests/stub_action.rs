//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/StubAction.cs` of Xaml.Behaviors.

use crate::{StyledElementAction, StyledElementActionImpl};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use std::cell::{Cell, RefCell};

/// An action that records its execution and returns a fixed value.
#[repr(C)]
pub(crate) struct StubAction {
    base: StyledElementAction,
    return_value: Option<BoxedValue>,
    sender: RefCell<Option<BoxedValue>>,
    parameter: RefCell<Option<BoxedValue>>,
    execute_count: Cell<i32>,
}

ferro_class!(StubAction: StyledElementAction);
ferroui_base::ferro_impl_classes!(StubAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for StubAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        this.execute_count.set(this.execute_count.get() + 1);
        *this.sender.borrow_mut() = sender.cloned();
        *this.parameter.borrow_mut() = parameter.cloned();
        this.return_value.clone()
    }
}

impl StubAction {
    /// `new StubAction()`.
    pub(crate) fn new() -> Ref<Self> {
        Self::with_return_value(None)
    }

    /// `new StubAction(returnValue)`.
    pub(crate) fn with_return_value(return_value: Option<BoxedValue>) -> Ref<Self> {
        instantiate(Self {
            base: StyledElementAction::construct(),
            return_value,
            sender: RefCell::new(None),
            parameter: RefCell::new(None),
            execute_count: Cell::new(0),
        })
    }

    pub(crate) fn sender(&self) -> Option<BoxedValue> {
        self.sender.borrow().clone()
    }

    pub(crate) fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.borrow().clone()
    }

    pub(crate) fn execute_count(&self) -> i32 {
        self.execute_count.get()
    }
}
