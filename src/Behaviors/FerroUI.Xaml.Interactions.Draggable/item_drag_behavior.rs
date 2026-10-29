//! Port of `ItemDragBehavior.cs`.

use crate::routed_handlers::RoutedHandlers;
use ferroui_base::collections::FerroList;
use ferroui_base::controls::IPseudoClasses;
use ferroui_base::input::{
    InputElement, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::layout::Orientation;
use ferroui_base::media::transformation::TransformOperationsBuilder;
use ferroui_base::media::ITransform;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Point, Rect, Ref, StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{Control, ItemsControl, ItemsSource};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Allows dragging items within an [`ItemsControl`].
///
/// The behavior is attached to the container of an item (a list box item, a tab item, the content
/// presenter of an items control); while the item is dragged along the
/// [`orientation`](Self::orientation) it is translated with the pointer, the containers it passes
/// make room for it, and on release the item moves to the index it was dropped at.
#[repr(C)]
pub struct ItemDragBehavior {
    base: StyledElementBehaviorOfT,
    enable_drag: Cell<bool>,
    drag_started: Cell<bool>,
    start: Cell<Point>,
    dragged_index: Cell<i32>,
    target_index: Cell<i32>,
    items_control: RefCell<Option<Ref<ItemsControl>>>,
    dragged_container: RefCell<Option<Ref<Control>>>,
    captured: Cell<bool>,
    handlers: RoutedHandlers,
}

ferro_class!(ItemDragBehavior: StyledElementBehaviorOfT);
ferro_class_info!(ItemDragBehavior { new: ItemDragBehavior::new });
ferroui_base::ferro_impl_classes!(ItemDragBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ItemDragBehavior {
        /// Identifies the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<ItemDragBehavior, _>("Orientation", Orientation::Horizontal)
        }

        /// Identifies the `HorizontalDragThreshold` property.
        pub fn horizontal_drag_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ItemDragBehavior, _>("HorizontalDragThreshold", 3.0)
        }

        /// Identifies the `VerticalDragThreshold` property.
        pub fn vertical_drag_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ItemDragBehavior, _>("VerticalDragThreshold", 3.0)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ItemDragBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_released_event(),
                move |_: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_released(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_pressed_event(),
                move |_: &Interactive, e: &PointerPressedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_pressed(e);
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
                        behavior.pointer_moved(e);
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
                        behavior.pointer_capture_lost(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.handlers.clear();
        }
    }
}

/// Not from the reference: how a container that is not dragged moves while an item is dragged
/// over it (the branches of the loop of `PointerMoved`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetShift {
    /// The dragged item passed the middle of a container after it: the container moves back by
    /// the size of the dragged item, and the item goes at its index or after.
    Backward,
    /// The dragged item passed the middle of a container before it: the container moves forward
    /// by the size of the dragged item, and the item goes at its index or before.
    Forward,
    /// The container stays where it is.
    None,
}

/// Not from the reference: the start and the size of a rectangle along an orientation.
fn start_and_length(orientation: Orientation, bounds: Rect) -> (f64, f64) {
    match orientation {
        Orientation::Horizontal => (bounds.x, bounds.width),
        Orientation::Vertical => (bounds.y, bounds.height),
    }
}

/// Not from the reference: the hit test of `PointerMoved`, which tells how the container whose
/// bounds are `target_bounds` moves while the dragged container (`dragged_bounds`) is translated by
/// `delta` along `orientation`.
pub(crate) fn target_shift(orientation: Orientation, dragged_bounds: Rect, delta: f64, target_bounds: Rect) -> TargetShift {
    let (dragged_start, dragged_length) = start_and_length(orientation, dragged_bounds);
    let dragged_delta_start = dragged_start + delta;
    let dragged_delta_end = dragged_start + delta + dragged_length;

    let (target_start, target_length) = start_and_length(orientation, target_bounds);
    let target_mid = target_start + target_length / 2.0;

    if target_start > dragged_start && dragged_delta_end >= target_mid {
        TargetShift::Backward
    } else if target_start < dragged_start && dragged_delta_start <= target_mid {
        TargetShift::Forward
    } else {
        TargetShift::None
    }
}

/// Not from the reference: the target index after a container at `target_index` was hit tested
/// (`_targetIndex` of the loop of `PointerMoved`): the last container passed going forward, the
/// first one passed going backward.
pub(crate) fn next_target_index(current: i32, target_index: i32, shift: TargetShift) -> i32 {
    match shift {
        TargetShift::Backward => {
            if current == -1 || target_index > current {
                target_index
            } else {
                current
            }
        }
        TargetShift::Forward => {
            if current == -1 || target_index < current {
                target_index
            } else {
                current
            }
        }
        TargetShift::None => current,
    }
}

/// Not from the reference: whether the pointer moved from `start` to `position` far enough along
/// `orientation` for the drag to start.
pub(crate) fn exceeds_drag_threshold(
    orientation: Orientation,
    start: Point,
    position: Point,
    horizontal_drag_threshold: f64,
    vertical_drag_threshold: f64,
) -> bool {
    let diff = start - position;
    match orientation {
        Orientation::Horizontal => diff.x.abs() > horizontal_drag_threshold,
        Orientation::Vertical => diff.y.abs() > vertical_drag_threshold,
    }
}

/// Not from the reference: moves an item of the items source of an items control (`IList` of the
/// reference: `RemoveAt` and `Insert`).
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference moves the item of any `IList`; an items
/// source of the framework is read only, so the port moves it in the lists whose type it knows: a
/// `FerroList` or an `ObservableCollection` of untyped values, texts, numbers, booleans, characters
/// or objects. Returns whether the source is one of them.
pub(crate) fn move_in_items_source(items_source: &ItemsSource, dragged_index: usize, target_index: usize) -> bool {
    macro_rules! move_in {
        ($($item:ty),* $(,)?) => {$(
            if let Some(list) = items_source.downcast_ref::<FerroList<$item>>() {
                if dragged_index >= list.count() || target_index >= list.count() {
                    return true;
                }
                let dragged_item = list.get(dragged_index);
                list.remove_at(dragged_index);
                list.insert(target_index, dragged_item);
                return true;
            }
            if let Some(list) = items_source.downcast_ref::<ObservableCollection<$item>>() {
                if dragged_index >= list.count() || target_index >= list.count() {
                    return true;
                }
                let dragged_item = list.get(dragged_index);
                list.remove_at(dragged_index);
                list.insert(target_index, dragged_item);
                return true;
            }
        )*};
    }

    move_in!(
        Option<BoxedValue>,
        BoxedValue,
        String,
        i32,
        i64,
        u32,
        u64,
        f32,
        f64,
        bool,
        char,
        Ref<FerroObject>,
        Ref<Control>,
    );
    false
}

impl ItemDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            enable_drag: Cell::new(false),
            drag_started: Cell::new(false),
            start: Cell::new(Point::default()),
            dragged_index: Cell::new(0),
            target_index: Cell::new(0),
            items_control: RefCell::new(None),
            dragged_container: RefCell::new(None),
            captured: Cell::new(false),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the orientation of the drag operation.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// Gets or sets the horizontal drag threshold in pixels.
    pub fn horizontal_drag_threshold(&self) -> f64 {
        self.get_value(Self::horizontal_drag_threshold_property())
    }

    pub fn set_horizontal_drag_threshold(&self, value: f64) {
        self.set_value(Self::horizontal_drag_threshold_property(), value)
    }

    /// Gets or sets the vertical drag threshold in pixels.
    pub fn vertical_drag_threshold(&self) -> f64 {
        self.get_value(Self::vertical_drag_threshold_property())
    }

    pub fn set_vertical_drag_threshold(&self, value: f64) {
        self.set_value(Self::vertical_drag_threshold_property(), value)
    }

    fn pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let properties = e.properties();
        let items_control = self
            .associated_object_as::<Control>()
            .and_then(|associated_object| associated_object.parent())
            .and_then(|parent| parent.cast::<ItemsControl>());
        let Some(items_control) = items_control.filter(|_| properties.is_left_button_pressed && self.is_enabled())
        else {
            return;
        };

        self.enable_drag.set(true);
        self.drag_started.set(false);
        let items_control_visual: &Visual = &items_control;
        self.start.set(e.get_position(Some(items_control_visual)));
        self.dragged_index.set(-1);
        self.target_index.set(-1);
        *self.items_control.borrow_mut() = Some(items_control.clone());
        let dragged_container = self.associated_object_as::<Control>();
        *self.dragged_container.borrow_mut() = dragged_container.clone();

        if let Some(dragged_container) = &dragged_container {
            Self::set_dragging_pseudo_classes(dragged_container, true);
        }

        Self::add_transforms(Some(&items_control));

        self.captured.set(true);
    }

    fn pointer_released(&self, e: &PointerReleasedEventArgs) {
        if self.captured.get() {
            if e.initial_press_mouse_button() == MouseButton::Left {
                self.released();
            }

            self.captured.set(false);
        }
    }

    fn pointer_capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        self.released();
        self.captured.set(false);
    }

    fn released(&self) {
        if !self.enable_drag.get() {
            return;
        }

        let items_control = self.items_control.borrow().clone();
        Self::remove_transforms(items_control.as_ref());

        if let Some(items_control) = &items_control {
            for control in items_control.get_realized_containers() {
                Self::set_dragging_pseudo_classes(&control, true);
            }
        }

        if self.drag_started.get() {
            let dragged_index = self.dragged_index.get();
            let target_index = self.target_index.get();
            if dragged_index >= 0 && target_index >= 0 && dragged_index != target_index {
                self.move_dragged_item(items_control.as_ref(), dragged_index, target_index);
            }
        }

        if let Some(items_control) = &items_control {
            for control in items_control.get_realized_containers() {
                Self::set_dragging_pseudo_classes(&control, false);
            }
        }

        let dragged_container = self.dragged_container.borrow().clone();
        if let Some(dragged_container) = &dragged_container {
            Self::set_dragging_pseudo_classes(dragged_container, false);
        }

        self.dragged_index.set(-1);
        self.target_index.set(-1);
        self.enable_drag.set(false);
        self.drag_started.set(false);
        *self.items_control.borrow_mut() = None;

        *self.dragged_container.borrow_mut() = None;
    }

    fn add_transforms(items_control: Option<&Ref<ItemsControl>>) {
        let Some(items_control) = items_control else {
            return;
        };

        for i in 0..items_control.items().count() {
            if let Some(container) = items_control.container_from_index(i as i32) {
                Self::set_translate_transform(&container, 0.0, 0.0);
            }
        }
    }

    fn remove_transforms(items_control: Option<&Ref<ItemsControl>>) {
        let Some(items_control) = items_control else {
            return;
        };

        for i in 0..items_control.items().count() {
            if let Some(container) = items_control.container_from_index(i as i32) {
                Self::set_translate_transform(&container, 0.0, 0.0);
            }
        }
    }

    fn move_dragged_item(&self, items_control: Option<&Ref<ItemsControl>>, dragged_index: i32, target_index: i32) {
        let Some(items_control) = items_control else {
            return;
        };
        let (dragged, target) = (dragged_index as usize, target_index as usize);

        if let Some(items_source) = items_control.items_source() {
            if move_in_items_source(&items_source, dragged, target) {
                if let Some(selecting_items_control) = items_control.cast::<SelectingItemsControl>() {
                    selecting_items_control.set_selected_index(target_index);
                }
            }
        } else {
            let item_collection = items_control.items();
            if !item_collection.is_read_only() && dragged < item_collection.count() && target < item_collection.count() {
                let dragged_item = item_collection.get_at(dragged);
                item_collection.remove_at(dragged);
                item_collection.insert(target, dragged_item);

                if let Some(control) = items_control.container_from_index(target_index) {
                    // Deviation (DEVIATIONS.md, Behaviors): the reference attaches the behavior
                    // to the container at the target index, which throws when the behavior is
                    // attached to another container; the port attaches it only when it is free
                    // or already attached to that container (where the attach does nothing).
                    let associated_object = self.associated_object();
                    if associated_object.as_ref().is_none_or(|associated_object| associated_object.ptr_eq(&control)) {
                        self.attach(control);
                    }
                }
                if let Some(selecting_items_control) = items_control.cast::<SelectingItemsControl>() {
                    selecting_items_control.set_selected_index(target_index);
                }
            }
        }
    }

    fn pointer_moved(&self, e: &PointerEventArgs) {
        let properties = e.properties();
        if !(self.captured.get() && properties.is_left_button_pressed && self.is_enabled()) {
            return;
        }

        let items_control = self.items_control.borrow().clone();
        let dragged_container = self.dragged_container.borrow().clone();
        let (Some(items_control), Some(dragged_container)) = (items_control, dragged_container) else {
            return;
        };
        if dragged_container.render_transform().is_none() || !self.enable_drag.get() {
            return;
        }

        let orientation = self.orientation();
        let items_control_visual: &Visual = &items_control;
        let position = e.get_position(Some(items_control_visual));
        let start = self.start.get();
        let delta = match orientation {
            Orientation::Horizontal => position.x - start.x,
            Orientation::Vertical => position.y - start.y,
        };

        if !self.drag_started.get() {
            if exceeds_drag_threshold(
                orientation,
                start,
                position,
                self.horizontal_drag_threshold(),
                self.vertical_drag_threshold(),
            ) {
                self.drag_started.set(true);
            } else {
                return;
            }
        }

        match orientation {
            Orientation::Horizontal => Self::set_translate_transform(&dragged_container, delta, 0.0),
            Orientation::Vertical => Self::set_translate_transform(&dragged_container, 0.0, delta),
        }

        self.dragged_index.set(items_control.index_from_container(&dragged_container));
        self.target_index.set(-1);

        let dragged_bounds = dragged_container.bounds();

        for i in 0..items_control.items().count() {
            let Some(target_container) = items_control.container_from_index(i as i32) else {
                continue;
            };
            if target_container.render_transform().is_none() || target_container.ptr_eq(&dragged_container) {
                continue;
            }

            let target_bounds = target_container.bounds();
            let target_index = items_control.index_from_container(&target_container);

            let shift = target_shift(orientation, dragged_bounds, delta, target_bounds);
            match (shift, orientation) {
                (TargetShift::Backward, Orientation::Horizontal) => {
                    Self::set_translate_transform(&target_container, -dragged_bounds.width, 0.0)
                }
                (TargetShift::Backward, Orientation::Vertical) => {
                    Self::set_translate_transform(&target_container, 0.0, -dragged_bounds.height)
                }
                (TargetShift::Forward, Orientation::Horizontal) => {
                    Self::set_translate_transform(&target_container, dragged_bounds.width, 0.0)
                }
                (TargetShift::Forward, Orientation::Vertical) => {
                    Self::set_translate_transform(&target_container, 0.0, dragged_bounds.height)
                }
                (TargetShift::None, _) => Self::set_translate_transform(&target_container, 0.0, 0.0),
            }

            self.target_index.set(next_target_index(self.target_index.get(), target_index, shift));
        }
    }

    pub(crate) fn set_dragging_pseudo_classes(control: &Control, is_dragging: bool) {
        if is_dragging {
            control.pseudo_classes().add_pseudo(":dragging");
        } else {
            control.pseudo_classes().remove_pseudo(":dragging");
        }
    }

    fn set_translate_transform(control: &Control, x: f64, y: f64) {
        let mut transform_builder = TransformOperationsBuilder::new(1);
        transform_builder.append_translate(x, y);
        let transform: Rc<dyn ITransform> = transform_builder.build();
        control.set_render_transform(Some(transform));
    }
}
