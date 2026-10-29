//! Port of `AutoScrollDuringDragBehavior.cs`.

use crate::routed_handlers::RoutedHandlers;
use ferroui_base::input::{
    InputElement, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Point, Rect, Ref,
    Size, StaticType, StyledElementImpl, StyledProperty, Vector, Visual,
};
use ferroui_controls::ScrollViewer;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::Cell;

/// Automatically scrolls the associated [`ScrollViewer`] when the pointer is dragged near its
/// edges.
#[repr(C)]
pub struct AutoScrollDuringDragBehavior {
    base: StyledElementBehaviorOfT,
    dragging: Cell<bool>,
    handlers: RoutedHandlers,
}

ferro_class!(AutoScrollDuringDragBehavior: StyledElementBehaviorOfT);
ferro_class_info!(AutoScrollDuringDragBehavior { new: AutoScrollDuringDragBehavior::new });
ferroui_base::ferro_impl_classes!(AutoScrollDuringDragBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl AutoScrollDuringDragBehavior {
        /// Identifies the `EdgeDistance` property.
        pub fn edge_distance_property() -> StyledProperty<f64> {
            FerroProperty::register::<AutoScrollDuringDragBehavior, _>("EdgeDistance", 20.0)
        }

        /// Identifies the `ScrollDelta` property.
        pub fn scroll_delta_property() -> StyledProperty<f64> {
            FerroProperty::register::<AutoScrollDuringDragBehavior, _>("ScrollDelta", 10.0)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AutoScrollDuringDragBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            let Some(associated_object) = this.associated_object_as::<ScrollViewer>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_pressed_event(),
                move |_: &Interactive, e: &PointerPressedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pressed(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_released_event(),
                move |_: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.released(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_moved_event(),
                move |_: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.moved(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_capture_lost_event(),
                move |_: &Interactive, e: &PointerCaptureLostEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.capture_lost(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            this.handlers.clear();
        }
    }
}

/// `double.Epsilon` of .NET: the smallest positive value (a subnormal).
const DOUBLE_EPSILON: f64 = f64::from_bits(1);

/// Not from the reference: the offset a scroll viewer whose bounds are `bounds`, whose offset is
/// `offset` and whose extent is `extent` scrolls to when the pointer is at `position`: by
/// `delta` towards an edge closer than `threshold`, within the scrollable range.
pub(crate) fn auto_scroll_offset(
    position: Point,
    bounds: Rect,
    offset: Vector,
    extent: Size,
    delta: f64,
    threshold: f64,
) -> Vector {
    let mut new_x = offset.x;
    let mut new_y = offset.y;

    if position.x < threshold {
        new_x = (new_x - delta).max(0.0);
    } else if position.x > bounds.width - threshold {
        new_x = (new_x + delta).min((extent.width - bounds.width).max(0.0));
    }

    if position.y < threshold {
        new_y = (new_y - delta).max(0.0);
    } else if position.y > bounds.height - threshold {
        new_y = (new_y + delta).min((extent.height - bounds.height).max(0.0));
    }

    Vector::new(new_x, new_y)
}

impl AutoScrollDuringDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<ScrollViewer as StaticType>::TYPE),
            dragging: Cell::new(false),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the distance from the edge that triggers scrolling.
    pub fn edge_distance(&self) -> f64 {
        self.get_value(Self::edge_distance_property())
    }

    pub fn set_edge_distance(&self, value: f64) {
        self.set_value(Self::edge_distance_property(), value)
    }

    /// Gets or sets the amount scrolled when triggered.
    pub fn scroll_delta(&self) -> f64 {
        self.get_value(Self::scroll_delta_property())
    }

    pub fn set_scroll_delta(&self, value: f64) {
        self.set_value(Self::scroll_delta_property(), value)
    }

    fn pressed(&self, e: &PointerPressedEventArgs) {
        if e.properties().is_left_button_pressed && self.is_enabled() {
            self.dragging.set(true);
        }
    }

    fn released(&self, e: &PointerReleasedEventArgs) {
        if e.initial_press_mouse_button() == MouseButton::Left {
            self.dragging.set(false);
        }
    }

    fn capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        self.dragging.set(false);
    }

    fn moved(&self, e: &PointerEventArgs) {
        let associated_object = self.associated_object_as::<ScrollViewer>();
        let Some(associated_object) = associated_object.filter(|_| self.dragging.get() && self.is_enabled()) else {
            return;
        };

        let associated_visual: &Visual = &associated_object;
        let pos = e.get_position(Some(associated_visual));
        let offset = associated_object.offset();
        let new_offset = auto_scroll_offset(
            pos,
            associated_object.bounds(),
            offset,
            associated_object.extent(),
            self.scroll_delta(),
            self.edge_distance(),
        );

        if (new_offset.x - offset.x).abs() > DOUBLE_EPSILON || (new_offset.y - offset.y).abs() > DOUBLE_EPSILON {
            associated_object.set_offset(new_offset);
        }
    }
}
