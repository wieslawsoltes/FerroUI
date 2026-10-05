use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl};
use crate::{
    Button, ContentControlImpl, ControlImpl, SpinDirection, SpinEventArgs, Spinner, SpinnerImpl,
    ValidSpinDirections,
};
use ferroui_base::input::navigation::XYFocusHelpers;
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerReleasedEventArgs, PointerWheelEventArgs,
};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::RefCell;

const PC_LEFT: &str = ":left";
const PC_RIGHT: &str = ":right";

/// Represents the location of the spin buttons of a [`ButtonSpinner`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Location {
    /// The buttons are placed at the left.
    Left,
    /// The buttons are placed at the right.
    Right,
}

/// A template button of the spinner and the click handler attached to it.
type SpinButton = (Ref<Button>, RoutedEventHandlerToken);

/// Represents a spinner control that includes two buttons.
#[repr(C)]
pub struct ButtonSpinner {
    base: Spinner,
    decrease_button: RefCell<Option<SpinButton>>,
    increase_button: RefCell<Option<SpinButton>>,
}

ferro_class! {
    ButtonSpinner: Spinner, virtuals ButtonSpinnerImpl: SpinnerImpl {
        /// Called when the `AllowSpin` property value changed.
        fn on_allow_spin_changed(this, old_value: bool, new_value: bool);
    }
}
ferroui_base::ferro_class_info!(ButtonSpinner { new: ButtonSpinner::new });

ferro_impl_classes!(
    ButtonSpinner: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for ButtonSpinner {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.button_spinner_location());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::button_spinner_location_property().as_property() {
            this.update_pseudo_classes(change.get_new_value::<Location>());
        }
    }
}

impl InputElementImpl for ButtonSpinner {
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if let Some(increase_button) = this.increase_button() {
            if !increase_button.is_enabled() {
                let mouse_position = e.get_position(Some(&increase_button));
                if mouse_position.x > 0.0
                    && mouse_position.x < increase_button.width()
                    && mouse_position.y > 0.0
                    && mouse_position.y < increase_button.height()
                {
                    e.set_handled(true);
                }
            }
        }

        if let Some(decrease_button) = this.decrease_button() {
            if !decrease_button.is_enabled() {
                let mouse_position = e.get_position(Some(&decrease_button));
                if mouse_position.x > 0.0
                    && mouse_position.x < decrease_button.width()
                    && mouse_position.y > 0.0
                    && mouse_position.y < decrease_button.height()
                {
                    e.set_handled(true);
                }
            }
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        // If XY navigation is enabled - do not spin with arrow keys, instead
        // use spinner buttons.
        if XYFocusHelpers::is_allowed_xy_navigation_mode(this, Some(e.key_device_type)) {
            return;
        }

        match e.key {
            Key::Up => {
                if this.allow_spin() {
                    this.on_spin(&SpinEventArgs::with_event(Some(Spinner::spin_event()), SpinDirection::Increase));
                    e.set_handled(true);
                }
            }
            Key::Down => {
                if this.allow_spin() {
                    this.on_spin(&SpinEventArgs::with_event(Some(Spinner::spin_event()), SpinDirection::Decrease));
                    e.set_handled(true);
                }
            }
            Key::Enter => {
                // Do not spin on the Enter key when the spinners have focus.
                if this.increase_button().is_some_and(|button| button.is_focused())
                    || this.decrease_button().is_some_and(|button| button.is_focused())
                {
                    e.set_handled(true);
                }
            }
            _ => {}
        }
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);

        if this.allow_spin() && this.is_keyboard_focus_within() && e.delta().y != 0.0 {
            let direction = if e.delta().y < 0.0 { SpinDirection::Decrease } else { SpinDirection::Increase };
            let spinner_event_args =
                SpinEventArgs::with_event_and_mouse_wheel(Some(Spinner::spin_event()), direction, true);
            this.on_spin(&spinner_event_args);
            e.set_handled(true);
        }
    }
}

impl TemplatedControlImpl for ButtonSpinner {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.set_increase_button(e.name_scope().find_as::<Button>("PART_IncreaseButton"));
        this.set_decrease_button(e.name_scope().find_as::<Button>("PART_DecreaseButton"));
        this.set_button_usage();
    }
}

impl SpinnerImpl for ButtonSpinner {
    fn on_valid_spin_direction_changed(this: &Self, _old_value: ValidSpinDirections, _new_value: ValidSpinDirections) {
        this.set_button_usage();
    }
}

impl ButtonSpinnerImpl for ButtonSpinner {
    fn on_allow_spin_changed(this: &Self, _old_value: bool, _new_value: bool) {
        this.set_button_usage();
    }
}

impl ButtonSpinner {
    /// The named parts expected in the control template, in addition to
    /// those of the base class.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_DecreaseButton", <Button as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_IncreaseButton", <Button as StaticType>::TYPE),
    ];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_LEFT, PC_RIGHT]);
}

ferroui_base::ferro_properties! { impl ButtonSpinner {
    ferro_property!(
        /// Defines the `AllowSpin` property.
        pub fn allow_spin_property() -> StyledProperty<bool> {
            FerroProperty::register::<ButtonSpinner, _>("AllowSpin", true)
        }
    );

    ferro_property!(
        /// Defines the `ShowButtonSpinner` property.
        pub fn show_button_spinner_property() -> StyledProperty<bool> {
            FerroProperty::register::<ButtonSpinner, _>("ShowButtonSpinner", true)
        }
    );

    ferro_property!(
        /// Defines the `ButtonSpinnerLocation` property.
        pub fn button_spinner_location_property() -> StyledProperty<Location> {
            FerroProperty::register::<ButtonSpinner, _>("ButtonSpinnerLocation", Location::Right)
        }
    );
} }

impl ButtonSpinner {
    fn static_constructor() {
        Self::allow_spin_property().changed().add_class_handler::<ButtonSpinner>(|spinner, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            spinner.on_allow_spin_changed(old_value, new_value);
        });
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Spinner::construct(), decrease_button: RefCell::new(None), increase_button: RefCell::new(None) }
    }

    /// Creates a button spinner.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn decrease_button(&self) -> Option<Ref<Button>> {
        self.decrease_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn set_decrease_button(&self, value: Option<Ref<Button>>) {
        let value = value.map(|button| self.attach_click(button));
        let old = self.decrease_button.replace(value);
        Self::detach_click(old);
    }

    fn increase_button(&self) -> Option<Ref<Button>> {
        self.increase_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn set_increase_button(&self, value: Option<Ref<Button>>) {
        let value = value.map(|button| self.attach_click(button));
        let old = self.increase_button.replace(value);
        Self::detach_click(old);
    }

    fn attach_click(&self, button: Ref<Button>) -> SpinButton {
        let weak = self.to_ref().downgrade();
        let token = button.click(move |sender, _| {
            if let Some(this) = weak.upgrade() {
                this.on_button_click(sender);
            }
        });
        (button, token)
    }

    fn detach_click(button: Option<SpinButton>) {
        if let Some((button, token)) = button {
            button.remove_handler(Button::click_event(), token);
        }
    }

    /// Whether the spinner should allow to spin.
    pub fn allow_spin(&self) -> bool {
        self.get_value(Self::allow_spin_property())
    }

    pub fn set_allow_spin(&self, value: bool) {
        self.set_value(Self::allow_spin_property(), value)
    }

    /// Whether the spin buttons should be shown.
    pub fn show_button_spinner(&self) -> bool {
        self.get_value(Self::show_button_spinner_property())
    }

    pub fn set_show_button_spinner(&self, value: bool) {
        self.set_value(Self::show_button_spinner_property(), value)
    }

    /// The current location of the spin buttons.
    pub fn button_spinner_location(&self) -> Location {
        self.get_value(Self::button_spinner_location_property())
    }

    pub fn set_button_spinner_location(&self, value: Location) {
        self.set_value(Self::button_spinner_location_property(), value)
    }


    /// Disables or enables the buttons based on the valid spin direction.
    fn set_button_usage(&self) {
        if let Some(increase_button) = self.increase_button() {
            increase_button.set_is_enabled(
                self.allow_spin() && self.valid_spin_direction().contains(ValidSpinDirections::INCREASE),
            );
        }

        if let Some(decrease_button) = self.decrease_button() {
            decrease_button.set_is_enabled(
                self.allow_spin() && self.valid_spin_direction().contains(ValidSpinDirections::DECREASE),
            );
        }
    }

    /// Called when the user clicks one of the spin buttons.
    fn on_button_click(&self, sender: &Interactive) {
        if self.allow_spin() {
            let is_increase = self.increase_button().is_some_and(|button| {
                let button: &Interactive = &button;
                std::ptr::eq(button, sender)
            });
            let direction = if is_increase { SpinDirection::Increase } else { SpinDirection::Decrease };
            self.on_spin(&SpinEventArgs::with_event(Some(Spinner::spin_event()), direction));
        }
    }

    fn update_pseudo_classes(&self, location: Location) {
        self.pseudo_classes().set(PC_LEFT, location == Location::Left);
        self.pseudo_classes().set(PC_RIGHT, location == Location::Right);
    }
}
