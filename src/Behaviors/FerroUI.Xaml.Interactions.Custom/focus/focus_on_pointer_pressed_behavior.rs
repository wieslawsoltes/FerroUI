//! Port of `Focus/FocusOnPointerPressedBehavior.cs`.

use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::Cell;

/// Focuses the associated control when a pointer is pressed on it.
#[repr(C)]
pub struct FocusOnPointerPressedBehavior {
    base: StyledElementBehaviorOfT,
    // The handler added to the associated control (the reference removes its delegate).
    pointer_pressed_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(FocusOnPointerPressedBehavior: StyledElementBehaviorOfT);
ferro_class_info!(FocusOnPointerPressedBehavior { new: FocusOnPointerPressedBehavior::new });
ferroui_base::ferro_impl_classes!(FocusOnPointerPressedBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for FocusOnPointerPressedBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler(
                InputElement::pointer_pressed_event(),
                move |_sender: &Interactive, _e: &PointerPressedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_pressed();
                    }
                },
            );
            this.pointer_pressed_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            if let Some(handler) = this.pointer_pressed_handler.take() {
                associated_object.remove_handler(InputElement::pointer_pressed_event(), handler);
            }
        }
    }
}

impl FocusOnPointerPressedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            pointer_pressed_handler: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn pointer_pressed(&self) {
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
