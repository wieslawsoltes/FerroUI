//! Port of `GridDragBehavior.cs`.

use crate::routed_handlers::RoutedHandlers;
use crate::{ItemDragBehavior, SelectionAdorner};
use ferroui_base::input::{
    InputElement, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Point, Ref,
    StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Control, Grid, ItemsControl};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};

/// Allows dragging of grid child controls with optional layout copying: the associated control
/// swaps its column, row and spans (as the `copy_*` properties select) with the child of the
/// [`Grid`] (or the container of the [`ItemsControl`]) the pointer is over.
#[repr(C)]
pub struct GridDragBehavior {
    base: StyledElementBehaviorOfT,
    enable_drag: Cell<bool>,
    parent: RefCell<Option<Ref<Control>>>,
    dragged_container: RefCell<Option<Ref<Control>>>,
    adorner: RefCell<Option<Ref<Control>>>,
    captured: Cell<bool>,
    handlers: RoutedHandlers,
}

ferro_class!(GridDragBehavior: StyledElementBehaviorOfT);
ferro_class_info!(GridDragBehavior { new: GridDragBehavior::new });
ferroui_base::ferro_impl_classes!(GridDragBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl GridDragBehavior {
        /// Identifies the `CopyColumn` property.
        pub fn copy_column_property() -> StyledProperty<bool> {
            FerroProperty::register::<GridDragBehavior, _>("CopyColumn", true)
        }

        /// Identifies the `CopyRow` property.
        pub fn copy_row_property() -> StyledProperty<bool> {
            FerroProperty::register::<GridDragBehavior, _>("CopyRow", true)
        }

        /// Identifies the `CopyColumnSpan` property.
        pub fn copy_column_span_property() -> StyledProperty<bool> {
            FerroProperty::register::<GridDragBehavior, _>("CopyColumnSpan", false)
        }

        /// Identifies the `CopyRowSpan` property.
        pub fn copy_row_span_property() -> StyledProperty<bool> {
            FerroProperty::register::<GridDragBehavior, _>("CopyRowSpan", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for GridDragBehavior {
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
                        behavior.released_event(e);
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
                        behavior.pressed(e);
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
            this.handlers.clear();
        }
    }
}

/// Not from the reference: the hit test of `Moved`, the last of `candidates` other than `dragged`
/// whose bounds contain `position` (in the coordinates of their parent).
pub(crate) fn find_drop_target(
    candidates: impl IntoIterator<Item = Ref<Control>>,
    dragged: &Ref<Control>,
    position: Point,
) -> Option<Ref<Control>> {
    let mut target = None;
    for candidate in candidates {
        if candidate.ptr_eq(dragged) {
            continue;
        }

        if candidate.bounds().contains(position) {
            target = Some(candidate);
        }
    }
    target
}

impl GridDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            enable_drag: Cell::new(false),
            parent: RefCell::new(None),
            dragged_container: RefCell::new(None),
            adorner: RefCell::new(None),
            captured: Cell::new(false),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets whether to copy the dragged element's column.
    pub fn copy_column(&self) -> bool {
        self.get_value(Self::copy_column_property())
    }

    pub fn set_copy_column(&self, value: bool) {
        self.set_value(Self::copy_column_property(), value)
    }

    /// Gets or sets whether to copy the dragged element's row.
    pub fn copy_row(&self) -> bool {
        self.get_value(Self::copy_row_property())
    }

    pub fn set_copy_row(&self, value: bool) {
        self.set_value(Self::copy_row_property(), value)
    }

    /// Gets or sets whether to copy the dragged element's column span.
    pub fn copy_column_span(&self) -> bool {
        self.get_value(Self::copy_column_span_property())
    }

    pub fn set_copy_column_span(&self, value: bool) {
        self.set_value(Self::copy_column_span_property(), value)
    }

    /// Gets or sets whether to copy the dragged element's row span.
    pub fn copy_row_span(&self) -> bool {
        self.get_value(Self::copy_row_span_property())
    }

    pub fn set_copy_row_span(&self, value: bool) {
        self.set_value(Self::copy_row_span_property(), value)
    }

    /// Shows the selection adorner over `control` (not used by the reference: its call is
    /// commented out).
    #[allow(dead_code)]
    fn add_adorner(&self, control: &Ref<Control>) {
        let Some(layer) = AdornerLayer::get_adorner_layer(control) else {
            return;
        };

        let adorner = SelectionAdorner::new();
        AdornerLayer::set_adorned_element(&adorner, control);

        adorner.set_parent(control);
        layer.children().add(adorner.clone());
        *self.adorner.borrow_mut() = Some(adorner.upcast());
    }

    /// Removes the selection adorner (not used by the reference: its call is commented out).
    #[allow(dead_code)]
    fn remove_adorner(&self, control: &Ref<Control>) {
        let adorner = self.adorner.borrow().clone();
        let (Some(layer), Some(adorner)) = (AdornerLayer::get_adorner_layer(control), adorner) else {
            return;
        };

        layer.children().remove(adorner.clone());
        adorner.set_parent(None);
        *self.adorner.borrow_mut() = None;
    }

    fn pressed(&self, e: &PointerPressedEventArgs) {
        let properties = e.properties();
        let parent = self
            .associated_object_as::<Control>()
            .and_then(|associated_object| associated_object.parent())
            .and_then(|parent| parent.cast::<Control>());
        let Some(parent) = parent.filter(|_| properties.is_left_button_pressed && self.is_enabled()) else {
            return;
        };

        self.enable_drag.set(true);
        *self.parent.borrow_mut() = Some(parent);
        let dragged_container = self.associated_object_as::<Control>();
        *self.dragged_container.borrow_mut() = dragged_container.clone();

        if let Some(dragged_container) = &dragged_container {
            ItemDragBehavior::set_dragging_pseudo_classes(dragged_container, true);
        }

        // AddAdorner(_draggedContainer);

        self.captured.set(true);
    }

    fn released_event(&self, e: &PointerReleasedEventArgs) {
        if self.captured.get() {
            if e.initial_press_mouse_button() == MouseButton::Left {
                self.released();
            }

            self.captured.set(false);
        }
    }

    fn capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        self.released();
        self.captured.set(false);
    }

    fn moved(&self, e: &PointerEventArgs) {
        let properties = e.properties();
        if !(self.captured.get() && properties.is_left_button_pressed && self.is_enabled()) {
            return;
        }

        let parent = self.parent.borrow().clone();
        let dragged_container = self.dragged_container.borrow().clone();
        let (Some(parent), Some(dragged_container)) = (parent, dragged_container) else {
            return;
        };
        if !self.enable_drag.get() {
            return;
        }

        let parent_visual: &Visual = &parent;
        let position = e.get_position(Some(parent_visual));

        let target = if let Some(grid) = parent.cast::<Grid>() {
            find_drop_target(grid.children().to_vec(), &dragged_container, position)
        } else if let Some(items_control) = parent.cast::<ItemsControl>() {
            find_drop_target(items_control.get_realized_containers(), &dragged_container, position)
        } else {
            return;
        };

        let Some(target) = target else {
            return;
        };

        self.swap_cells(&dragged_container, &target);
    }

    /// Not from the reference: the end of `Moved`, which swaps the cells of the dragged control
    /// and of the target as the `copy_*` properties select.
    pub(crate) fn swap_cells(&self, dragged_container: &Control, target: &Control) {
        let copy_column = self.copy_column();
        let copy_row = self.copy_row();
        let copy_column_span = self.copy_column_span();
        let copy_row_span = self.copy_row_span();

        let source_column = Grid::get_column(dragged_container);
        let source_row = Grid::get_row(dragged_container);
        let source_column_span = Grid::get_column_span(dragged_container);
        let source_row_span = Grid::get_row_span(dragged_container);

        let target_column = Grid::get_column(target);
        let target_row = Grid::get_row(target);
        let target_column_span = Grid::get_column_span(target);
        let target_row_span = Grid::get_row_span(target);

        if copy_column {
            Grid::set_column(dragged_container, target_column);
        }

        if copy_row {
            Grid::set_row(dragged_container, target_row);
        }

        if copy_column_span {
            Grid::set_column_span(dragged_container, target_column_span);
        }

        if copy_row_span {
            Grid::set_row_span(dragged_container, target_row_span);
        }

        if copy_column {
            Grid::set_column(target, source_column);
        }

        if copy_row {
            Grid::set_row(target, source_row);
        }

        if copy_column_span {
            Grid::set_column_span(target, source_column_span);
        }

        if copy_row_span {
            Grid::set_row_span(target, source_row_span);
        }
    }

    fn released(&self) {
        if self.enable_drag.get() {
            if self.parent.borrow().is_some() && self.dragged_container.borrow().is_some() {
                // RemoveAdorner(_draggedContainer);
            }

            let dragged_container = self.dragged_container.borrow().clone();
            if let Some(dragged_container) = &dragged_container {
                ItemDragBehavior::set_dragging_pseudo_classes(dragged_container, false);
            }

            self.enable_drag.set(false);
            *self.parent.borrow_mut() = None;
            *self.dragged_container.borrow_mut() = None;
        }
    }
}
