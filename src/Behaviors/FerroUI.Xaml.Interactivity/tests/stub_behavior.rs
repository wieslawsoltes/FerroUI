//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/StubBehavior.cs` of Xaml.Behaviors.

use crate::{ActionCollection, IBehavior, Interaction};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref};
use std::cell::{Cell, RefCell};

/// A behavior that counts its attachments and detachments: a plain object that implements the
/// behavior contract itself.
#[repr(C)]
pub(crate) struct StubBehavior {
    base: FerroObject,
    attach_count: Cell<i32>,
    detach_count: Cell<i32>,
    actions: ActionCollection,
    associated_object: RefCell<Option<Ref<FerroObject>>>,
}

ferro_class!(StubBehavior: FerroObject);
ferroui_base::ferro_impl_classes!(StubBehavior: FerroObjectImpl);

impl StubBehavior {
    /// The class implements the behavior contract.
    fn static_constructor() {
        <dyn IBehavior>::register::<StubBehavior>();
    }

    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            attach_count: Cell::new(0),
            detach_count: Cell::new(0),
            actions: ActionCollection::new(),
            associated_object: RefCell::new(None),
        })
    }

    pub(crate) fn attach_count(&self) -> i32 {
        self.attach_count.get()
    }

    pub(crate) fn detach_count(&self) -> i32 {
        self.detach_count.get()
    }

    #[allow(dead_code)]
    pub(crate) fn actions(&self) -> ActionCollection {
        self.actions.clone()
    }

    pub(crate) fn associated_object(&self) -> Option<Ref<FerroObject>> {
        self.associated_object.borrow().clone()
    }

    #[allow(dead_code)]
    pub(crate) fn execute(&self, sender: Option<&BoxedValue>, parameter: &BoxedValue) -> Vec<BoxedValue> {
        Interaction::execute_actions(sender, Some(&self.actions), Some(parameter))
    }
}

impl IBehavior for StubBehavior {
    fn associated_object(&self) -> Option<Ref<FerroObject>> {
        StubBehavior::associated_object(self)
    }

    fn attach(&self, associated_object: Option<Ref<FerroObject>>) {
        *self.associated_object.borrow_mut() = associated_object;
        self.attach_count.set(self.attach_count.get() + 1);
    }

    fn detach(&self) {
        *self.associated_object.borrow_mut() = None;
        self.detach_count.set(self.detach_count.get() + 1);
    }
}
