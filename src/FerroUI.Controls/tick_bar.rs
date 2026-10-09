use crate::{Control, ControlImpl};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{BrushExtensions, DrawingContext, IBrush, IPen};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use std::ops::Deref;
use std::rc::Rc;

/// Enum which describes how to position the [`TickBar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TickBarPlacement {
    /// Position this tick at the left of target element.
    Left = 0,
    /// Position this tick at the top of target element.
    Top = 1,
    /// Position this tick at the right of target element.
    Right = 2,
    /// Position this tick at the bottom of target element.
    Bottom = 3,
}

/// The value of the `Ticks` property: a shared, observable list of tick
/// values, compared by identity.
#[derive(Clone, Default)]
pub struct TickList(Rc<FerroList<f64>>);

impl TickList {
    /// Creates an empty list.
    pub fn new() -> Self {
        Self(Rc::new(FerroList::new()))
    }

    /// Creates a list holding `ticks`.
    pub fn from_items(ticks: impl IntoIterator<Item = f64>) -> Self {
        let result = Self::new();
        result.add_range(ticks);
        result
    }

    /// The underlying list.
    #[inline]
    pub fn list(&self) -> &Rc<FerroList<f64>> {
        &self.0
    }
}

impl Deref for TickList {
    type Target = FerroList<f64>;

    fn deref(&self) -> &FerroList<f64> {
        &self.0
    }
}

impl PartialEq for TickList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// An element that is used for drawing the ticks of a [`Slider`](crate::Slider).
#[repr(C)]
pub struct TickBar {
    base: Control,
}

ferro_class!(TickBar: Control);
ferroui_base::ferro_class_info!(TickBar { new: TickBar::new });
ferro_impl_classes!(TickBar: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

ferroui_base::ferro_impl_classes!(TickBar: FerroObjectImpl);

impl VisualImpl for TickBar {
    /// Draws the ticks.
    ///
    /// Ticks can be drawn in 8 different ways depending on the `Placement`
    /// and the `IsDirectionReversed` properties.
    ///
    /// The primary ticks (for the `Minimum` and `Maximum` values) have a
    /// height of 100% of the tick bar's render size (its width or height
    /// depending on the `Placement` property).
    ///
    /// The secondary ticks (all other ticks) have a height of 75% of the
    /// tick bar's render size.
    ///
    /// The brush used to fill the ticks is specified by the `Fill` property.
    fn render(this: &Self, dc: &mut DrawingContext) {
        let bounds = this.bounds();
        let mut size = Size::new(bounds.width, bounds.height);
        let minimum = this.minimum();
        let maximum = this.maximum();
        let range = maximum - minimum;
        // Height for the primary ticks (for the minimum and maximum value).
        let tick_len;
        let mut logical_to_physical;
        let mut start_point;
        let mut end_point;
        let reserved_space = this.reserved_space();
        let r_space =
            if this.orientation() == Orientation::Horizontal { reserved_space.width } else { reserved_space.height };

        // Take the thumb size into account.
        let half_reserved_space = r_space * 0.5;

        let placement = this.placement();

        match placement {
            TickBarPlacement::Top => {
                if MathUtilities::greater_than_or_close(r_space, size.width) {
                    return;
                }
                size = Size::new(size.width - r_space, size.height);
                tick_len = -size.height;
                start_point = Point::new(half_reserved_space, size.height);
                end_point = Point::new(half_reserved_space + size.width, size.height);
                logical_to_physical = size.width / range;
            }
            TickBarPlacement::Bottom => {
                if MathUtilities::greater_than_or_close(r_space, size.width) {
                    return;
                }
                size = Size::new(size.width - r_space, size.height);
                tick_len = size.height;
                start_point = Point::new(half_reserved_space, 0.0);
                end_point = Point::new(half_reserved_space + size.width, 0.0);
                logical_to_physical = size.width / range;
            }
            TickBarPlacement::Left => {
                if MathUtilities::greater_than_or_close(r_space, size.height) {
                    return;
                }
                size = Size::new(size.width, size.height - r_space);

                tick_len = -size.width;
                start_point = Point::new(size.width, size.height + half_reserved_space);
                end_point = Point::new(size.width, half_reserved_space);
                logical_to_physical = size.height / range * -1.0;
            }
            TickBarPlacement::Right => {
                if MathUtilities::greater_than_or_close(r_space, size.height) {
                    return;
                }
                size = Size::new(size.width, size.height - r_space);
                tick_len = size.width;
                start_point = Point::new(0.0, size.height + half_reserved_space);
                end_point = Point::new(0.0, half_reserved_space);
                logical_to_physical = size.height / range * -1.0;
            }
        }

        // Height for the secondary ticks.
        let tick_len2 = tick_len * 0.75;

        // Invert the direction of the ticks.
        if this.is_direction_reversed() {
            logical_to_physical *= -1.0;

            // Swap the start point and the end point.
            std::mem::swap(&mut start_point, &mut end_point);
        }

        let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::with_brush(
            this.fill().map(|fill| BrushExtensions::to_immutable(&fill)),
            1.0,
        ));

        // Is it vertical?
        if placement == TickBarPlacement::Left || placement == TickBarPlacement::Right {
            // Reduce the tick interval if it is more than would be visible on
            // the screen.
            let mut interval = this.tick_frequency();
            if interval > 0.0 {
                let min_interval = (maximum - minimum) / size.height;
                if interval < min_interval {
                    interval = min_interval;
                }
            }

            // Draw the minimum and maximum ticks.
            dc.draw_line(&pen, start_point, Point::new(start_point.x + tick_len, start_point.y));
            dc.draw_line(
                &pen,
                Point::new(start_point.x, end_point.y),
                Point::new(start_point.x + tick_len, end_point.y),
            );

            let ticks = this.ticks().filter(|ticks| ticks.count() > 0);

            // Draw the ticks using the specified ticks collection.
            if let Some(ticks) = ticks {
                for &tick in ticks.snapshot().iter() {
                    if MathUtilities::less_than_or_close(tick, minimum)
                        || MathUtilities::greater_than_or_close(tick, maximum)
                    {
                        continue;
                    }

                    let adjusted_tick = tick - minimum;

                    let y = adjusted_tick * logical_to_physical + start_point.y;
                    dc.draw_line(&pen, Point::new(start_point.x, y), Point::new(start_point.x + tick_len2, y));
                }
            }
            // Draw the ticks using the specified tick frequency.
            else if interval > 0.0 {
                let mut i = interval;
                while i < range {
                    let y = i * logical_to_physical + start_point.y;

                    dc.draw_line(&pen, Point::new(start_point.x, y), Point::new(start_point.x + tick_len2, y));
                    i += interval;
                }
            }
        } else {
            // The placement is top or bottom.

            // Reduce the tick interval if it is more than would be visible on
            // the screen.
            let mut interval = this.tick_frequency();
            if interval > 0.0 {
                let min_interval = (maximum - minimum) / size.width;
                if interval < min_interval {
                    interval = min_interval;
                }
            }

            // Draw the minimum and maximum ticks.
            dc.draw_line(&pen, start_point, Point::new(start_point.x, start_point.y + tick_len));
            dc.draw_line(
                &pen,
                Point::new(end_point.x, start_point.y),
                Point::new(end_point.x, start_point.y + tick_len),
            );

            let ticks = this.ticks().filter(|ticks| ticks.count() > 0);

            // Draw the ticks using the specified ticks collection.
            if let Some(ticks) = ticks {
                for &tick in ticks.snapshot().iter() {
                    if MathUtilities::less_than_or_close(tick, minimum)
                        || MathUtilities::greater_than_or_close(tick, maximum)
                    {
                        continue;
                    }
                    let adjusted_tick = tick - minimum;

                    let x = adjusted_tick * logical_to_physical + start_point.x;
                    dc.draw_line(&pen, Point::new(x, start_point.y), Point::new(x, start_point.y + tick_len2));
                }
            }
            // Draw the ticks using the specified tick frequency.
            else if interval > 0.0 {
                let mut i = interval;
                while i < range {
                    let x = i * logical_to_physical + start_point.x;
                    dc.draw_line(&pen, Point::new(x, start_point.y), Point::new(x, start_point.y + tick_len2));
                    i += interval;
                }
            }
        }
    }
}

ferroui_base::ferro_properties! { impl TickBar {
    ferro_property!(
        /// Defines the `Fill` property.
        pub fn fill_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TickBar, _>("Fill", None)
        }
    );

    ferro_property!(
        /// Defines the `Minimum` property.
        pub fn minimum_property() -> StyledProperty<f64> {
            FerroProperty::register::<TickBar, _>("Minimum", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Maximum` property.
        pub fn maximum_property() -> StyledProperty<f64> {
            FerroProperty::register::<TickBar, _>("Maximum", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `TickFrequency` property.
        pub fn tick_frequency_property() -> StyledProperty<f64> {
            FerroProperty::register::<TickBar, _>("TickFrequency", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<TickBar, _>("Orientation", Orientation::Horizontal)
        }
    );

    ferro_property!(
        /// Defines the `Ticks` property.
        pub fn ticks_property() -> StyledProperty<Option<TickList>> {
            FerroProperty::register::<TickBar, _>("Ticks", None)
        }
    );

    ferro_property!(
        /// Defines the `IsDirectionReversed` property.
        pub fn is_direction_reversed_property() -> StyledProperty<bool> {
            FerroProperty::register::<TickBar, _>("IsDirectionReversed", false)
        }
    );

    ferro_property!(
        /// Defines the `Placement` property.
        pub fn placement_property() -> StyledProperty<TickBarPlacement> {
            FerroProperty::register::<TickBar, _>("Placement", TickBarPlacement::Left)
        }
    );

    ferro_property!(
        /// Defines the `ReservedSpace` property.
        pub fn reserved_space_property() -> StyledProperty<Rect> {
            FerroProperty::register::<TickBar, _>("ReservedSpace", Rect::default())
        }
    );
} }

impl TickBar {
    fn static_constructor() {
        Visual::affects_render::<TickBar>(&[
            Self::fill_property().as_property(),
            Self::is_direction_reversed_property().as_property(),
            Self::reserved_space_property().as_property(),
            Self::maximum_property().as_property(),
            Self::minimum_property().as_property(),
            Self::orientation_property().as_property(),
            Self::placement_property().as_property(),
            Self::tick_frequency_property().as_property(),
            Self::ticks_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The brush used to fill the ticks of the tick bar.
    pub fn fill(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::fill_property())
    }

    pub fn set_fill(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::fill_property(), value)
    }

    /// The logical position where the minimum tick will be drawn.
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }

    pub fn set_minimum(&self, value: f64) {
        self.set_value(Self::minimum_property(), value)
    }

    /// The logical position where the maximum tick will be drawn.
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }

    pub fn set_maximum(&self, value: f64) {
        self.set_value(Self::maximum_property(), value)
    }

    /// The tick frequency defines how the ticks will be drawn.
    pub fn tick_frequency(&self) -> f64 {
        self.get_value(Self::tick_frequency_property())
    }

    pub fn set_tick_frequency(&self, value: f64) {
        self.set_value(Self::tick_frequency_property(), value)
    }

    /// The orientation of the tick bar's parent.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// The collection of values which are the logical positions used to
    /// draw the ticks.
    pub fn ticks(&self) -> Option<TickList> {
        self.get_value(Self::ticks_property())
    }

    pub fn set_ticks(&self, value: Option<TickList>) {
        self.set_value(Self::ticks_property(), value)
    }

    /// Defines the direction of value incrementation.
    ///
    /// By default, if the orientation is horizontal, ticks will be drawn
    /// from left to right (and from bottom to top for the vertical
    /// orientation). If this is true the ticks are drawn in the opposite
    /// direction.
    pub fn is_direction_reversed(&self) -> bool {
        self.get_value(Self::is_direction_reversed_property())
    }

    pub fn set_is_direction_reversed(&self, value: bool) {
        self.set_value(Self::is_direction_reversed_property(), value)
    }

    /// Specifies how the ticks will be placed. This property affects the
    /// way ticks are drawn.
    pub fn placement(&self) -> TickBarPlacement {
        self.get_value(Self::placement_property())
    }

    pub fn set_placement(&self, value: TickBarPlacement) {
        self.set_value(Self::placement_property(), value)
    }

    /// The tick bar uses the reserved space for left and right spacing (for
    /// the horizontal orientation) or top and bottom spacing (for the
    /// vertical orientation). The space on both sides of the tick bar is
    /// half of the specified reserved space.
    pub fn reserved_space(&self) -> Rect {
        self.get_value(Self::reserved_space_property())
    }

    pub fn set_reserved_space(&self, value: Rect) {
        self.set_value(Self::reserved_space_property(), value)
    }
}
