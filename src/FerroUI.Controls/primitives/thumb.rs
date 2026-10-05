use super::{TemplatedControl, TemplatedControlImpl};
use crate::metadata::PseudoClassesAttribute;
use crate::ControlImpl;
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, VectorEventArgs,
};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_routed_event, instantiate, FerroObjectImpl, Point,
    Ref, StyledElementImpl, Vector, VisualImpl,
};
use std::cell::Cell;

const PC_PRESSED: &str = ":pressed";

/// A control that can be dragged by the user.
#[repr(C)]
pub struct Thumb {
    base: TemplatedControl,
    last_point: Cell<Option<Point>>,
}

ferro_class! {
    Thumb: TemplatedControl, virtuals ThumbImpl: TemplatedControlImpl {
        /// Invoked when an unhandled `DragStarted` event reaches an element
        /// in its route that is derived from this class. Implement this
        /// method to add class handling for this event.
        fn on_drag_started(this, e: &VectorEventArgs);
        /// Invoked when an unhandled `DragDelta` event reaches an element in
        /// its route that is derived from this class. Implement this method
        /// to add class handling for this event.
        fn on_drag_delta(this, e: &VectorEventArgs);
        /// Invoked when an unhandled `DragCompleted` event reaches an element
        /// in its route that is derived from this class. Implement this
        /// method to add class handling for this event.
        fn on_drag_completed(this, e: &VectorEventArgs);
    }
}
ferroui_base::ferro_class_info!(Thumb { new: Thumb::new });

ferro_impl_classes!(Thumb: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, TemplatedControlImpl);

impl ControlImpl for Thumb {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ThumbAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for Thumb {}

impl InputElementImpl for Thumb {
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        if let Some(last_point) = this.last_point.get() {
            let mut ev = VectorEventArgs::new();
            ev.set_routed_event(Some(Self::drag_completed_event()));
            ev.vector = last_point.into();

            this.last_point.set(None);

            this.raise_event(&ev);
        }

        this.pseudo_classes().remove_pseudo(PC_PRESSED);

        Self::parent_on_pointer_capture_lost(this, e);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        if let Some(last_point) = this.last_point.get() {
            let mut ev = VectorEventArgs::new();
            ev.set_routed_event(Some(Self::drag_delta_event()));
            ev.vector = (e.get_position(Some(this)) - last_point).into();

            this.raise_event(&ev);
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        e.set_handled(true);
        let last_point = e.get_position(Some(this));
        this.last_point.set(Some(last_point));

        let mut ev = VectorEventArgs::new();
        ev.set_routed_event(Some(Self::drag_started_event()));
        ev.vector = last_point.into();

        this.pseudo_classes().add_pseudo(PC_PRESSED);

        e.prevent_gesture_recognition();

        this.raise_event(&ev);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.last_point.get().is_some() {
            e.set_handled(true);
            this.last_point.set(None);

            let mut ev = VectorEventArgs::new();
            ev.set_routed_event(Some(Self::drag_completed_event()));
            ev.vector = e.get_position(Some(this)).into();

            this.raise_event(&ev);
        }

        this.pseudo_classes().remove_pseudo(PC_PRESSED);
    }
}

impl ThumbImpl for Thumb {
    fn on_drag_started(_this: &Self, _e: &VectorEventArgs) {}

    fn on_drag_delta(_this: &Self, _e: &VectorEventArgs) {}

    fn on_drag_completed(_this: &Self, _e: &VectorEventArgs) {}
}

impl Thumb {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_PRESSED]);

    ferro_routed_event!(
        /// Defines the `DragStarted` routed event.
        pub fn drag_started_event() -> RoutedEvent<VectorEventArgs> {
            RoutedEvent::register::<Thumb, _>("DragStarted", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `DragDelta` routed event.
        pub fn drag_delta_event() -> RoutedEvent<VectorEventArgs> {
            RoutedEvent::register::<Thumb, _>("DragDelta", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `DragCompleted` routed event.
        pub fn drag_completed_event() -> RoutedEvent<VectorEventArgs> {
            RoutedEvent::register::<Thumb, _>("DragCompleted", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        Self::drag_started_event().add_class_handler_with::<Thumb>(
            |x, e| x.on_drag_started(e),
            RoutingStrategies::BUBBLE,
            false,
        );
        Self::drag_delta_event().add_class_handler_with::<Thumb>(
            |x, e| x.on_drag_delta(e),
            RoutingStrategies::BUBBLE,
            false,
        );
        Self::drag_completed_event().add_class_handler_with::<Thumb>(
            |x, e| x.on_drag_completed(e),
            RoutingStrategies::BUBBLE,
            false,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct(), last_point: Cell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when a drag of the thumb starts.
    pub fn drag_started(&self, handler: impl Fn(&Interactive, &VectorEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::drag_started_event(), handler)
    }

    /// Raised while the thumb is dragged.
    pub fn drag_delta(&self, handler: impl Fn(&Interactive, &VectorEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::drag_delta_event(), handler)
    }

    /// Raised when a drag of the thumb completes.
    pub fn drag_completed(
        &self,
        handler: impl Fn(&Interactive, &VectorEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::drag_completed_event(), handler)
    }

    /// Moves the reference point of the drag in progress by a vector.
    pub(crate) fn adjust_drag(&self, v: Vector) {
        if let Some(last_point) = self.last_point.get() {
            self.last_point.set(Some(last_point + v));
        }
    }
}
