//! Port of `Cursor/PointerOverCursorBehavior.cs`.

use ferroui_base::input::{Cursor, InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::Cell;
use std::rc::Rc;

/// Changes the cursor when the pointer is over the associated control.
#[repr(C)]
pub struct PointerOverCursorBehavior {
    base: StyledElementBehaviorOfT,
    // The handlers are removed by their tokens (a closure has no identity to remove it by, as the
    // reference removes its methods).
    handlers: Cell<Option<(RoutedEventHandlerToken, RoutedEventHandlerToken)>>,
}

ferro_class!(PointerOverCursorBehavior: StyledElementBehaviorOfT);
ferro_class_info!(PointerOverCursorBehavior { new: PointerOverCursorBehavior::new });
ferroui_base::ferro_impl_classes!(PointerOverCursorBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl PointerOverCursorBehavior {
        /// Identifies the `Cursor` property.
        pub fn cursor_property() -> StyledProperty<Option<Rc<Cursor>>> {
            FerroProperty::register::<PointerOverCursorBehavior, _>("Cursor", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerOverCursorBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<InputElement>() else {
                return;
            };

            this.remove_handlers(&associated_object);

            let behavior = this.to_ref().downgrade();
            let entered = associated_object.add_handler_with(
                InputElement::pointer_entered_event(),
                move |_sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_entered(e);
                    }
                },
                RoutingStrategies::DIRECT,
                false,
            );

            let behavior = this.to_ref().downgrade();
            let exited = associated_object.add_handler_with(
                InputElement::pointer_exited_event(),
                move |_sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_exited(e);
                    }
                },
                RoutingStrategies::DIRECT,
                false,
            );

            this.handlers.set(Some((entered, exited)));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<InputElement>() {
                this.remove_handlers(&associated_object);
            }
        }
    }
}

impl PointerOverCursorBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<InputElement as StaticType>::TYPE), handlers: Cell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the cursor to apply while the pointer is over the control.
    pub fn cursor(&self) -> Option<Rc<Cursor>> {
        self.get_value(Self::cursor_property())
    }

    pub fn set_cursor(&self, value: Option<Rc<Cursor>>) {
        self.set_value(Self::cursor_property(), value)
    }

    fn remove_handlers(&self, associated_object: &InputElement) {
        if let Some((entered, exited)) = self.handlers.take() {
            associated_object.remove_handler(InputElement::pointer_entered_event(), entered);
            associated_object.remove_handler(InputElement::pointer_exited_event(), exited);
        }
    }

    fn on_pointer_entered(&self, _e: &PointerEventArgs) {
        if let (Some(associated_object), Some(cursor)) = (self.associated_object_as::<InputElement>(), self.cursor()) {
            associated_object.set_current_value(InputElement::cursor_property(), Some(cursor));
        }
    }

    fn on_pointer_exited(&self, _e: &PointerEventArgs) {
        if let Some(associated_object) = self.associated_object_as::<InputElement>() {
            associated_object.clear_value(InputElement::cursor_property());
        }
    }
}
