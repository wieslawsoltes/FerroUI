//! Port of `Core/AttachedToVisualTreeTriggerBase.cs` (`AttachedToVisualTreeTriggerBase<T>`).

use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StaticType, StyledElementImpl, TypeInfo, Visual};
use ferroui_xaml_interactivity::{DisposableAction, DisposingTrigger, DisposingTriggerImpl, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// A trigger that executes when the associated object is attached to the visual tree.
///
/// The resource [`on_attached_to_visual_tree_override`](AttachedToVisualTreeTriggerBaseImpl::on_attached_to_visual_tree_override)
/// creates each time the associated object is attached to the visual tree is disposed when it
/// leaves the visual tree or the trigger is detached.
///
/// The class is abstract. The type argument `T` of the reference (a visual) is the runtime type
/// passed to [`construct`](Self::construct) (see
/// [`BehaviorOfT`](ferroui_xaml_interactivity::BehaviorOfT)).
#[repr(C)]
pub struct AttachedToVisualTreeTriggerBase {
    base: DisposingTrigger,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    AttachedToVisualTreeTriggerBase: DisposingTrigger,
    virtuals AttachedToVisualTreeTriggerBaseImpl: DisposingTriggerImpl {
        /// Called after the associated object is attached to the visual tree.
        ///
        /// Returns a resource disposed when the associated object leaves the visual tree or the
        /// trigger is detached.
        fn on_attached_to_visual_tree_override(this) -> Rc<dyn IDisposable>;
    }
}

ferroui_base::ferro_impl_classes!(AttachedToVisualTreeTriggerBase: FerroObjectImpl, StyledElementImpl);

impl AttachedToVisualTreeTriggerBaseImpl for AttachedToVisualTreeTriggerBase {
    fn on_attached_to_visual_tree_override(_this: &Self) -> Rc<dyn IDisposable> {
        panic!(
            "AttachedToVisualTreeTriggerBase is abstract: 'on_attached_to_visual_tree_override' must be implemented \
             by the deriving class"
        )
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingTriggerImpl for AttachedToVisualTreeTriggerBase {
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
    impl StyledElementBehaviorImpl for AttachedToVisualTreeTriggerBase {
        fn on_attached_to_visual_tree(this: &Self) {
            this.on_delayed_dispose();
            let disposable = this.on_attached_to_visual_tree_override();
            *this.disposable.borrow_mut() = Some(disposable);
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.on_delayed_dispose();
        }
    }
}

impl AttachedToVisualTreeTriggerBase {
    /// Creates the class data for triggers attached to visuals of `associated_type` (the `T` of
    /// the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        debug_assert!(
            <Visual as StaticType>::TYPE.is_assignable_from(associated_type),
            "the associated type of an AttachedToVisualTreeTriggerBase is a visual"
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
