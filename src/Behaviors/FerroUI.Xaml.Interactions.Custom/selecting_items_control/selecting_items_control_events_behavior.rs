//! Port of `SelectingItemsControl/SelectingItemsControlEventsBehavior.cs`.

use ferroui_base::interactivity::Interactive;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StaticType, StyledElementImpl};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::SelectionChangedEventArgs;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// A base class for behaviors that handle the selection changed event of the associated
/// [`SelectingItemsControl`]: the event goes to
/// [`on_selection_changed`](SelectingItemsControlEventsBehaviorImpl::on_selection_changed).
///
/// The class is abstract.
#[repr(C)]
pub struct SelectingItemsControlEventsBehavior {
    base: DisposingBehavior,
}

ferro_class! {
    SelectingItemsControlEventsBehavior: DisposingBehavior,
    virtuals SelectingItemsControlEventsBehaviorImpl: DisposingBehaviorImpl {
        /// Called when the selection of the associated control changes. The `object? sender` of
        /// the reference is the element the handler is attached to.
        fn on_selection_changed(this, sender: &Interactive, e: &SelectionChangedEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(
    SelectingItemsControlEventsBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

impl SelectingItemsControlEventsBehaviorImpl for SelectingItemsControlEventsBehavior {
    fn on_selection_changed(_this: &Self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for SelectingItemsControlEventsBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(selecting_items_control) = this.associated_object_as::<SelectingItemsControl>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            let token = selecting_items_control.selection_changed(move |sender: &Interactive, e: &SelectionChangedEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_selection_changed(sender, e);
                }
            });

            let selecting_items_control = selecting_items_control.downgrade();
            DisposableAction::create(move || {
                if let Some(selecting_items_control) = selecting_items_control.upgrade() {
                    selecting_items_control.remove_handler(SelectingItemsControl::selection_changed_event(), token);
                }
            })
        }
    }
}

impl SelectingItemsControlEventsBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingBehavior::construct(<SelectingItemsControl as StaticType>::TYPE) }
    }
}
