//! Port of `Core/DataContextChangedBehavior.cs` (`DataContextChangedBehavior<T>`).

use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StaticType, StyledElement, StyledElementImpl, TypeInfo};
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A base class for behaviors using DataContextChanged event.
///
/// The class is abstract. The type argument `T` of the reference (a styled element) is the runtime
/// type passed to [`construct`](Self::construct) (see
/// [`BehaviorOfT`](ferroui_xaml_interactivity::BehaviorOfT)).
#[repr(C)]
pub struct DataContextChangedBehavior {
    base: DisposingBehavior,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    DataContextChangedBehavior: DisposingBehavior,
    virtuals DataContextChangedBehaviorImpl: DisposingBehaviorImpl {
        /// Called when the `DataContextChanged` event of the associated object is raised.
        ///
        /// Returns a disposable resource to be disposed when the behavior is detached.
        fn on_data_context_changed_event_override(this) -> Rc<dyn IDisposable>;
    }
}

ferroui_base::ferro_impl_classes!(DataContextChangedBehavior: FerroObjectImpl, StyledElementImpl);

impl DataContextChangedBehaviorImpl for DataContextChangedBehavior {
    fn on_data_context_changed_event_override(_this: &Self) -> Rc<dyn IDisposable> {
        panic!("DataContextChangedBehavior is abstract: 'on_data_context_changed_event_override' must be implemented by the deriving class")
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for DataContextChangedBehavior {
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
    impl StyledElementBehaviorImpl for DataContextChangedBehavior {
        fn on_data_context_changed_event(this: &Self) {
            // The reference replaces the resource without disposing the previous one.
            let disposable = this.on_data_context_changed_event_override();
            *this.disposable.borrow_mut() = Some(disposable);
        }
    }
}

impl DataContextChangedBehavior {
    /// Creates the class data for behaviors attached to objects of `associated_type` (the `T` of
    /// the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        debug_assert!(
            <StyledElement as StaticType>::TYPE.is_assignable_from(associated_type),
            "the associated type of a DataContextChangedBehavior is a styled element"
        );
        Self { base: DisposingBehavior::construct(associated_type), disposable: RefCell::new(None) }
    }

    fn on_delayed_dispose(&self) {
        let disposable = self.disposable.borrow_mut().take();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }
}
