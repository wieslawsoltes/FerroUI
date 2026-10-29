//! Port of `ItemsControl/ItemNudgeDropBehavior.cs`.

use ferroui_base::input::{DragDrop, DragEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::layout::Orientation;
use ferroui_base::media::transformation::TransformOperationsBuilder;
use ferroui_base::media::ITransform;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty, Vector,
};
use ferroui_controls::{Control, ItemsControl, ListBox};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::Cell;
use std::rc::Rc;

/// Moves the containers of the associated [`ItemsControl`] aside while an item is dragged over
/// it: the containers after the drag position are translated by their size along the
/// [`orientation`](Self::orientation), to show where the item drops. The translations are removed
/// when the drag leaves or drops.
#[repr(C)]
pub struct ItemNudgeDropBehavior {
    base: StyledElementBehaviorOfT,
    // The handlers added to the associated object (the reference removes its delegates).
    drag_leave_handler: Cell<Option<RoutedEventHandlerToken>>,
    drag_over_handler: Cell<Option<RoutedEventHandlerToken>>,
    drop_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(ItemNudgeDropBehavior: StyledElementBehaviorOfT);
ferro_class_info!(ItemNudgeDropBehavior { new: ItemNudgeDropBehavior::new });
ferroui_base::ferro_impl_classes!(ItemNudgeDropBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ItemNudgeDropBehavior {
        /// Identifies the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<ItemNudgeDropBehavior, _>("Orientation", Orientation::Vertical)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ItemNudgeDropBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<ItemsControl>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            this.drag_leave_handler.set(Some(associated_object.add_handler(
                DragDrop::drag_leave_event(),
                move |sender: &Interactive, _e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_drag_leave(sender);
                    }
                },
            )));

            let behavior = this.to_ref().downgrade();
            this.drag_over_handler.set(Some(associated_object.add_handler(
                DragDrop::drag_over_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_drag_over(sender, e);
                    }
                },
            )));

            let behavior = this.to_ref().downgrade();
            this.drop_handler.set(Some(associated_object.add_handler(
                DragDrop::drop_event(),
                move |sender: &Interactive, _e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_drop(sender);
                    }
                },
            )));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<ItemsControl>() else {
                return;
            };

            if let Some(handler) = this.drag_leave_handler.take() {
                associated_object.remove_handler(DragDrop::drag_leave_event(), handler);
            }
            if let Some(handler) = this.drag_over_handler.take() {
                associated_object.remove_handler(DragDrop::drag_over_event(), handler);
            }
            if let Some(handler) = this.drop_handler.take() {
                associated_object.remove_handler(DragDrop::drop_event(), handler);
            }
        }
    }
}

impl ItemNudgeDropBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<ItemsControl as StaticType>::TYPE),
            drag_leave_handler: Cell::new(None),
            drag_over_handler: Cell::new(None),
            drop_handler: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the orientation along which the containers move aside.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    fn apply_translation(control: &Control, x: f64, y: f64) {
        let mut transform_builder = TransformOperationsBuilder::new(1);
        transform_builder.append_translate(x, y);
        let transform: Rc<dyn ITransform> = transform_builder.build();
        control.set_render_transform(Some(transform));
    }

    fn remove_translations(&self, sender: &Interactive) {
        if let Some(items_control) = sender.to_ref().cast::<ItemsControl>() {
            for container in items_control.get_realized_containers() {
                Self::apply_translation(&container, 0.0, 0.0);
            }
        }
    }

    fn on_drop(&self, sender: &Interactive) {
        self.remove_translations(sender);
    }

    fn on_drag_leave(&self, sender: &Interactive) {
        self.remove_translations(sender);
    }

    fn on_drag_over(&self, sender: &Interactive, e: &DragEventArgs) {
        let Some(items_control) = sender.to_ref().cast::<ItemsControl>() else {
            return;
        };

        let is_horizontal = self.orientation() == Orientation::Horizontal;

        let drag_position = e.get_position(&items_control);

        let mut scroll_offset = Vector::ZERO;

        if let Some(list_box) = items_control.clone().cast::<ListBox>() {
            if let Some(scroll) = list_box.scroll() {
                if let Some(scrollable) = scroll.as_scrollable() {
                    scroll_offset = scrollable.offset();
                }
            }
        }

        for index in 0..items_control.item_count() {
            let Some(container) = items_control.container_from_index(index) else {
                continue;
            };

            let bounds = container.bounds();
            let container_mid_point =
                if is_horizontal { bounds.center().x - scroll_offset.x } else { bounds.center().y - scroll_offset.y };

            let translation_x =
                if is_horizontal && drag_position.x <= container_mid_point { bounds.width } else { 0.0 };
            let translation_y =
                if !is_horizontal && drag_position.y <= container_mid_point { bounds.height } else { 0.0 };

            Self::apply_translation(&container, translation_x, translation_y);
        }
    }
}
