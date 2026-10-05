// Portions of this source file are adapted from the Windows Presentation Foundation project.
// (https://github.com/dotnet/wpf/)

use super::{RangeBase, ScrollBar, Thumb};
use crate::metadata::PseudoClassesAttribute;
use crate::{Button, Control, ControlImpl};
use ferroui_base::input::{InputElementImpl, VectorEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{Layoutable, LayoutableImpl, Orientation};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Nullable, Point, Rect, Ref, Size, StyledElement, StyledElementImpl, StyledProperty,
    Vector, VisualImpl,
};
use std::cell::Cell;

const PC_VERTICAL: &str = ":vertical";
const PC_HORIZONTAL: &str = ":horizontal";

/// A control that positions a [`Thumb`] and the buttons before and after it
/// along a range: the building block of scroll bars and sliders.
#[repr(C)]
pub struct Track {
    base: Control,
    deferred_thumb_drag: Cell<Option<Vector>>,
    last_drag: Cell<Vector>,
    thumb_center_offset: Cell<f64>,
    density: Cell<f64>,
    thumb_drag_delta_token: Cell<Option<RoutedEventHandlerToken>>,
    thumb_drag_completed_token: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class! {
    Track: Control, virtuals TrackImpl: ControlImpl {
        /// Calculates the distance along the thumb of a specified point
        /// along the track.
        ///
        /// Returns the distance between the thumb and the specified point
        /// value.
        fn value_from_point(this, point: Point) -> f64;
        /// Calculates the change in the value of the track when the thumb
        /// moves by the given horizontal and vertical displacement.
        fn value_from_distance(this, horizontal: f64, vertical: f64) -> f64;
    }
}
ferroui_base::ferro_class_info!(Track { new: Track::new });

ferro_impl_classes!(Track: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for Track {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.orientation());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::orientation_property().as_property() {
            this.update_pseudo_classes(change.get_new_value::<Orientation>());
        } else if change.property() == Self::defer_thumb_drag_property().as_property()
            && !change.get_new_value::<bool>()
        {
            this.apply_deferred_thumb_drag();
        }
    }
}

impl LayoutableImpl for Track {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut desired_size = Size::new(0.0, 0.0);

        // Only measure the thumb. The repeat buttons will be sized based on
        // the thumb.
        if let Some(thumb) = this.thumb() {
            thumb.measure(available_size);
            desired_size = thumb.desired_size();
        }

        if !this.viewport_size().is_nan() {
            // A scroll bar can shrink to 0 in the direction of scrolling.
            if this.orientation() == Orientation::Vertical {
                desired_size = desired_size.with_height(0.0);
            } else {
                desired_size = desired_size.with_width(0.0);
            }
        }

        desired_size
    }

    fn arrange_override(this: &Self, arrange_size: Size) -> Size {
        let is_vertical = this.orientation() == Orientation::Vertical;
        let viewport_size = f64::max(0.0, this.viewport_size());

        // If the viewport is NaN, compute the thumb's size based on its
        // desired size, otherwise compute the thumb based on the viewport
        // and extent properties.
        let (mut decrease_button_length, mut thumb_length, mut increase_button_length) =
            if this.viewport_size().is_nan() {
                this.compute_slider_lengths(arrange_size, is_vertical)
            } else {
                // Don't arrange if there's not enough content or the track
                // is too small.
                match this.compute_scroll_bar_lengths(arrange_size, viewport_size, is_vertical) {
                    Some(lengths) => lengths,
                    None => return arrange_size,
                }
            };

        // Layout the pieces of the track.
        let mut offset = Point::default();
        let mut piece_size = arrange_size;
        let is_direction_reversed = this.is_direction_reversed();

        if is_vertical {
            Self::coerce_length(&mut decrease_button_length, arrange_size.height);
            Self::coerce_length(&mut increase_button_length, arrange_size.height);
            Self::coerce_length(&mut thumb_length, arrange_size.height);

            offset = offset.with_y(if is_direction_reversed { decrease_button_length + thumb_length } else { 0.0 });
            piece_size = piece_size.with_height(increase_button_length);

            if let Some(increase_button) = this.increase_button() {
                increase_button.arrange(Rect::from_position_size(offset, piece_size));
            }

            offset = offset.with_y(if is_direction_reversed { 0.0 } else { increase_button_length + thumb_length });
            piece_size = piece_size.with_height(decrease_button_length);

            if let Some(decrease_button) = this.decrease_button() {
                decrease_button.arrange(Rect::from_position_size(offset, piece_size));
            }

            offset = offset.with_y(if is_direction_reversed { decrease_button_length } else { increase_button_length });
            piece_size = piece_size.with_height(thumb_length);

            if let Some(thumb) = this.thumb() {
                let bounds = Rect::from_position_size(offset, piece_size);
                let adjust = this.calculate_thumb_adjustment(&thumb, bounds);
                thumb.arrange(bounds);
                thumb.adjust_drag(adjust);
            }

            this.thumb_center_offset.set(offset.y + (thumb_length * 0.5));
        } else {
            Self::coerce_length(&mut decrease_button_length, arrange_size.width);
            Self::coerce_length(&mut increase_button_length, arrange_size.width);
            Self::coerce_length(&mut thumb_length, arrange_size.width);

            offset = offset.with_x(if is_direction_reversed { increase_button_length + thumb_length } else { 0.0 });
            piece_size = piece_size.with_width(decrease_button_length);

            if let Some(decrease_button) = this.decrease_button() {
                decrease_button.arrange(Rect::from_position_size(offset, piece_size));
            }

            offset = offset.with_x(if is_direction_reversed { 0.0 } else { decrease_button_length + thumb_length });
            piece_size = piece_size.with_width(increase_button_length);

            if let Some(increase_button) = this.increase_button() {
                increase_button.arrange(Rect::from_position_size(offset, piece_size));
            }

            offset = offset.with_x(if is_direction_reversed { increase_button_length } else { decrease_button_length });
            piece_size = piece_size.with_width(thumb_length);

            if let Some(thumb) = this.thumb() {
                let bounds = Rect::from_position_size(offset, piece_size);
                let adjust = this.calculate_thumb_adjustment(&thumb, bounds);
                thumb.arrange(bounds);
                thumb.adjust_drag(adjust);
            }

            this.thumb_center_offset.set(offset.x + (thumb_length * 0.5));
        }

        this.last_drag.set(Vector::default());
        arrange_size
    }
}

impl TrackImpl for Track {
    fn value_from_point(this: &Self, point: Point) -> f64 {
        // Find the distance from the center of the thumb to the given point.
        let val = if this.orientation() == Orientation::Horizontal {
            this.thumb_value()
                + this.value_from_distance(
                    point.x - this.thumb_center_offset.get(),
                    point.y - (this.bounds().height * 0.5),
                )
        } else {
            this.thumb_value()
                + this.value_from_distance(
                    point.x - (this.bounds().width * 0.5),
                    point.y - this.thumb_center_offset.get(),
                )
        };

        f64::max(this.minimum(), f64::min(this.maximum(), val))
    }

    fn value_from_distance(this: &Self, horizontal: f64, vertical: f64) -> f64 {
        let scale = if this.is_direction_reversed() { -1.0 } else { 1.0 };

        if this.orientation() == Orientation::Horizontal {
            scale * horizontal * this.density.get()
        } else {
            // Increases in y cause decreases in the slider's value.
            -1.0 * scale * vertical * this.density.get()
        }
    }
}

impl Track {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_VERTICAL, PC_HORIZONTAL]);
}

ferroui_base::ferro_properties! { impl Track {
    ferro_property!(
        /// Defines the `Minimum` property.
        pub fn minimum_property() -> StyledProperty<f64> {
            RangeBase::minimum_property().add_owner::<Track>()
        }
    );

    ferro_property!(
        /// Defines the `Maximum` property.
        pub fn maximum_property() -> StyledProperty<f64> {
            RangeBase::maximum_property().add_owner::<Track>()
        }
    );

    ferro_property!(
        /// Defines the `Value` property.
        pub fn value_property() -> StyledProperty<f64> {
            RangeBase::value_property().add_owner::<Track>()
        }
    );

    ferro_property!(
        /// Defines the `ViewportSize` property.
        pub fn viewport_size_property() -> StyledProperty<f64> {
            ScrollBar::viewport_size_property().add_owner::<Track>()
        }
    );

    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            ScrollBar::orientation_property().add_owner::<Track>()
        }
    );

    ferro_property!(
        /// Defines the `Thumb` property.
        pub fn thumb_property() -> StyledProperty<Option<Ref<Thumb>>> {
            FerroProperty::register::<Track, _>("Thumb", None)
        }
    );

    ferro_property!(
        /// Defines the `IncreaseButton` property.
        pub fn increase_button_property() -> StyledProperty<Option<Ref<Button>>> {
            FerroProperty::register::<Track, _>("IncreaseButton", None)
        }
    );

    ferro_property!(
        /// Defines the `DecreaseButton` property.
        pub fn decrease_button_property() -> StyledProperty<Option<Ref<Button>>> {
            FerroProperty::register::<Track, _>("DecreaseButton", None)
        }
    );

    ferro_property!(
        /// Defines the `IsDirectionReversed` property.
        pub fn is_direction_reversed_property() -> StyledProperty<bool> {
            FerroProperty::register::<Track, _>("IsDirectionReversed", false)
        }
    );

    ferro_property!(
        /// Defines the `IgnoreThumbDrag` property.
        pub fn ignore_thumb_drag_property() -> StyledProperty<bool> {
            FerroProperty::register::<Track, _>("IgnoreThumbDrag", false)
        }
    );

    ferro_property!(
        /// Defines the `DeferThumbDrag` property.
        pub fn defer_thumb_drag_property() -> StyledProperty<bool> {
            FerroProperty::register::<Track, _>("DeferThumbDrag", false)
        }
    );
} }

impl Track {
    fn static_constructor() {
        Self::thumb_property().changed().add_class_handler::<Track>(|x, e| x.thumb_changed(e));
        Self::increase_button_property().changed().add_class_handler::<Track>(|x, e| x.button_changed(e));
        Self::decrease_button_property().changed().add_class_handler::<Track>(|x, e| x.button_changed(e));
        Layoutable::affects_arrange::<Track>(&[
            Self::is_direction_reversed_property().as_property(),
            Self::minimum_property().as_property(),
            Self::maximum_property().as_property(),
            Self::value_property().as_property(),
            Self::orientation_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            deferred_thumb_drag: Cell::new(None),
            last_drag: Cell::new(Vector::default()),
            thumb_center_offset: Cell::new(0.0),
            density: Cell::new(0.0),
            thumb_drag_delta_token: Cell::new(None),
            thumb_drag_completed_token: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The minimum possible value.
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }

    pub fn set_minimum(&self, value: f64) {
        self.set_value(Self::minimum_property(), value)
    }

    /// The maximum possible value.
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }

    pub fn set_maximum(&self, value: f64) {
        self.set_value(Self::maximum_property(), value)
    }

    /// The current value.
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }

    pub fn set_range_value(&self, value: f64) {
        self.set_value(Self::value_property(), value)
    }

    /// The value of the thumb's current position. This can differ from the
    /// value while a thumb drag is deferred.
    fn thumb_value(&self) -> f64 {
        self.value()
            + match self.deferred_thumb_drag.get() {
                None => 0.0,
                Some(vector) => self.value_from_distance(vector.x, vector.y),
            }
    }

    /// The amount of the scrollable content that is currently visible.
    pub fn viewport_size(&self) -> f64 {
        self.get_value(Self::viewport_size_property())
    }

    pub fn set_viewport_size(&self, value: f64) {
        self.set_value(Self::viewport_size_property(), value)
    }

    /// The orientation of the track.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// The thumb of the track: its content.
    pub fn thumb(&self) -> Option<Ref<Thumb>> {
        self.get_value(Self::thumb_property())
    }

    pub fn set_thumb(&self, value: impl Into<Nullable<Thumb>>) {
        self.set_value(Self::thumb_property(), value.into().0)
    }

    /// The button that increases the value.
    pub fn increase_button(&self) -> Option<Ref<Button>> {
        self.get_value(Self::increase_button_property())
    }

    pub fn set_increase_button(&self, value: impl Into<Nullable<Button>>) {
        self.set_value(Self::increase_button_property(), value.into().0)
    }

    /// The button that decreases the value.
    pub fn decrease_button(&self) -> Option<Ref<Button>> {
        self.get_value(Self::decrease_button_property())
    }

    pub fn set_decrease_button(&self, value: impl Into<Nullable<Button>>) {
        self.set_value(Self::decrease_button_property(), value.into().0)
    }

    /// Whether the direction of increasing value is reversed.
    pub fn is_direction_reversed(&self) -> bool {
        self.get_value(Self::is_direction_reversed_property())
    }

    pub fn set_is_direction_reversed(&self, value: bool) {
        self.set_value(Self::is_direction_reversed_property(), value)
    }

    /// Whether drags of the thumb are ignored.
    pub fn ignore_thumb_drag(&self) -> bool {
        self.get_value(Self::ignore_thumb_drag_property())
    }

    pub fn set_ignore_thumb_drag(&self, value: bool) {
        self.set_value(Self::ignore_thumb_drag_property(), value)
    }

    /// Whether the value is only updated when a drag of the thumb completes.
    pub fn defer_thumb_drag(&self) -> bool {
        self.get_value(Self::defer_thumb_drag_property())
    }

    pub fn set_defer_thumb_drag(&self, value: bool) {
        self.set_value(Self::defer_thumb_drag_property(), value)
    }

    fn calculate_thumb_adjustment(&self, thumb: &Thumb, new_thumb_bounds: Rect) -> Vector {
        let thumb_delta: Vector = (new_thumb_bounds.position() - thumb.bounds().position()).into();
        self.last_drag.get() - thumb_delta
    }

    fn coerce_length(component_length: &mut f64, track_length: f64) {
        if *component_length < 0.0 {
            *component_length = 0.0;
        } else if *component_length > track_length || component_length.is_nan() {
            *component_length = track_length;
        }
    }

    /// Returns the lengths of the decrease button, the thumb and the
    /// increase button.
    fn compute_slider_lengths(&self, arrange_size: Size, is_vertical: bool) -> (f64, f64, f64) {
        let min = self.minimum();
        let range = f64::max(0.0, self.maximum() - min);
        let offset = f64::min(range, self.thumb_value() - min);

        let thumb = self.thumb();

        // Compute the thumb size.
        let (track_length, mut thumb_length) = if is_vertical {
            (arrange_size.height, thumb.map_or(0.0, |thumb| thumb.desired_size().height))
        } else {
            (arrange_size.width, thumb.map_or(0.0, |thumb| thumb.desired_size().width))
        };

        Self::coerce_length(&mut thumb_length, track_length);

        let remaining_track_length = track_length - thumb_length;

        let mut decrease_button_length = remaining_track_length * offset / range;
        Self::coerce_length(&mut decrease_button_length, remaining_track_length);

        let mut increase_button_length = remaining_track_length - decrease_button_length;
        Self::coerce_length(&mut increase_button_length, remaining_track_length);

        self.density.set(range / remaining_track_length);

        (decrease_button_length, thumb_length, increase_button_length)
    }

    /// Returns the lengths of the decrease button, the thumb and the
    /// increase button, or `None` when the pieces must not be arranged.
    fn compute_scroll_bar_lengths(
        &self,
        arrange_size: Size,
        viewport_size: f64,
        is_vertical: bool,
    ) -> Option<(f64, f64, f64)> {
        let min = self.minimum();
        let range = f64::max(0.0, self.maximum() - min);
        let offset = f64::min(range, self.thumb_value() - min);
        let extent = f64::max(0.0, range) + viewport_size;
        let track_length = if is_vertical { arrange_size.height } else { arrange_size.width };
        let mut thumb_min_length = 10.0;

        let min_length_property =
            if is_vertical { Layoutable::min_height_property() } else { Layoutable::min_width_property() };

        if let Some(thumb) = self.thumb() {
            if thumb.is_set(min_length_property.as_property()) {
                thumb_min_length = thumb.get_value(min_length_property);
            }
        }

        let mut thumb_length = track_length * viewport_size / extent;
        Self::coerce_length(&mut thumb_length, track_length);
        thumb_length = f64::max(thumb_min_length, thumb_length);

        // If we don't have enough content to scroll, disable the track.
        let not_enough_content_to_scroll = MathUtilities::less_than_or_close(range, 0.0);
        let thumb_longer_than_track = thumb_length > track_length;

        // If there's not enough content or the thumb is longer than the
        // track, hide the track and don't arrange the pieces.
        if not_enough_content_to_scroll || thumb_longer_than_track {
            self.show_children(false);
            self.thumb_center_offset.set(f64::NAN);
            self.density.set(f64::NAN);
            return None; // don't arrange
        }

        self.show_children(true);

        // Compute the lengths of the increase and decrease buttons.
        let remaining_track_length = track_length - thumb_length;
        let mut decrease_button_length = remaining_track_length * offset / range;
        Self::coerce_length(&mut decrease_button_length, remaining_track_length);

        let mut increase_button_length = remaining_track_length - decrease_button_length;
        Self::coerce_length(&mut increase_button_length, remaining_track_length);

        self.density.set(range / remaining_track_length);

        Some((decrease_button_length, thumb_length, increase_button_length))
    }

    fn thumb_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_thumb, new_thumb) = e.get_old_and_new_value::<Option<Ref<Thumb>>>();

        if let Some(old_thumb) = old_thumb {
            if let Some(token) = self.thumb_drag_delta_token.take() {
                old_thumb.remove_handler(Thumb::drag_delta_event(), token);
            }
            if let Some(token) = self.thumb_drag_completed_token.take() {
                old_thumb.remove_handler(Thumb::drag_completed_event(), token);
            }
            StyledElement::logical_children(self).remove(&old_thumb.clone().upcast());
            self.visual_children().remove(&old_thumb.upcast());
        }

        if let Some(new_thumb) = new_thumb {
            let weak = self.to_ref().downgrade();
            self.thumb_drag_delta_token.set(Some(new_thumb.drag_delta(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.thumb_dragged(e);
                }
            })));
            let weak = self.to_ref().downgrade();
            self.thumb_drag_completed_token.set(Some(new_thumb.drag_completed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.apply_deferred_thumb_drag();
                }
            })));
            StyledElement::logical_children(self).add(new_thumb.clone().upcast());
            self.visual_children().add(new_thumb.upcast());
        }
    }

    fn button_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_button, new_button) = e.get_old_and_new_value::<Option<Ref<Button>>>();

        if let Some(old_button) = old_button {
            StyledElement::logical_children(self).remove(&old_button.clone().upcast());
            self.visual_children().remove(&old_button.upcast());
        }

        if let Some(new_button) = new_button {
            StyledElement::logical_children(self).add(new_button.clone().upcast());
            self.visual_children().add(new_button.upcast());
        }
    }

    fn thumb_dragged(&self, e: &VectorEventArgs) {
        if self.ignore_thumb_drag() {
            return;
        }

        if self.defer_thumb_drag() {
            self.deferred_thumb_drag.set(Some(e.vector));
            self.invalidate_arrange();
        } else {
            self.apply_thumb_drag(e.vector);
        }
    }

    fn apply_thumb_drag(&self, vector: Vector) {
        let delta = self.value_from_distance(vector.x, vector.y);
        let factor = vector / delta;
        let old_value = self.value();

        self.set_current_value(
            Self::value_property(),
            MathUtilities::clamp(self.value() + delta, self.minimum(), self.maximum()),
        );

        // Record the part of the drag that actually had effect as the last
        // drag delta. Due to clamping, we need to compare the two values
        // instead of using the drag delta.
        self.last_drag.set(factor * (self.value() - old_value));
    }

    fn apply_deferred_thumb_drag(&self) {
        if let Some(vector) = self.deferred_thumb_drag.take() {
            self.apply_thumb_drag(vector);
        }
    }

    fn show_children(&self, visible: bool) {
        // Setting IsVisible = false on the track would cause it to stop
        // being laid out. Instead show/hide the child controls.
        if let Some(thumb) = self.thumb() {
            thumb.set_is_visible(visible);
        }

        if let Some(increase_button) = self.increase_button() {
            increase_button.set_is_visible(visible);
        }

        if let Some(decrease_button) = self.decrease_button() {
            decrease_button.set_is_visible(visible);
        }
    }

    fn update_pseudo_classes(&self, o: Orientation) {
        self.pseudo_classes().set(PC_VERTICAL, o == Orientation::Vertical);
        self.pseudo_classes().set(PC_HORIZONTAL, o == Orientation::Horizontal);
    }
}
