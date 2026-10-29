//! Port of `Core/LoadedBehavior.cs` (`LoadedBehavior<T>`).

use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StaticType, StyledElementImpl, TypeInfo};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A base class for behaviors using loaded event.
///
/// The class is abstract. The type argument `T` of the reference (a control) is the runtime
/// type passed to [`construct`](Self::construct) (see
/// [`BehaviorOfT`](ferroui_xaml_interactivity::BehaviorOfT)).
#[repr(C)]
pub struct LoadedBehavior {
    base: DisposingBehavior,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    LoadedBehavior: DisposingBehavior,
    virtuals LoadedBehaviorImpl: DisposingBehaviorImpl {
        /// Called after the associated object is loaded.
        ///
        /// Returns a disposable resource to be disposed when the behavior is detached.
        fn on_loaded_override(this) -> Rc<dyn IDisposable>;
    }
}

ferroui_base::ferro_impl_classes!(LoadedBehavior: FerroObjectImpl, StyledElementImpl);

impl LoadedBehaviorImpl for LoadedBehavior {
    fn on_loaded_override(_this: &Self) -> Rc<dyn IDisposable> {
        panic!("LoadedBehavior is abstract: 'on_loaded_override' must be implemented by the deriving class")
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for LoadedBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_delayed_dispose();
                }
            })
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for LoadedBehavior {
        fn on_loaded(this: &Self) {
            // The reference replaces the resource without disposing the previous one.
            let disposable = this.on_loaded_override();
            *this.disposable.borrow_mut() = Some(disposable);
        }
    }
}

impl LoadedBehavior {
    /// Creates the class data for behaviors attached to objects of `associated_type` (the `T` of
    /// the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        debug_assert!(
            <Control as StaticType>::TYPE.is_assignable_from(associated_type),
            "the associated type of a LoadedBehavior is a control"
        );
        Self { base: DisposingBehavior::construct(associated_type), disposable: RefCell::new(None) }
    }

    fn on_delayed_dispose(&self) {
        // The reference keeps the disposed resource (it does not clear the field).
        let disposable = self.disposable.borrow().clone();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }
}
