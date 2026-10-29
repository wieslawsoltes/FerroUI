//! Port of `Focus/FocusOnPointerMovedBehavior.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::Cell;

/// Focuses the associated control when the pointer moves over it.
#[repr(C)]
pub struct FocusOnPointerMovedBehavior {
    base: StyledElementBehaviorOfT,
    // The handler added to the associated control (the reference removes its delegate).
    pointer_moved_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(FocusOnPointerMovedBehavior: StyledElementBehaviorOfT);
ferro_class_info!(FocusOnPointerMovedBehavior { new: FocusOnPointerMovedBehavior::new });
ferroui_base::ferro_impl_classes!(FocusOnPointerMovedBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for FocusOnPointerMovedBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler(
                InputElement::pointer_moved_event(),
                move |_sender: &Interactive, _e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_moved();
                    }
                },
            );
            this.pointer_moved_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            if let Some(handler) = this.pointer_moved_handler.take() {
                associated_object.remove_handler(InputElement::pointer_moved_event(), handler);
            }
        }
    }
}

impl FocusOnPointerMovedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            pointer_moved_handler: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn pointer_moved(&self) {
        let behavior = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(associated_object) =
                    behavior.upgrade().and_then(|behavior| behavior.associated_object_as::<Control>())
                {
                    associated_object.focus();
                }
            },
            DispatcherPriority::DEFAULT,
        );
    }
}
