use super::platform::IPlatformDragSource;
use super::{DragDropEffects, DragEventArgs, IDataTransfer, LocalBoxFuture, PointerPressedEventArgs};
use crate::interactivity::{Interactive, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies};
use crate::{
    ferro_property, ferro_routed_event, AttachedProperty, FerroLocator, FerroProperty, LocatorExtensions,
    StyledPropertyOptions,
};
use std::rc::Rc;

/// Defines the attached property and the routed events of drag-and-drop,
/// and starts drag-and-drop operations.
pub struct DragDrop;

crate::ferro_static_type!(DragDrop);

impl DragDrop {
    ferro_routed_event!(
        /// Event which is raised, when a drag-and-drop operation enters the
        /// element.
        pub fn drag_enter_event() -> RoutedEvent<DragEventArgs> {
            RoutedEvent::register::<DragDrop, _>("DragEnter", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Event which is raised, when a drag-and-drop operation leaves the
        /// element.
        pub fn drag_leave_event() -> RoutedEvent<DragEventArgs> {
            RoutedEvent::register::<DragDrop, _>("DragLeave", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Event which is raised, when a drag-and-drop operation is updated
        /// while over the element.
        pub fn drag_over_event() -> RoutedEvent<DragEventArgs> {
            RoutedEvent::register::<DragDrop, _>("DragOver", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Event which is raised, when a drag-and-drop operation should
        /// complete over the element.
        pub fn drop_event() -> RoutedEvent<DragEventArgs> {
            RoutedEvent::register::<DragDrop, _>("Drop", RoutingStrategies::BUBBLE)
        }
    );

}

crate::ferro_properties! { impl DragDrop {
    ferro_property!(
        /// Defines the `AllowDrop` attached property: whether the element
        /// can be the target of a drag-and-drop operation. It is inherited.
        pub fn allow_drop_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<DragDrop, Interactive, _>(
                "AllowDrop",
                StyledPropertyOptions::new(false).inherits(true),
            )
        }
    );
} }

impl DragDrop {
    /// Gets a value indicating whether the given element can be used as
    /// the target of a drag-and-drop operation.
    pub fn get_allow_drop(interactive: &Interactive) -> bool {
        interactive.get_value(Self::allow_drop_property())
    }

    /// Sets a value indicating whether the given element can be used as
    /// the target of a drag-and-drop operation.
    pub fn set_allow_drop(interactive: &Interactive, value: bool) {
        interactive.set_value(Self::allow_drop_property(), value)
    }

    /// Adds a handler for the DragEnter attached event.
    pub fn add_drag_enter_handler(
        element: &Interactive,
        handler: impl Fn(&Interactive, &DragEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        element.add_handler(Self::drag_enter_event(), handler)
    }

    /// Removes a handler for the DragEnter attached event.
    pub fn remove_drag_enter_handler(element: &Interactive, handler: RoutedEventHandlerToken) {
        element.remove_handler(Self::drag_enter_event(), handler);
    }

    /// Adds a handler for the DragLeave attached event.
    pub fn add_drag_leave_handler(
        element: &Interactive,
        handler: impl Fn(&Interactive, &DragEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        element.add_handler(Self::drag_leave_event(), handler)
    }

    /// Removes a handler for the DragLeave attached event.
    pub fn remove_drag_leave_handler(element: &Interactive, handler: RoutedEventHandlerToken) {
        element.remove_handler(Self::drag_leave_event(), handler);
    }

    /// Adds a handler for the DragOver attached event.
    pub fn add_drag_over_handler(
        element: &Interactive,
        handler: impl Fn(&Interactive, &DragEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        element.add_handler(Self::drag_over_event(), handler)
    }

    /// Removes a handler for the DragOver attached event.
    pub fn remove_drag_over_handler(element: &Interactive, handler: RoutedEventHandlerToken) {
        element.remove_handler(Self::drag_over_event(), handler);
    }

    /// Adds a handler for the Drop attached event.
    pub fn add_drop_handler(
        element: &Interactive,
        handler: impl Fn(&Interactive, &DragEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        element.add_handler(Self::drop_event(), handler)
    }

    /// Removes a handler for the Drop attached event.
    pub fn remove_drop_handler(element: &Interactive, handler: RoutedEventHandlerToken) {
        element.remove_handler(Self::drop_event(), handler);
    }

    /// Starts a dragging operation with the given data transfer and
    /// returns its result.
    ///
    /// `trigger_event` is the pointer pressed event that triggered the
    /// operation. `data_transfer` is automatically disposed when the
    /// operation completes: it must NOT be disposed by the caller.
    /// `allowed_effects` are the effects the source allows.
    pub fn do_drag_drop_async(
        trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        let Some(drag_source) = FerroLocator::current().get_service::<dyn IPlatformDragSource>() else {
            data_transfer.dispose();
            return Box::pin(std::future::ready(DragDropEffects::NONE));
        };

        drag_source.do_drag_drop_async(trigger_event, data_transfer, allowed_effects)
    }
}
