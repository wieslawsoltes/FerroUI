//! Port of `Triggers/DisposingTrigger.cs` (`DisposingTrigger<T>`).

use crate::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTriggerOfT};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl, TypeInfo};
use std::cell::RefCell;
use std::rc::Rc;

/// A base class for triggers that require a disposable resource: the resource
/// [`on_attached_override`](DisposingTriggerImpl::on_attached_override) creates is disposed when
/// the trigger is detached.
///
/// The class is abstract. The type argument `T` of the reference is the runtime type passed to
/// [`construct`](Self::construct) (see [`BehaviorOfT`](crate::BehaviorOfT)).
#[repr(C)]
pub struct DisposingTrigger {
    base: StyledElementTriggerOfT,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    DisposingTrigger: StyledElementTriggerOfT, virtuals DisposingTriggerImpl: StyledElementBehaviorImpl {
        /// Called after the trigger is attached to the associated object.
        ///
        /// Returns a disposable resource to be disposed when the trigger is detached.
        fn on_attached_override(this) -> Rc<dyn IDisposable>;
    }
}

ferroui_base::ferro_impl_classes!(DisposingTrigger: FerroObjectImpl, StyledElementImpl);

impl DisposingTriggerImpl for DisposingTrigger {
    fn on_attached_override(_this: &Self) -> Rc<dyn IDisposable> {
        panic!("DisposingTrigger is abstract: 'on_attached_override' must be implemented by the deriving class")
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DisposingTrigger {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            let previous = this.disposable.borrow_mut().take();
            if let Some(previous) = previous {
                previous.dispose();
            }
            let disposable = this.on_attached_override();
            *this.disposable.borrow_mut() = Some(disposable);
        }

        fn on_detaching(this: &Self) {
            Self::parent_on_detaching(this);

            let disposable = this.disposable.borrow().clone();
            if let Some(disposable) = disposable {
                disposable.dispose();
            }
        }
    }
}

impl DisposingTrigger {
    /// Creates the class data for triggers attached to objects of `associated_type`; see
    /// [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        Self { base: StyledElementTriggerOfT::construct(associated_type), disposable: RefCell::new(None) }
    }
}
