//! Port of `Core/AttachedToLogicalTreeBehavior.cs` (`AttachedToLogicalTreeBehavior<T>`).

use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StaticType, StyledElement, StyledElementImpl, TypeInfo};
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A base class for behaviors using attached to logical tree event.
///
/// The class is abstract. The type argument `T` of the reference (a styled element) is the
/// runtime type passed to [`construct`](Self::construct) (see
/// [`BehaviorOfT`](ferroui_xaml_interactivity::BehaviorOfT)).
#[repr(C)]
pub struct AttachedToLogicalTreeBehavior {
    base: DisposingBehavior,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    AttachedToLogicalTreeBehavior: DisposingBehavior,
    virtuals AttachedToLogicalTreeBehaviorImpl: DisposingBehaviorImpl {
        /// Called after the associated object is attached to the logical tree.
        ///
        /// Returns a disposable resource to be disposed when the behavior is detached.
        fn on_attached_to_logical_tree_override(this) -> Rc<dyn IDisposable>;
    }
}

ferroui_base::ferro_impl_classes!(AttachedToLogicalTreeBehavior: FerroObjectImpl, StyledElementImpl);

impl AttachedToLogicalTreeBehaviorImpl for AttachedToLogicalTreeBehavior {
    fn on_attached_to_logical_tree_override(_this: &Self) -> Rc<dyn IDisposable> {
        panic!(
            "AttachedToLogicalTreeBehavior is abstract: 'on_attached_to_logical_tree_override' must be implemented by \
             the deriving class"
        )
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for AttachedToLogicalTreeBehavior {
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
    impl StyledElementBehaviorImpl for AttachedToLogicalTreeBehavior {
        fn on_attached_to_logical_tree_event(this: &Self) {
            let disposable = this.on_attached_to_logical_tree_override();
            *this.disposable.borrow_mut() = Some(disposable);
        }
    }
}

impl AttachedToLogicalTreeBehavior {
    /// Creates the class data for behaviors attached to styled elements of `associated_type` (the
    /// `T` of the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        debug_assert!(
            <StyledElement as StaticType>::TYPE.is_assignable_from(associated_type),
            "the associated type of an AttachedToLogicalTreeBehavior is a styled element"
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
