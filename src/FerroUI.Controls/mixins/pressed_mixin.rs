use crate::Control;
use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{ObjectType, Upcast};

/// Adds pressed functionality to control classes.
///
/// Adds the `:pressed` class when the item is pressed.
pub struct PressedMixin;

impl PressedMixin {
    /// Attaches the pressed functionality to the control class `TControl`.
    pub fn attach<TControl: ObjectType + Upcast<Control>>() {
        // The handlers live as long as the events they are attached to.
        let _ = InputElement::pointer_pressed_event().add_class_handler_with::<TControl>(
            |x, e| Self::handle_pointer_pressed(Upcast::<Control>::upcast(x), e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let _ = InputElement::pointer_released_event().add_class_handler_with::<TControl>(
            |x, _| Self::handle_pointer_released(Upcast::<Control>::upcast(x)),
            RoutingStrategies::TUNNEL,
            false,
        );
        let _ = InputElement::pointer_capture_lost_event().add_class_handler_with::<TControl>(
            |x, _| Self::handle_pointer_released(Upcast::<Control>::upcast(x)),
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    fn handle_pointer_pressed(sender: &Control, e: &PointerPressedEventArgs) {
        if e.get_current_point(Some(sender)).properties.is_left_button_pressed {
            sender.pseudo_classes().set(":pressed", true);
        }
    }

    fn handle_pointer_released(sender: &Control) {
        sender.pseudo_classes().set(":pressed", false);
    }
}
