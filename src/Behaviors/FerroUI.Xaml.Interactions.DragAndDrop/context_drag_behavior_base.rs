//! Port of `ContextDragBehaviorBase.cs`.

use crate::drag_drop_context_store::{DragDropContextStore, RemoveContextOnDrop};
use crate::routed_handlers::RoutedHandlers;
use crate::values::boxed_object;
use crate::ContextDropBehaviorBase;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{
    DataTransfer, DataTransferItem, DragDrop, LocalBoxFuture, DragDropEffects, InputElement, Key, KeyEventArgs, KeyModifiers,
    MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_properties, BoxedValue, FerroObjectImpl, FerroProperty, Point, StaticType, StyledElementImpl,
    StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};

/// Behavior base class that starts a drag operation using the associated context data: the
/// [`context`](Self::context), or else the data context of the associated control, travels with
/// the drag (by its key in the drag data) to the drop behaviors of this crate.
///
/// The class is abstract: a deriving class implements the members of
/// [`ContextDragBehaviorBaseImpl`].
#[repr(C)]
pub struct ContextDragBehaviorBase {
    base: StyledElementBehaviorOfT,
    drag_start_point: Cell<Point>,
    trigger_event: RefCell<Option<PointerPressedEventArgs>>,
    lock: Cell<bool>,
    captured: Cell<bool>,
    handlers: RoutedHandlers,
}

ferro_class! {
    ContextDragBehaviorBase: StyledElementBehaviorOfT, virtuals ContextDragBehaviorBaseImpl: StyledElementBehaviorImpl {
        /// Called before the drag operation begins.
        fn on_before_drag_drop(this, sender: Option<&BoxedValue>, e: &PointerEventArgs, context: Option<&BoxedValue>);
        /// Called after the drag operation completes.
        fn on_after_drag_drop(this, sender: Option<&BoxedValue>, e: &PointerEventArgs, context: Option<&BoxedValue>);
    }
}

ferroui_base::ferro_impl_classes!(ContextDragBehaviorBase: FerroObjectImpl, StyledElementImpl);

impl ContextDragBehaviorBaseImpl for ContextDragBehaviorBase {
    fn on_before_drag_drop(
        _this: &Self,
        _sender: Option<&BoxedValue>,
        _e: &PointerEventArgs,
        _context: Option<&BoxedValue>,
    ) {
        panic!("ContextDragBehaviorBase is abstract: 'on_before_drag_drop' must be implemented by the deriving class")
    }

    fn on_after_drag_drop(
        _this: &Self,
        _sender: Option<&BoxedValue>,
        _e: &PointerEventArgs,
        _context: Option<&BoxedValue>,
    ) {
        panic!("ContextDragBehaviorBase is abstract: 'on_after_drag_drop' must be implemented by the deriving class")
    }
}

ferro_properties! {
    impl ContextDragBehaviorBase {
        /// Identifies the `Context` property.
        pub fn context_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContextDragBehaviorBase, _>("Context", None)
        }

        /// Identifies the `HorizontalDragThreshold` property.
        pub fn horizontal_drag_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ContextDragBehaviorBase, _>("HorizontalDragThreshold", 3.0)
        }

        /// Identifies the `VerticalDragThreshold` property.
        pub fn vertical_drag_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ContextDragBehaviorBase, _>("VerticalDragThreshold", 3.0)
        }
    }
}

/// The routes of the pointer handlers: direct, tunnel and bubble.
const ALL_ROUTES: RoutingStrategies =
    RoutingStrategies::DIRECT.union(RoutingStrategies::TUNNEL).union(RoutingStrategies::BUBBLE);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ContextDragBehaviorBase {
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
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::key_down_event(),
                move |_: &Interactive, e: &KeyEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_key_down(e);
                    }
                },
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.handlers.clear();
        }
    }
}

/// Not from the reference: the effects a drag allows, from the keyboard modifiers of the press
/// that started it (`DoDragDrop` of the reference).
pub(crate) fn drag_effects_of(key_modifiers: KeyModifiers) -> DragDropEffects {
    let mut effect = DragDropEffects::NONE;

    if key_modifiers.contains(KeyModifiers::ALT) {
        effect |= DragDropEffects::LINK;
    } else if key_modifiers.contains(KeyModifiers::SHIFT) {
        effect |= DragDropEffects::MOVE;
    } else if key_modifiers.contains(KeyModifiers::CONTROL) {
        effect |= DragDropEffects::COPY;
    } else {
        effect |= DragDropEffects::MOVE;
    }

    effect
}

/// Not from the reference: whether `a` and `b` are the same data context (`==` of the reference
/// on two objects: reference equality).
pub(crate) fn same_data_context(a: Option<&BoxedValue>, b: Option<&BoxedValue>) -> bool {
    ValueTypes::identity_equals(a, b)
}

/// Not from the reference: whether the pointer moved from `start` to `point` farther than one of
/// the thresholds.
pub(crate) fn exceeds_drag_threshold(start: Point, point: Point, horizontal: f64, vertical: f64) -> bool {
    let diff = start - point;
    diff.x.abs() > horizontal || diff.y.abs() > vertical
}

/// Starts the drag of `value` with the press `trigger_event` (`DoDragDrop` of the reference); the
/// returned future completes with the drag and removes the value from the context store then.
pub(crate) fn do_drag_drop(trigger_event: &PointerPressedEventArgs, value: Option<&BoxedValue>) -> LocalBoxFuture<()> {
    let data = DataTransfer::new();
    let context_key = DragDropContextStore::add(value);

    if let Some(context_key) = &context_key {
        data.add(DataTransferItem::create(
            &ContextDropBehaviorBase::context_data_transfer_format(),
            Some(context_key.clone()),
        ));
    }

    let effect = drag_effects_of(trigger_event.key_modifiers());

    // try { await DoDragDropAsync(..) } finally { DragDropContextStore.Remove(contextKey) }
    let remove_context = RemoveContextOnDrop(context_key);
    let operation = DragDrop::do_drag_drop_async(trigger_event, data, effect);
    Box::pin(async move {
        let _remove_context = remove_context;
        operation.await;
    })
}

impl ContextDragBehaviorBase {
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

    /// Gets or sets context data passed to the drag handler.
    pub fn context(&self) -> Option<BoxedValue> {
        self.get_value(Self::context_property())
    }

    pub fn set_context(&self, value: Option<BoxedValue>) {
        self.set_value(Self::context_property(), value)
    }

    /// Gets or sets the horizontal distance in pixels required to start a drag.
    pub fn horizontal_drag_threshold(&self) -> f64 {
        self.get_value(Self::horizontal_drag_threshold_property())
    }

    pub fn set_horizontal_drag_threshold(&self, value: f64) {
        self.set_value(Self::horizontal_drag_threshold_property(), value)
    }

    /// Gets or sets the vertical distance in pixels required to start a drag.
    pub fn vertical_drag_threshold(&self) -> f64 {
        self.get_value(Self::vertical_drag_threshold_property())
    }

    pub fn set_vertical_drag_threshold(&self, value: f64) {
        self.set_value(Self::vertical_drag_threshold_property(), value)
    }

    fn released(&self) {
        *self.trigger_event.borrow_mut() = None;
        self.lock.set(false);
    }

    fn associated_object_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let properties = e.properties();
        if properties.is_left_button_pressed && self.is_enabled() {
            if let Some(control) = e.source().and_then(|source| source.cast::<Control>()) {
                let associated_data_context =
                    self.associated_object_as::<Control>().and_then(|associated_object| associated_object.data_context());
                if same_data_context(associated_data_context.as_ref(), control.data_context().as_ref()) {
                    self.drag_start_point.set(e.get_position(None));
                    *self.trigger_event.borrow_mut() = Some(e.clone());
                    self.lock.set(true);
                    self.captured.set(true);

                    // Drag detection must not consume the initial press. Selection and
                    // interactive content still need to observe it before a drag starts.
                    e.set_handled(false);
                    return;
                }
            }
        }
        e.set_handled(false);
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
        if !(self.captured.get() && properties.is_left_button_pressed && self.is_enabled()) {
            return;
        }

        let point = e.get_position(None);
        if !exceeds_drag_threshold(
            self.drag_start_point.get(),
            point,
            self.horizontal_drag_threshold(),
            self.vertical_drag_threshold(),
        ) {
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

        self.on_before_drag_drop(Some(&sender), &trigger_event, context.as_ref());

        // `async void` of the reference: the drag runs as a task of the dispatcher, started now.
        let operation = do_drag_drop(&trigger_event, context.as_ref());
        let this = self.to_ref();
        drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            operation.await;

            this.on_after_drag_drop(Some(&sender), &trigger_event, context.as_ref());

            *this.trigger_event.borrow_mut() = None;
        }));
    }

    fn associated_object_capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        self.released();
        self.captured.set(false);
    }

    fn associated_object_key_down(&self, e: &KeyEventArgs) {
        if e.key == Key::Escape {
            self.released();
            self.captured.set(false);
        }
    }
}
