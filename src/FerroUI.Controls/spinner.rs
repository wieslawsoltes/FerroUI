use crate::primitives::TemplatedControlImpl;
use crate::{ContentControl, ContentControlImpl, ControlImpl};
use bitflags::bitflags;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, ferro_routed_event_args, FerroObjectImpl, FerroProperty, StyledElementImpl, StyledProperty, VisualImpl,
};

bitflags! {
    /// Represents spin directions that are valid.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct ValidSpinDirections: i32 {
        /// Can not increase nor decrease.
        const NONE = 0;

        /// Can increase.
        const INCREASE = 1;

        /// Can decrease.
        const DECREASE = 2;
    }
}

/// Represents spin directions that could be initiated by the end-user.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SpinDirection {
    /// Represents a spin initiated by the end-user in order to increase a
    /// value.
    #[default]
    Increase = 0,

    /// Represents a spin initiated by the end-user in order to decrease a
    /// value.
    Decrease = 1,
}

/// Provides data for the spinner's `Spin` event.
#[derive(Clone)]
pub struct SpinEventArgs {
    base: RoutedEventArgs,
    direction: SpinDirection,
    using_mouse_wheel: bool,
}

ferro_routed_event_args!(SpinEventArgs: RoutedEventArgs);

impl SpinEventArgs {
    /// Creates args for a spin in the given direction, with no routed event.
    pub fn new(direction: SpinDirection) -> Self {
        Self { base: RoutedEventArgs::new(), direction, using_mouse_wheel: false }
    }

    /// Creates args for a routed event and a spin in the given direction.
    pub fn with_event<T: ?Sized>(routed_event: Option<&RoutedEvent<T>>, direction: SpinDirection) -> Self {
        Self::with_event_and_mouse_wheel(routed_event, direction, false)
    }

    /// Creates args for a spin in the given direction, with no routed
    /// event, stating whether the spin was initiated by a mouse wheel.
    pub fn with_mouse_wheel(direction: SpinDirection, using_mouse_wheel: bool) -> Self {
        Self { base: RoutedEventArgs::new(), direction, using_mouse_wheel }
    }

    /// Creates args for a routed event and a spin in the given direction,
    /// stating whether the spin was initiated by a mouse wheel.
    pub fn with_event_and_mouse_wheel<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        direction: SpinDirection,
        using_mouse_wheel: bool,
    ) -> Self {
        let base = RoutedEventArgs::new();
        base.set_routed_event(routed_event);
        Self { base, direction, using_mouse_wheel }
    }

    /// The direction for the spin.
    #[inline]
    pub fn direction(&self) -> SpinDirection {
        self.direction
    }

    /// Whether the spin event originated from a mouse wheel event.
    #[inline]
    pub fn using_mouse_wheel(&self) -> bool {
        self.using_mouse_wheel
    }
}

/// Base class for controls that represents controls that can spin.
///
/// The class is abstract: it has no `new`.
#[repr(C)]
pub struct Spinner {
    base: ContentControl,
}

ferro_class! {
    Spinner: ContentControl, virtuals SpinnerImpl: ContentControlImpl {
        /// Called when the valid spin direction changed.
        fn on_valid_spin_direction_changed(this, old_value: ValidSpinDirections, new_value: ValidSpinDirections);
        /// Raises the `Spin` event when spinning is initiated by the
        /// end-user.
        fn on_spin(this, e: &SpinEventArgs);
    }
}

ferro_impl_classes!(
    Spinner: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for Spinner {}

impl SpinnerImpl for Spinner {
    fn on_valid_spin_direction_changed(_this: &Self, _old_value: ValidSpinDirections, _new_value: ValidSpinDirections) {}

    fn on_spin(this: &Self, e: &SpinEventArgs) {
        let valid = if e.direction() == SpinDirection::Increase {
            ValidSpinDirections::INCREASE
        } else {
            ValidSpinDirections::DECREASE
        };

        // Only raise the event if spin is allowed.
        if this.valid_spin_direction() & valid == valid {
            this.raise_event(e);
        }
    }
}

ferroui_base::ferro_properties! { impl Spinner {
    ferro_property!(
        /// Defines the `ValidSpinDirection` property.
        pub fn valid_spin_direction_property() -> StyledProperty<ValidSpinDirections> {
            FerroProperty::register::<Spinner, _>(
                "ValidSpinDirection",
                ValidSpinDirections::INCREASE | ValidSpinDirections::DECREASE,
            )
        }
    );
} }

impl Spinner {
    ferro_routed_event!(
        /// Defines the `Spin` routed event.
        pub fn spin_event() -> RoutedEvent<SpinEventArgs> {
            RoutedEvent::register::<Spinner, _>("Spin", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        Self::valid_spin_direction_property().changed().add_class_handler::<Spinner>(|spinner, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<ValidSpinDirections>();
            spinner.on_valid_spin_direction_changed(old_value, new_value);
        });
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct() }
    }

    /// Occurs when spinning is initiated by the end-user.
    pub fn spin(&self, handler: impl Fn(&Interactive, &SpinEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::spin_event(), handler)
    }

    /// The valid spin directions.
    pub fn valid_spin_direction(&self) -> ValidSpinDirections {
        self.get_value(Self::valid_spin_direction_property())
    }

    pub fn set_valid_spin_direction(&self, value: ValidSpinDirections) {
        self.set_value(Self::valid_spin_direction_property(), value)
    }
}
