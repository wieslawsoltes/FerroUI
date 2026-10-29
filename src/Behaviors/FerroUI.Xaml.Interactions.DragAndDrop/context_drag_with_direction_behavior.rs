//! Port of `ContextDragWithDirectionBehavior.cs`.

use crate::context_drag_behavior_base::{drag_effects_of, exceeds_drag_threshold, same_data_context};
use crate::drag_drop_context_store::{DragDropContextStore, RemoveContextOnDrop};
use crate::routed_handlers::RoutedHandlers;
use crate::values::boxed_object;
use crate::{ContextDropBehaviorBase, IDragHandler};
use ferroui_base::input::{
    DataFormat, DataFormatOf, DataTransfer, DataTransferItem, DragDrop, LocalBoxFuture, InputElement, MouseButton,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Point,
    Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Behavior that starts drag and drop with information about drag direction: the drag data holds
/// `"up"` or `"down"`, as the pointer moved up or down to start the drag.
#[repr(C)]
pub struct ContextDragWithDirectionBehavior {
    base: StyledElementBehaviorOfT,
    drag_start_point: Cell<Point>,
    trigger_event: RefCell<Option<PointerPressedEventArgs>>,
    lock: Cell<bool>,
    captured: Cell<bool>,
    handlers: RoutedHandlers,
}

ferro_class!(ContextDragWithDirectionBehavior: StyledElementBehaviorOfT);
ferro_class_info!(ContextDragWithDirectionBehavior { new: ContextDragWithDirectionBehavior::new });
ferroui_base::ferro_impl_classes!(ContextDragWithDirectionBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ContextDragWithDirectionBehavior {
        /// Identifies the `Context` property.
        pub fn context_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContextDragWithDirectionBehavior, _>("Context", None)
        }

        /// Identifies the `Handler` property.
        pub fn handler_property() -> StyledProperty<Option<Rc<dyn IDragHandler>>> {
            FerroProperty::register::<ContextDragWithDirectionBehavior, _>("Handler", None)
        }

        /// Identifies the `HorizontalDragThreshold` property.
        pub fn horizontal_drag_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ContextDragWithDirectionBehavior, _>("HorizontalDragThreshold", 3.0)
        }

        /// Identifies the `VerticalDragThreshold` property.
        pub fn vertical_drag_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ContextDragWithDirectionBehavior, _>("VerticalDragThreshold", 3.0)
        }
    }
}

/// The routes of the pointer handlers: direct, tunnel and bubble.
const ALL_ROUTES: RoutingStrategies =
    RoutingStrategies::DIRECT.union(RoutingStrategies::TUNNEL).union(RoutingStrategies::BUBBLE);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ContextDragWithDirectionBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_pressed_event(),
                move |_: &Interactive, e: &PointerPressedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_pointer_pressed(e);
                    }
                },
                ALL_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_released_event(),
                move |_: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_pointer_released(e);
                    }
                },
                ALL_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_moved_event(),
                move |sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_pointer_moved(boxed_object(&sender.to_ref()), e);
                    }
                },
                ALL_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_capture_lost_event(),
                move |_: &Interactive, e: &PointerCaptureLostEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_capture_lost(e);
                    }
                },
                ALL_ROUTES,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.handlers.clear();
        }
    }
}

impl ContextDragWithDirectionBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            drag_start_point: Cell::new(Point::default()),
            trigger_event: RefCell::new(None),
            lock: Cell::new(false),
            captured: Cell::new(false),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The application format of the direction in the drag data.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the identifier of the format names the namespace of
    /// the port.
    pub(crate) fn direction_data_transfer_format() -> DataFormatOf<String> {
        DataFormat::create_string_application_format("FerroUI.Xaml.Interactions.DragAndDrop.Direction")
    }

    /// Gets or sets the context used for drag operations.
    pub fn context(&self) -> Option<BoxedValue> {
        self.get_value(Self::context_property())
    }

    pub fn set_context(&self, value: Option<BoxedValue>) {
        self.set_value(Self::context_property(), value)
    }

    /// Gets or sets the drag handler to notify.
    pub fn handler(&self) -> Option<Rc<dyn IDragHandler>> {
        self.get_value(Self::handler_property())
    }

    pub fn set_handler(&self, value: Option<Rc<dyn IDragHandler>>) {
        self.set_value(Self::handler_property(), value)
    }

    /// Gets or sets the horizontal drag threshold.
    pub fn horizontal_drag_threshold(&self) -> f64 {
        self.get_value(Self::horizontal_drag_threshold_property())
    }

    pub fn set_horizontal_drag_threshold(&self, value: f64) {
        self.set_value(Self::horizontal_drag_threshold_property(), value)
    }

    /// Gets or sets the vertical drag threshold.
    pub fn vertical_drag_threshold(&self) -> f64 {
        self.get_value(Self::vertical_drag_threshold_property())
    }

    pub fn set_vertical_drag_threshold(&self, value: f64) {
        self.set_value(Self::vertical_drag_threshold_property(), value)
    }

    fn do_drag_drop(
        trigger_event: &PointerPressedEventArgs,
        value: Option<&BoxedValue>,
        direction: &str,
    ) -> LocalBoxFuture<()> {
        let data = DataTransfer::new();
        let context_key = DragDropContextStore::add(value);

        if let Some(context_key) = &context_key {
            data.add(DataTransferItem::create(
                &ContextDropBehaviorBase::context_data_transfer_format(),
                Some(context_key.clone()),
            ));
        }

        data.add(DataTransferItem::create(&Self::direction_data_transfer_format(), Some(direction.to_string())));

        let effect = drag_effects_of(trigger_event.key_modifiers());

        let remove_context = RemoveContextOnDrop(context_key);
        let operation = DragDrop::do_drag_drop_async(trigger_event, data, effect);
        Box::pin(async move {
            let _remove_context = remove_context;
            operation.await;
        })
    }

    fn released(&self) {
        *self.trigger_event.borrow_mut() = None;
        self.lock.set(false);
    }

    fn associated_object_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let properties = e.properties();
        if properties.is_left_button_pressed {
            if let Some(control) = e.source().and_then(|source| source.cast::<Control>()) {
                let associated_data_context =
                    self.associated_object_as::<Control>().and_then(|associated_object| associated_object.data_context());
                if same_data_context(associated_data_context.as_ref(), control.data_context().as_ref()) {
                    self.drag_start_point.set(e.get_position(None));
                    *self.trigger_event.borrow_mut() = Some(e.clone());
                    self.lock.set(true);
                    self.captured.set(true);
                }
            }
        }
    }

    fn associated_object_pointer_released(&self, e: &PointerReleasedEventArgs) {
        if self.captured.get() {
            if e.initial_press_mouse_button() == MouseButton::Left && self.trigger_event.borrow().is_some() {
                self.released();
            }

            self.captured.set(false);
        }
    }

    fn associated_object_pointer_moved(&self, sender: BoxedValue, e: &PointerEventArgs) {
        let properties = e.properties();
        let trigger_event = self.trigger_event.borrow().clone();
        let Some(trigger_event) = trigger_event else {
            return;
        };
        if !(self.captured.get() && properties.is_left_button_pressed) {
            return;
        }

        let point = e.get_position(None);
        let start = self.drag_start_point.get();
        if !exceeds_drag_threshold(start, point, self.horizontal_drag_threshold(), self.vertical_drag_threshold()) {
            return;
        }

        if self.lock.get() {
            self.lock.set(false);
        } else {
            return;
        }

        let context = self.context().or_else(|| {
            self.associated_object_as::<Control>().and_then(|associated_object| associated_object.data_context())
        });

        if let Some(handler) = self.handler() {
            handler.before_drag_drop(Some(&sender), &trigger_event, context.as_ref());
        }

        let diff = start - point;
        let direction = if diff.y > 0.0 { "up" } else { "down" };
        let operation = Self::do_drag_drop(&trigger_event, context.as_ref(), direction);
        let this = self.to_ref();
        drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            operation.await;

            if let Some(handler) = this.handler() {
                handler.after_drag_drop(Some(&sender), &trigger_event, context.as_ref());
            }

            *this.trigger_event.borrow_mut() = None;
        }));
    }

    fn associated_object_capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        self.released();
        self.captured.set(false);
    }
}
