use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::primitives::{
    RangeBase, ScrollBar, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, Thumb, Track,
};
use crate::{Button, ControlImpl, TickBar, TickList};
use ferroui_base::input::navigation::XYFocusHelpers;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerEventArgs, PointerPoint,
    PointerPressedEventArgs, PointerReleasedEventArgs, VectorEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty, StyledPropertyMetadata, Visual,
    VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_VERTICAL: &str = ":vertical";
const PC_HORIZONTAL: &str = ":horizontal";
const PC_PRESSED: &str = ":pressed";

const TOLERANCE: f64 = 0.0001;

/// Describes how to position the ticks in a [`Slider`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TickPlacement {
    /// No tick marks will appear.
    None = 0,
    /// Tick marks will appear above the track for a horizontal slider, or to
    /// the left of the track for a vertical slider.
    TopLeft = 1,
    /// Tick marks will appear below the track for a horizontal slider, or to
    /// the right of the track for a vertical slider.
    BottomRight = 2,
    /// Tick marks appear on both sides of either a horizontal or vertical
    /// slider.
    Outside = 3,
}

/// A control that lets the user select from a range of values by moving a
/// thumb control along a track.
#[repr(C)]
pub struct Slider {
    base: RangeBase,
    // Slider required parts
    is_dragging: Cell<bool>,
    is_focus_engaged: Cell<bool>,
    track: RefCell<Option<Ref<Track>>>,
    decrease_button: RefCell<Option<Ref<Button>>>,
    increase_button: RefCell<Option<Ref<Button>>>,
    decrease_button_press_dispose: RefCell<Option<Rc<dyn IDisposable>>>,
    decrease_button_release_dispose: RefCell<Option<Rc<dyn IDisposable>>>,
    increase_button_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    increase_button_release_dispose: RefCell<Option<Rc<dyn IDisposable>>>,
    pointer_moved_dispose: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    Slider: RangeBase, virtuals SliderImpl: TemplatedControlImpl {
        /// Called when the user starts dragging the thumb.
        fn on_thumb_drag_started(this, e: &VectorEventArgs);
        /// Called when the user stops dragging the thumb.
        fn on_thumb_drag_completed(this, e: &VectorEventArgs);
    }
}
ferroui_base::ferro_class_info!(Slider { new: Slider::new });

ferro_impl_classes!(Slider: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl ControlImpl for Slider {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::SliderAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for Slider {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.orientation());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::orientation_property().as_property() {
            this.update_pseudo_classes(change.get_new_value::<Orientation>());
        }
    }
}

impl TemplatedControlImpl for Slider {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        for slot in [
            &this.decrease_button_press_dispose,
            &this.decrease_button_release_dispose,
            &this.increase_button_subscription,
            &this.increase_button_release_dispose,
            &this.pointer_moved_dispose,
        ] {
            let previous = slot.borrow_mut().take();
            if let Some(previous) = previous {
                previous.dispose();
            }
        }

        let track = e.name_scope().find_as::<Track>("PART_Track");
        *this.track.borrow_mut() = track.clone();

        let weak = this.to_ref().downgrade();

        if let Some(track) = track {
            track.set_ignore_thumb_drag(true);

            let decrease_button = e.name_scope().find_as::<Button>("PART_DecreaseButton");
            let increase_button = e.name_scope().find_as::<Button>("PART_IncreaseButton");
            *this.decrease_button.borrow_mut() = decrease_button.clone();
            *this.increase_button.borrow_mut() = increase_button.clone();

            let subscribe = |button: &Button| {
                let pressed_weak = weak.clone();
                let press = button.add_disposable_handler_with(
                    InputElement::pointer_pressed_event(),
                    move |_, e: &PointerPressedEventArgs| {
                        if let Some(this) = pressed_weak.upgrade() {
                            this.track_pressed(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                );
                let released_weak = weak.clone();
                let release = button.add_disposable_handler_with(
                    InputElement::pointer_released_event(),
                    move |_, e: &PointerReleasedEventArgs| {
                        if let Some(this) = released_weak.upgrade() {
                            this.track_released(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                );
                (press, release)
            };

            if let Some(decrease_button) = &decrease_button {
                let (press, release) = subscribe(decrease_button);
                *this.decrease_button_press_dispose.borrow_mut() = Some(press);
                *this.decrease_button_release_dispose.borrow_mut() = Some(release);
            }

            if let Some(increase_button) = &increase_button {
                let (press, release) = subscribe(increase_button);
                *this.increase_button_subscription.borrow_mut() = Some(press);
                *this.increase_button_release_dispose.borrow_mut() = Some(release);
            }
        }

        let pointer_moved = this.add_disposable_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e: &PointerEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.track_moved(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        *this.pointer_moved_dispose.borrow_mut() = Some(pointer_moved);
    }
}

impl InputElementImpl for Slider {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if e.handled() || !e.key_modifiers.is_empty() {
            return;
        }

        let using_xy_navigation = XYFocusHelpers::is_allowed_xy_navigation_mode(this, Some(e.key_device_type));
        let allow_arrow_keys = this.is_focus_engaged.get() || !using_xy_navigation;

        let mut handled = true;

        match e.key {
            Key::Enter if using_xy_navigation => {
                this.is_focus_engaged.set(!this.is_focus_engaged.get());
                handled = true;
            }
            Key::Escape if using_xy_navigation => {
                this.is_focus_engaged.set(false);
                handled = true;
            }
            Key::Down | Key::Left if allow_arrow_keys => {
                this.move_to_next_tick(if this.is_direction_reversed() {
                    this.small_change()
                } else {
                    -this.small_change()
                });
            }
            Key::Up | Key::Right if allow_arrow_keys => {
                this.move_to_next_tick(if this.is_direction_reversed() {
                    -this.small_change()
                } else {
                    this.small_change()
                });
            }
            Key::PageUp => {
                this.move_to_next_tick(if this.is_direction_reversed() {
                    -this.large_change()
                } else {
                    this.large_change()
                });
            }
            Key::PageDown => {
                this.move_to_next_tick(if this.is_direction_reversed() {
                    this.large_change()
                } else {
                    -this.large_change()
                });
            }
            Key::Home => {
                this.set_current_value(RangeBase::value_property(), this.minimum());
            }
            Key::End => {
                this.set_current_value(RangeBase::value_property(), this.maximum());
            }
            _ => {
                handled = false;
            }
        }

        e.set_handled(handled);
    }
}

impl SliderImpl for Slider {
    fn on_thumb_drag_started(this: &Self, _e: &VectorEventArgs) {
        this.is_dragging.set(true);
    }

    fn on_thumb_drag_completed(this: &Self, _e: &VectorEventArgs) {
        this.is_dragging.set(false);
    }
}

impl Slider {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_DecreaseButton", <Button as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_IncreaseButton", <Button as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_Track", <Track as StaticType>::TYPE),
    ];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute =
        PseudoClassesAttribute::new(&[PC_VERTICAL, PC_HORIZONTAL, PC_PRESSED]);
}

ferroui_base::ferro_properties! { impl Slider {
    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            ScrollBar::orientation_property().add_owner::<Slider>()
        }
    );

    ferro_property!(
        /// Defines the `IsDirectionReversed` property.
        pub fn is_direction_reversed_property() -> StyledProperty<bool> {
            Track::is_direction_reversed_property().add_owner::<Slider>()
        }
    );

    ferro_property!(
        /// Defines the `IsSnapToTickEnabled` property.
        pub fn is_snap_to_tick_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<Slider, _>("IsSnapToTickEnabled", false)
        }
    );

    ferro_property!(
        /// Defines the `TickFrequency` property.
        pub fn tick_frequency_property() -> StyledProperty<f64> {
            FerroProperty::register::<Slider, _>("TickFrequency", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `TickPlacement` property.
        pub fn tick_placement_property() -> StyledProperty<TickPlacement> {
            FerroProperty::register::<Slider, _>("TickPlacement", TickPlacement::None)
        }
    );

    ferro_property!(
        /// Defines the `Ticks` property.
        pub fn ticks_property() -> StyledProperty<Option<TickList>> {
            TickBar::ticks_property().add_owner::<Slider>()
        }
    );
} }

impl Slider {
    fn static_constructor() {
        crate::mixins::PressedMixin::attach::<Slider>();

        InputElement::focusable_property().override_default_value::<Slider>(true);
        Self::orientation_property().override_default_value::<Slider>(Orientation::Horizontal);
        Thumb::drag_started_event().add_class_handler_with::<Slider>(
            |x, e| x.on_thumb_drag_started(e),
            RoutingStrategies::BUBBLE,
            false,
        );
        Thumb::drag_completed_event().add_class_handler_with::<Slider>(
            |x, e| x.on_thumb_drag_completed(e),
            RoutingStrategies::BUBBLE,
            false,
        );

        RangeBase::value_property()
            .override_metadata::<Slider>(StyledPropertyMetadata::new(None).with_enable_data_validation(true));
        crate::automation::AutomationProperties::control_type_override_property()
            .override_default_value::<Slider>(Some(crate::automation::peers::AutomationControlType::Slider));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: RangeBase::construct(),
            is_dragging: Cell::new(false),
            is_focus_engaged: Cell::new(false),
            track: RefCell::new(None),
            decrease_button: RefCell::new(None),
            increase_button: RefCell::new(None),
            decrease_button_press_dispose: RefCell::new(None),
            decrease_button_release_dispose: RefCell::new(None),
            increase_button_subscription: RefCell::new(None),
            increase_button_release_dispose: RefCell::new(None),
            pointer_moved_dispose: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Defines the ticks to be drawn on the tick bar.
    pub fn ticks(&self) -> Option<TickList> {
        self.get_value(Self::ticks_property())
    }

    pub fn set_ticks(&self, value: Option<TickList>) {
        self.set_value(Self::ticks_property(), value)
    }

    /// The orientation of the slider.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// The direction of increasing value.
    ///
    /// True if the direction of increasing value is to the left for a
    /// horizontal slider or down for a vertical slider; otherwise, false.
    /// The default is false.
    pub fn is_direction_reversed(&self) -> bool {
        self.get_value(Self::is_direction_reversed_property())
    }

    pub fn set_is_direction_reversed(&self, value: bool) {
        self.set_value(Self::is_direction_reversed_property(), value)
    }

    /// Indicates whether the slider automatically moves the thumb to the
    /// closest tick mark.
    pub fn is_snap_to_tick_enabled(&self) -> bool {
        self.get_value(Self::is_snap_to_tick_enabled_property())
    }

    pub fn set_is_snap_to_tick_enabled(&self, value: bool) {
        self.set_value(Self::is_snap_to_tick_enabled_property(), value)
    }

    /// The interval between tick marks.
    pub fn tick_frequency(&self) -> f64 {
        self.get_value(Self::tick_frequency_property())
    }

    pub fn set_tick_frequency(&self, value: f64) {
        self.set_value(Self::tick_frequency_property(), value)
    }

    /// Indicates where to draw tick marks in relation to the track.
    pub fn tick_placement(&self) -> TickPlacement {
        self.get_value(Self::tick_placement_property())
    }

    pub fn set_tick_placement(&self, value: TickPlacement) {
        self.set_value(Self::tick_placement_property(), value)
    }

    /// Whether the slider is currently being dragged.
    pub fn is_dragging(&self) -> bool {
        self.is_dragging.get()
    }

    /// The track part of the slider.
    pub fn track(&self) -> Option<Ref<Track>> {
        self.track.borrow().clone()
    }


    fn move_to_next_tick(&self, direction: f64) {
        if direction == 0.0 {
            return;
        }

        let value = self.value();
        let minimum = self.minimum();
        let maximum = self.maximum();

        // Find the next value by snapping.
        let mut next = self.snap_to_tick(f64::max(minimum, f64::min(maximum, value + direction)));

        // Search for the next tick greater than value?
        let greater_than = direction > 0.0;

        // If the snapping brought us back to value, find the next tick
        // point.
        if (next - value).abs() < TOLERANCE
            // Stop if searching up if already at Max.
            && !(greater_than && (value - maximum).abs() < TOLERANCE)
            // Stop if searching down if already at Min.
            && !(!greater_than && (value - minimum).abs() < TOLERANCE)
        {
            let ticks = self.ticks().filter(|ticks| ticks.count() > 0);
            let tick_frequency = self.tick_frequency();

            // If the ticks collection is available, use it. Note that the
            // ticks may be unsorted.
            if let Some(ticks) = ticks {
                for &tick in ticks.snapshot().iter() {
                    // Find the smallest tick greater than value or the
                    // largest tick less than value.
                    if greater_than
                        && MathUtilities::greater_than(tick, value)
                        && (MathUtilities::less_than(tick, next) || (next - value).abs() < TOLERANCE)
                        || !greater_than
                            && MathUtilities::less_than(tick, value)
                            && (MathUtilities::greater_than(tick, next) || (next - value).abs() < TOLERANCE)
                    {
                        next = tick;
                    }
                }
            } else if MathUtilities::greater_than(tick_frequency, 0.0) {
                // Find the current tick we are at.
                let mut tick_number = ((value - minimum) / tick_frequency).round_ties_even();

                if greater_than {
                    tick_number += 1.0;
                } else {
                    tick_number -= 1.0;
                }

                next = minimum + tick_number * tick_frequency;
            }
        }

        // Update if we've found a better value.
        if (next - value).abs() > TOLERANCE {
            self.set_current_value(RangeBase::value_property(), next);
        }
    }

    fn track_visual(&self) -> Option<Ref<Visual>> {
        self.track.borrow().clone().map(Ref::upcast)
    }

    fn track_moved(&self, e: &PointerEventArgs) {
        if !self.is_enabled() {
            self.is_dragging.set(false);
            return;
        }

        if self.is_dragging.get() {
            self.move_to_point(e.get_current_point(self.track_visual().as_deref()));
        }
    }

    fn track_released(&self, _e: &PointerReleasedEventArgs) {
        self.is_dragging.set(false);
    }

    fn track_pressed(&self, e: &PointerPressedEventArgs) {
        if e.get_current_point(Some(self)).properties.is_left_button_pressed {
            self.move_to_point(e.get_current_point(self.track_visual().as_deref()));
            self.is_dragging.set(true);
        }
    }

    fn move_to_point(&self, pos_on_track: PointerPoint) {
        let Some(track) = self.track.borrow().clone() else {
            return;
        };

        let orient = self.orientation() == Orientation::Horizontal;
        let thumb_bounds = track.thumb().map(|thumb| thumb.bounds());
        // The smallest positive value is added to avoid a division by zero.
        let thumb_length =
            thumb_bounds.map_or(0.0, |bounds| if orient { bounds.width } else { bounds.height }) + 5e-324;
        let track_length = (if orient { track.bounds().width } else { track.bounds().height }) - thumb_length;
        let track_pos = if orient { pos_on_track.position.x } else { pos_on_track.position.y };
        let logical_pos = MathUtilities::clamp((track_pos - thumb_length * 0.5) / track_length, 0.0, 1.0);
        let invert = if orient == self.is_direction_reversed() { 1.0 } else { 0.0 };
        let calc_val = f64::abs(invert - logical_pos);
        let range = self.maximum() - self.minimum();
        let final_value = calc_val * range + self.minimum();

        self.set_current_value(
            RangeBase::value_property(),
            if self.is_snap_to_tick_enabled() { self.snap_to_tick(final_value) } else { final_value },
        );
    }

    /// Snaps the input value to the closest tick.
    fn snap_to_tick(&self, mut value: f64) -> f64 {
        if self.is_snap_to_tick_enabled() {
            let mut previous = self.minimum();
            let mut next = self.maximum();

            let ticks = self.ticks().filter(|ticks| ticks.count() > 0);
            let tick_frequency = self.tick_frequency();

            // If the ticks collection is available, use it. Note that the
            // ticks may be unsorted.
            if let Some(ticks) = ticks {
                for &tick in ticks.snapshot().iter() {
                    if MathUtilities::are_close(tick, value) {
                        return value;
                    }

                    if MathUtilities::less_than(tick, value) && MathUtilities::greater_than(tick, previous) {
                        previous = tick;
                    } else if MathUtilities::greater_than(tick, value) && MathUtilities::less_than(tick, next) {
                        next = tick;
                    }
                }
            } else if MathUtilities::greater_than(tick_frequency, 0.0) {
                previous =
                    self.minimum() + ((value - self.minimum()) / tick_frequency).round_ties_even() * tick_frequency;
                next = f64::min(self.maximum(), previous + tick_frequency);
            }

            // Choose the closest value between previous and next. If tie,
            // snap to 'next'.
            value = if MathUtilities::greater_than_or_close(value, (previous + next) * 0.5) { next } else { previous };
        }

        value
    }

    fn update_pseudo_classes(&self, o: Orientation) {
        self.pseudo_classes().set(PC_VERTICAL, o == Orientation::Vertical);
        self.pseudo_classes().set(PC_HORIZONTAL, o == Orientation::Horizontal);
    }
}
