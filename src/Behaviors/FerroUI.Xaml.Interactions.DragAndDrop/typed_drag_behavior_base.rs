//! Port of `TypedDragBehaviorBase.cs`.

use crate::context_drag_behavior_base::{do_drag_drop, exceeds_drag_threshold, same_data_context};
use crate::routed_handlers::RoutedHandlers;
use crate::values::boxed_object;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{
    InputElement, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerUpdateKind,
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

/// Behavior base class that initiates a drag operation for a specific data type: the press starts
/// a drag only over an element whose data context is of the [`data_type`](Self::data_type).
///
/// The class is abstract: a deriving class implements the members of
/// [`TypedDragBehaviorBaseImpl`].
#[repr(C)]
pub struct TypedDragBehaviorBase {
    base: StyledElementBehaviorOfT,
    drag_start_point: Cell<Point>,
    trigger_event: RefCell<Option<PointerPressedEventArgs>>,
    value: RefCell<Option<BoxedValue>>,
    lock: Cell<bool>,
    handlers: RoutedHandlers,
}

ferro_class! {
    TypedDragBehaviorBase: StyledElementBehaviorOfT, virtuals TypedDragBehaviorBaseImpl: StyledElementBehaviorImpl {
        /// Called before the drag operation begins.
        fn on_before_drag_drop(this, sender: Option<&BoxedValue>, e: &PointerEventArgs, context: Option<&BoxedValue>);
        /// Called after the drag operation completes.
        fn on_after_drag_drop(this, sender: Option<&BoxedValue>, e: &PointerEventArgs, context: Option<&BoxedValue>);
    }
}

ferroui_base::ferro_impl_classes!(TypedDragBehaviorBase: FerroObjectImpl, StyledElementImpl);

impl TypedDragBehaviorBaseImpl for TypedDragBehaviorBase {
    fn on_before_drag_drop(
        _this: &Self,
        _sender: Option<&BoxedValue>,
        _e: &PointerEventArgs,
        _context: Option<&BoxedValue>,
    ) {
        panic!("TypedDragBehaviorBase is abstract: 'on_before_drag_drop' must be implemented by the deriving class")
    }

    fn on_after_drag_drop(
        _this: &Self,
        _sender: Option<&BoxedValue>,
        _e: &PointerEventArgs,
        _context: Option<&BoxedValue>,
    ) {
        panic!("TypedDragBehaviorBase is abstract: 'on_after_drag_drop' must be implemented by the deriving class")
    }
}

ferro_properties! {
    impl TypedDragBehaviorBase {
        /// Identifies the `DataType` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the `Type` of the reference is the value type of
        /// the framework (what `{x:Type}` gives in markup): the handle type of a class, the type of
        /// a model object or of a value.
        pub fn data_type_property() -> StyledProperty<Option<ValueType>> {
            FerroProperty::register::<TypedDragBehaviorBase, _>("DataType", None)
        }
    }
}

/// The routes of the pointer handlers: direct, tunnel and bubble.
const ALL_ROUTES: RoutingStrategies =
    RoutingStrategies::DIRECT.union(RoutingStrategies::TUNNEL).union(RoutingStrategies::BUBBLE);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TypedDragBehaviorBase {
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
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.handlers.clear();
        }
    }
}

/// Not from the reference: whether `value` is of `data_type` (`Type.IsInstanceOfType`): an object
/// of the class or of a class deriving from it, a value of the type, or one assignable to it
/// without conversion.
pub(crate) fn is_instance_of_type(data_type: ValueType, value: Option<&BoxedValue>) -> bool {
    let Some(value) = value else {
        return false;
    };

    ValueTypes::is_assignable(ValueType::of_value(&**value), data_type) || ValueTypes::try_cast(value, data_type).is_some()
}

impl TypedDragBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            drag_start_point: Cell::new(Point::default()),
            trigger_event: RefCell::new(None),
            value: RefCell::new(None),
            lock: Cell::new(false),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Gets or sets the data type allowed for dragging.
    pub fn data_type(&self) -> Option<ValueType> {
        self.get_value(Self::data_type_property())
    }

    pub fn set_data_type(&self, value: Option<ValueType>) {
        self.set_value(Self::data_type_property(), value)
    }

    fn associated_object_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let properties = e.properties();
        if !properties.is_left_button_pressed {
            return;
        }

        let Some(control) = e.source().and_then(|source| source.cast::<Control>()) else {
            return;
        };
        let associated_data_context =
            self.associated_object_as::<Control>().and_then(|associated_object| associated_object.data_context());
        let data_context = control.data_context();
        let Some(data_type) = self.data_type() else {
            return;
        };
        if same_data_context(associated_data_context.as_ref(), data_context.as_ref())
            && is_instance_of_type(data_type, data_context.as_ref())
        {
            self.drag_start_point.set(e.get_position(None));
            *self.trigger_event.borrow_mut() = Some(e.clone());
            *self.value.borrow_mut() = data_context;
            self.lock.set(true);
        }
    }

    fn associated_object_pointer_released(&self, e: &PointerReleasedEventArgs) {
        let properties = e.properties();
        if properties.pointer_update_kind == PointerUpdateKind::LeftButtonReleased && self.trigger_event.borrow().is_some()
        {
            *self.trigger_event.borrow_mut() = None;
            *self.value.borrow_mut() = None;
            self.lock.set(false);
        }
    }

    fn associated_object_pointer_moved(&self, sender: BoxedValue, e: &PointerEventArgs) {
        let properties = e.properties();
        let trigger_event = self.trigger_event.borrow().clone();
        let Some(trigger_event) = trigger_event.filter(|_| properties.is_left_button_pressed) else {
            return;
        };

        let point = e.get_position(None);
        if !exceeds_drag_threshold(self.drag_start_point.get(), point, 3.0, 3.0) {
            return;
        }

        if self.lock.get() {
            self.lock.set(false);
        } else {
            return;
        }

        let value = self.value.borrow().clone();
        self.on_before_drag_drop(Some(&sender), &trigger_event, value.as_ref());

        let operation = do_drag_drop(&trigger_event, value.as_ref());
        let this = self.to_ref();
        drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            operation.await;

            this.on_after_drag_drop(Some(&sender), &trigger_event, value.as_ref());

            *this.trigger_event.borrow_mut() = None;
            *this.value.borrow_mut() = None;
        }));
    }
}
