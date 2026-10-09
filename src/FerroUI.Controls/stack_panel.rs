// This source file is adapted from the Windows Presentation Foundation
// project (https://github.com/dotnet/wpf/), MIT License, courtesy of The .NET
// Foundation.

use crate::primitives::{IScrollSnapPointsInfo, SnapPointsAlignment, SnapPointsChangedHandler};
use crate::{Control, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::{INavigableContainer, InputElement, InputElementImpl, NavigationDirection};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{Layoutable, LayoutableImpl, Orientation};
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// A panel which lays out its children horizontally or vertically.
#[repr(C)]
pub struct StackPanel {
    base: Panel,
}

ferro_class! {
    StackPanel: Panel, virtuals StackPanelImpl: PanelImpl {
        /// Arranges a single child in the slot computed by the panel.
        fn arrange_child(this, child: &Ref<Control>, rect: Rect, panel_size: Size, orientation: Orientation);
        /// Gets the next control in the specified direction.
        ///
        /// `from` is the control from which movement begins.
        fn get_control_in_direction(this, direction: NavigationDirection, from: Option<&Ref<Control>>) -> Option<Ref<InputElement>>;
    }
}
ferroui_base::ferro_class_info!(StackPanel { new: StackPanel::new });

ferro_impl_classes!(
    StackPanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferroui_base::ferro_impl_classes!(StackPanel: FerroObjectImpl);

impl LayoutableImpl for StackPanel {
    /// General stack panel layout behaviour is to grow unbounded in the
    /// "stacking" direction (size to content). Children in this dimension are
    /// encouraged to be as large as they like. In the other dimension, the
    /// stack panel will assume the maximum size of its children.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut stack_desired_size = Size::default();
        let children = this.children().snapshot();
        let mut layout_slot_size = available_size;
        let horizontal = this.orientation() == Orientation::Horizontal;
        let spacing = this.spacing();
        let mut has_visible_child = false;

        // Initialize child sizing and iterator data.
        // Allow children as much size as they want along the stack.
        if horizontal {
            layout_slot_size = layout_slot_size.with_width(f64::INFINITY);
        } else {
            layout_slot_size = layout_slot_size.with_height(f64::INFINITY);
        }

        // Iterate through children.
        for child in children.iter() {
            // Measure the child.
            child.measure(layout_slot_size);

            let is_visible = child.is_visible();

            if is_visible && !has_visible_child {
                has_visible_child = true;
            }

            let child_desired_size = child.desired_size();

            // Accumulate child size.
            if horizontal {
                stack_desired_size = stack_desired_size.with_width(
                    stack_desired_size.width + (if is_visible { spacing } else { 0.0 }) + child_desired_size.width,
                );
                stack_desired_size =
                    stack_desired_size.with_height(max(stack_desired_size.height, child_desired_size.height));
            } else {
                stack_desired_size =
                    stack_desired_size.with_width(max(stack_desired_size.width, child_desired_size.width));
                stack_desired_size = stack_desired_size.with_height(
                    stack_desired_size.height + (if is_visible { spacing } else { 0.0 }) + child_desired_size.height,
                );
            }
        }

        if horizontal {
            stack_desired_size = stack_desired_size
                .with_width(stack_desired_size.width - (if has_visible_child { spacing } else { 0.0 }));
        } else {
            stack_desired_size = stack_desired_size
                .with_height(stack_desired_size.height - (if has_visible_child { spacing } else { 0.0 }));
        }

        stack_desired_size
    }

    /// Content arrangement.
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        let horizontal = this.orientation() == Orientation::Horizontal;
        let mut rc_child = Rect::from_size(final_size);
        let mut previous_child_size = 0.0;
        let spacing = this.spacing();

        // Arrange and position children.
        for child in children.iter() {
            if !child.is_visible() {
                continue;
            }

            if horizontal {
                rc_child = rc_child.with_x(rc_child.x + previous_child_size);
                previous_child_size = child.desired_size().width;
                rc_child = rc_child.with_width(previous_child_size);
                rc_child = rc_child.with_height(max(final_size.height, child.desired_size().height));
                previous_child_size += spacing;
            } else {
                rc_child = rc_child.with_y(rc_child.y + previous_child_size);
                previous_child_size = child.desired_size().height;
                rc_child = rc_child.with_height(previous_child_size);
                rc_child = rc_child.with_width(max(final_size.width, child.desired_size().width));
                previous_child_size += spacing;
            }

            this.arrange_child(child, rc_child, final_size, this.orientation());
        }

        this.raise_snap_points_changed();

        final_size
    }
}

impl StackPanelImpl for StackPanel {
    fn arrange_child(_this: &Self, child: &Ref<Control>, rect: Rect, _panel_size: Size, _orientation: Orientation) {
        child.arrange(rect);
    }

    fn get_control_in_direction(
        this: &Self,
        direction: NavigationDirection,
        from: Option<&Ref<Control>>,
    ) -> Option<Ref<InputElement>> {
        let horiz = this.orientation() == Orientation::Horizontal;
        let children = this.children();
        let mut index: i64 = from.and_then(|from| children.index_of(from)).map_or(-1, |index| index as i64);

        match direction {
            NavigationDirection::First => index = 0,
            NavigationDirection::Last => index = children.count() as i64 - 1,
            NavigationDirection::Next => index += 1,
            NavigationDirection::Previous => {
                if index != -1 {
                    index -= 1;
                }
            }
            NavigationDirection::Left => {
                if index != -1 {
                    index = if horiz { index - 1 } else { -1 };
                }
            }
            NavigationDirection::Right => {
                if index != -1 {
                    index = if horiz { index + 1 } else { -1 };
                }
            }
            NavigationDirection::Up => {
                if index != -1 {
                    index = if horiz { -1 } else { index - 1 };
                }
            }
            NavigationDirection::Down => {
                if index != -1 {
                    index = if horiz { -1 } else { index + 1 };
                }
            }
            _ => index = -1,
        }

        if index >= 0 && (index as usize) < children.count() {
            Some(children.get(index as usize).upcast())
        } else {
            None
        }
    }
}

impl INavigableContainer for StackPanel {
    fn get_control(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<InputElement>>,
        wrap: bool,
    ) -> Option<Ref<InputElement>> {
        let from = from.and_then(|from| from.cast::<Control>());
        let mut result = self.get_control_in_direction(direction, from.as_ref());

        if result.is_none() && wrap {
            use NavigationDirection::*;
            if self.orientation() == Orientation::Vertical {
                match direction {
                    Up | Previous | PageUp => result = self.get_control_in_direction(Last, None),
                    Down | Next | PageDown => result = self.get_control_in_direction(First, None),
                    _ => {}
                }
            } else {
                match direction {
                    Left | Previous | PageUp => result = self.get_control_in_direction(Last, None),
                    Right | Next | PageDown => result = self.get_control_in_direction(First, None),
                    _ => {}
                }
            }
        }

        result
    }
}

ferroui_base::ferro_properties! { impl StackPanel {
    ferro_property!(
        /// Defines the `Spacing` property.
        pub fn spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<StackPanel, _>("Spacing", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<StackPanel, _>("Orientation", Orientation::Vertical)
        }
    );

    ferro_property!(
        /// Defines the `AreHorizontalSnapPointsRegular` property.
        pub fn are_horizontal_snap_points_regular_property() -> StyledProperty<bool> {
            FerroProperty::register::<StackPanel, _>("AreHorizontalSnapPointsRegular", false)
        }
    );

    ferro_property!(
        /// Defines the `AreVerticalSnapPointsRegular` property.
        pub fn are_vertical_snap_points_regular_property() -> StyledProperty<bool> {
            FerroProperty::register::<StackPanel, _>("AreVerticalSnapPointsRegular", false)
        }
    );
} }

impl StackPanel {
    ferro_routed_event!(
        /// Defines the `HorizontalSnapPointsChanged` event.
        pub fn horizontal_snap_points_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<StackPanel, _>("HorizontalSnapPointsChanged", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `VerticalSnapPointsChanged` event.
        pub fn vertical_snap_points_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<StackPanel, _>("VerticalSnapPointsChanged", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        Layoutable::affects_measure::<StackPanel>(&[Self::spacing_property().as_property()]);
        Layoutable::affects_measure::<StackPanel>(&[Self::orientation_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The size of the spacing to place between child controls.
    pub fn spacing(&self) -> f64 {
        self.get_value(Self::spacing_property())
    }

    pub fn set_spacing(&self, value: f64) {
        self.set_value(Self::spacing_property(), value)
    }

    /// The orientation in which child controls will be layed out.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// Occurs when the measurements for horizontal snap points change.
    pub fn horizontal_snap_points_changed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::horizontal_snap_points_changed_event(), handler)
    }

    /// Occurs when the measurements for vertical snap points change.
    pub fn vertical_snap_points_changed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::vertical_snap_points_changed_event(), handler)
    }

    /// Whether the horizontal snap points for the panel are equidistant from
    /// each other.
    pub fn are_horizontal_snap_points_regular(&self) -> bool {
        self.get_value(Self::are_horizontal_snap_points_regular_property())
    }

    pub fn set_are_horizontal_snap_points_regular(&self, value: bool) {
        self.set_value(Self::are_horizontal_snap_points_regular_property(), value)
    }

    /// Whether the vertical snap points for the panel are equidistant from
    /// each other.
    pub fn are_vertical_snap_points_regular(&self) -> bool {
        self.get_value(Self::are_vertical_snap_points_regular_property())
    }

    pub fn set_are_vertical_snap_points_regular(&self, value: bool) {
        self.set_value(Self::are_vertical_snap_points_regular_property(), value)
    }

    /// Raises the snap points changed event of the panel's orientation.
    fn raise_snap_points_changed(&self) {
        let event = if self.orientation() == Orientation::Horizontal {
            Self::horizontal_snap_points_changed_event()
        } else {
            Self::vertical_snap_points_changed_event()
        };
        self.raise_event(&RoutedEventArgs::with_event(event));
    }

    /// Returns the set of distances between irregular snap points for a
    /// specified orientation and alignment.
    ///
    /// Panics if the snap points of the orientation are regular.
    pub fn get_irregular_snap_points(
        &self,
        orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> Vec<f64> {
        let mut snap_points = Vec::new();

        match orientation {
            Orientation::Horizontal => {
                if self.are_horizontal_snap_points_regular() {
                    panic!("The horizontal snap points are regular.");
                }
                if self.orientation() == Orientation::Horizontal {
                    for child in self.visual_children().snapshot().iter() {
                        let bounds = child.bounds();
                        snap_points.push(match snap_points_alignment {
                            SnapPointsAlignment::Near => bounds.left(),
                            SnapPointsAlignment::Center => bounds.center().x,
                            SnapPointsAlignment::Far => bounds.right(),
                        });
                    }
                }
            }
            Orientation::Vertical => {
                if self.are_vertical_snap_points_regular() {
                    panic!("The vertical snap points are regular.");
                }
                if self.orientation() == Orientation::Vertical {
                    for child in self.visual_children().snapshot().iter() {
                        let bounds = child.bounds();
                        snap_points.push(match snap_points_alignment {
                            SnapPointsAlignment::Near => bounds.top(),
                            SnapPointsAlignment::Center => bounds.center().y,
                            SnapPointsAlignment::Far => bounds.bottom(),
                        });
                    }
                }
            }
        }

        snap_points
    }

    /// Gets the distance between regular snap points for a specified
    /// orientation and alignment, and the offset of the first snap point.
    ///
    /// Panics if the snap points of the panel's orientation are not regular.
    pub fn get_regular_snap_points(
        &self,
        _orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> (f64, f64) {
        let mut offset = 0.0;
        let Some(first_child) = self.visual_children().try_get(0) else {
            return (0.0, offset);
        };
        let bounds = first_child.bounds();

        let snap_point = match self.orientation() {
            Orientation::Horizontal => {
                if !self.are_horizontal_snap_points_regular() {
                    panic!("The horizontal snap points are not regular.");
                }

                offset = match snap_points_alignment {
                    SnapPointsAlignment::Near => bounds.left(),
                    SnapPointsAlignment::Center => bounds.center().x,
                    SnapPointsAlignment::Far => bounds.right(),
                };
                bounds.width
            }
            Orientation::Vertical => {
                if !self.are_vertical_snap_points_regular() {
                    panic!("The vertical snap points are not regular.");
                }

                offset = match snap_points_alignment {
                    SnapPointsAlignment::Near => bounds.top(),
                    SnapPointsAlignment::Center => bounds.center().y,
                    SnapPointsAlignment::Far => bounds.bottom(),
                };
                bounds.height
            }
        };

        (snap_point + self.spacing(), offset)
    }
}

impl IScrollSnapPointsInfo for StackPanel {
    fn are_horizontal_snap_points_regular(&self) -> bool {
        StackPanel::are_horizontal_snap_points_regular(self)
    }

    fn set_are_horizontal_snap_points_regular(&self, value: bool) {
        StackPanel::set_are_horizontal_snap_points_regular(self, value)
    }

    fn are_vertical_snap_points_regular(&self) -> bool {
        StackPanel::are_vertical_snap_points_regular(self)
    }

    fn set_are_vertical_snap_points_regular(&self, value: bool) {
        StackPanel::set_are_vertical_snap_points_regular(self, value)
    }

    fn get_irregular_snap_points(
        &self,
        orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> Vec<f64> {
        StackPanel::get_irregular_snap_points(self, orientation, snap_points_alignment)
    }

    fn get_regular_snap_points(
        &self,
        orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> (f64, f64) {
        StackPanel::get_regular_snap_points(self, orientation, snap_points_alignment)
    }

    fn horizontal_snap_points_changed(&self, handler: SnapPointsChangedHandler) -> Rc<dyn IDisposable> {
        self.add_disposable_handler(StackPanel::horizontal_snap_points_changed_event(), move |sender, e| {
            handler(sender, e)
        })
    }

    fn vertical_snap_points_changed(&self, handler: SnapPointsChangedHandler) -> Rc<dyn IDisposable> {
        self.add_disposable_handler(StackPanel::vertical_snap_points_changed_event(), move |sender, e| {
            handler(sender, e)
        })
    }
}
