//! Port of `Core/AttachedToLogicalTreeTriggerBase.cs` (`AttachedToLogicalTreeTriggerBase<T>`).

use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StaticType, StyledElementImpl, TypeInfo, Visual};
use ferroui_xaml_interactivity::{DisposableAction, DisposingTrigger, DisposingTriggerImpl, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// A trigger that executes when the associated object is attached to the logical tree.
///
/// The class is abstract. The type argument `T` of the reference (a visual) is the runtime type
/// passed to [`construct`](Self::construct) (see
/// [`BehaviorOfT`](ferroui_xaml_interactivity::BehaviorOfT)).
#[repr(C)]
pub struct AttachedToLogicalTreeTriggerBase {
    base: DisposingTrigger,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    AttachedToLogicalTreeTriggerBase: DisposingTrigger,
    virtuals AttachedToLogicalTreeTriggerBaseImpl: DisposingTriggerImpl {
        /// Called after the associated object is attached to the logical tree.
        ///
        /// Returns a disposable resource to be disposed when the trigger is detached.
        fn on_attached_to_logical_tree_override(this) -> Rc<dyn IDisposable>;
    }
}

ferroui_base::ferro_impl_classes!(AttachedToLogicalTreeTriggerBase: FerroObjectImpl, StyledElementImpl);

impl AttachedToLogicalTreeTriggerBaseImpl for AttachedToLogicalTreeTriggerBase {
    fn on_attached_to_logical_tree_override(_this: &Self) -> Rc<dyn IDisposable> {
        panic!(
            "AttachedToLogicalTreeTriggerBase is abstract: 'on_attached_to_logical_tree_override' must be implemented \
             by the deriving class"
        )
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingTriggerImpl for AttachedToLogicalTreeTriggerBase {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let trigger = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_delayed_dispose();
                }
            })
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AttachedToLogicalTreeTriggerBase {
        fn on_attached_to_logical_tree_event(this: &Self) {
            let disposable = this.on_attached_to_logical_tree_override();
            *this.disposable.borrow_mut() = Some(disposable);
        }
    }
}

impl AttachedToLogicalTreeTriggerBase {
    /// Creates the class data for triggers attached to visuals of `associated_type` (the `T` of
    /// the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        debug_assert!(
            <Visual as StaticType>::TYPE.is_assignable_from(associated_type),
            "the associated type of an AttachedToLogicalTreeTriggerBase is a visual"
        );
        Self { base: DisposingTrigger::construct(associated_type), disposable: RefCell::new(None) }
    }

    fn on_delayed_dispose(&self) {
        let disposable = self.disposable.borrow_mut().take();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }
}
