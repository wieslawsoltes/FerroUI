//! Port of `Window/WindowDragMoveBehavior.cs`.

use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType};
use ferroui_controls::{Control, TopLevel, Window};
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, BehaviorOfT};
use std::cell::Cell;

/// A behavior that allows the user to drag the window by clicking and holding the associated
/// control.
#[repr(C)]
pub struct WindowDragMoveBehavior {
    base: BehaviorOfT,
    pointer_pressed: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(WindowDragMoveBehavior: BehaviorOfT);
ferro_class_info!(WindowDragMoveBehavior { new: WindowDragMoveBehavior::new });
ferroui_base::ferro_impl_classes!(WindowDragMoveBehavior: FerroObjectImpl);

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for WindowDragMoveBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            if let Some(associated_object) = this.associated_object_as::<Control>() {
                let behavior = this.to_ref().downgrade();
                let token = associated_object.add_handler(
                    InputElement::pointer_pressed_event(),
                    move |_sender: &Interactive, e: &PointerPressedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.associated_object_pointer_pressed(e);
                        }
                    },
                );
                this.remove_pointer_pressed_handler();
                this.pointer_pressed.set(Some(token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            this.remove_pointer_pressed_handler();
        }
    }
}

impl WindowDragMoveBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: BehaviorOfT::construct(<Control as StaticType>::TYPE), pointer_pressed: Cell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    // The handler is removed by its token (a closure has no identity to remove it by, as the
    // reference removes its method).
    fn remove_pointer_pressed_handler(&self) {
        let Some(token) = self.pointer_pressed.take() else {
            return;
        };
        if let Some(associated_object) = self.associated_object_as::<Control>() {
            associated_object.remove_handler(InputElement::pointer_pressed_event(), token);
        }
    }

    fn associated_object_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        if let Some(associated_object) = self.associated_object_as::<Control>() {
            let window = TopLevel::get_top_level(Some(crate::part_c::control_as_visual(&associated_object)))
                .and_then(|top_level| top_level.cast::<Window>());
            if let Some(window) = window {
                window.begin_move_drag(e);
            }
        }
    }
}
