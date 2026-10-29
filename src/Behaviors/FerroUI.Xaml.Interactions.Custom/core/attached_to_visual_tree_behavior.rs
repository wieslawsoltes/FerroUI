//! Port of `Core/AttachedToVisualTreeBehavior.cs` (`AttachedToVisualTreeBehavior<T>`).

use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StaticType, StyledElementImpl, TypeInfo, Visual};
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A base class for behaviors using attached to visual tree event.
///
/// The resource [`on_attached_to_visual_tree_override`](AttachedToVisualTreeBehaviorImpl::on_attached_to_visual_tree_override)
/// creates each time the associated object is attached to the visual tree is disposed when it
/// leaves the visual tree or the behavior is detached.
///
/// The class is abstract. The type argument `T` of the reference (a visual) is the runtime type
/// passed to [`construct`](Self::construct) (see
/// [`BehaviorOfT`](ferroui_xaml_interactivity::BehaviorOfT)).
#[repr(C)]
pub struct AttachedToVisualTreeBehavior {
    base: DisposingBehavior,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    AttachedToVisualTreeBehavior: DisposingBehavior,
    virtuals AttachedToVisualTreeBehaviorImpl: DisposingBehaviorImpl {
        /// Called after the associated object is attached to the visual tree.
        ///
        /// Returns a resource disposed when the associated object leaves the visual tree or the
        /// behavior is detached.
        fn on_attached_to_visual_tree_override(this) -> Rc<dyn IDisposable>;
    }
}

ferroui_base::ferro_impl_classes!(AttachedToVisualTreeBehavior: FerroObjectImpl, StyledElementImpl);

impl AttachedToVisualTreeBehaviorImpl for AttachedToVisualTreeBehavior {
    fn on_attached_to_visual_tree_override(_this: &Self) -> Rc<dyn IDisposable> {
        panic!(
            "AttachedToVisualTreeBehavior is abstract: 'on_attached_to_visual_tree_override' must be implemented by \
             the deriving class"
        )
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for AttachedToVisualTreeBehavior {
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
    impl StyledElementBehaviorImpl for AttachedToVisualTreeBehavior {
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

impl AttachedToVisualTreeBehavior {
    /// Creates the class data for behaviors attached to visuals of `associated_type` (the `T` of
    /// the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        debug_assert!(
            <Visual as StaticType>::TYPE.is_assignable_from(associated_type),
            "the associated type of an AttachedToVisualTreeBehavior is a visual"
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
