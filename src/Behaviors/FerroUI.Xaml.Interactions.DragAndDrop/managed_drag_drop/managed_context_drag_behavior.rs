//! Port of `ManagedDragDrop/ManagedContextDragBehavior.cs`.

use crate::context_drag_behavior_base::{exceeds_drag_threshold, same_data_context};
use crate::routed_handlers::RoutedHandlers;
use crate::{DragPreviewService, ManagedDragDropService};
use ferroui_base::input::{
    DragDropEffects, InputElement, KeyModifiers, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Point, Ref, StaticType, StyledElement, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{as_selectable, Control, TopLevel};
use ferroui_system::threading::tasks::TaskCompletionSource;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// Whether a managed drag of any behavior is in progress (`s_isDragging`).
    static IS_DRAGGING: Cell<bool> = const { Cell::new(false) };
}

/// Behavior that initiates an in-process managed drag with an optional preview window.
///
/// This avoids the drag and drop of the system and integrates with [`ManagedDragDropService`]:
/// the drop targets are [`ManagedContextDropBehavior`](crate::ManagedContextDropBehavior)s.
#[repr(C)]
pub struct ManagedContextDragBehavior {
    base: StyledElementBehaviorOfT,
    drag_start_point: Cell<Point>,
    trigger_event: RefCell<Option<PointerPressedEventArgs>>,
    lock: Cell<bool>,
    captured: Cell<bool>,
    top_level: RefCell<Option<Ref<TopLevel>>>,
    internal_dragging: Cell<bool>,
    internal_drag_tcs: RefCell<Option<TaskCompletionSource<bool>>>,
    // Stores the calculated preview offset (TopLeftOfControl - PointerPosition) in TopLevel client
    // coordinates when enabled
    calculated_preview_offset: Cell<Option<Point>>,
    handlers: RoutedHandlers,
    top_level_handlers: RoutedHandlers,
    detached_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(ManagedContextDragBehavior: StyledElementBehaviorOfT);
ferro_class_info!(ManagedContextDragBehavior { new: ManagedContextDragBehavior::new });
ferroui_base::ferro_impl_classes!(ManagedContextDragBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ManagedContextDragBehavior {
        /// Gets or sets the context value used as a drag payload when the drag starts.
        pub fn context_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ManagedContextDragBehavior, _>("Context", None)
        }

        /// Gets or sets the data template used to render the drag preview window content.
        pub fn preview_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<ManagedContextDragBehavior, _>("PreviewTemplate", None)
        }

        /// Gets or sets the minimal horizontal distance required to start dragging.
        pub fn horizontal_drag_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ManagedContextDragBehavior, _>("HorizontalDragThreshold", 3.0)
        }

        /// Gets or sets the minimal vertical distance required to start dragging.
        pub fn vertical_drag_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ManagedContextDragBehavior, _>("VerticalDragThreshold", 3.0)
        }

        /// Gets or sets the fixed logical offset applied to the preview position. Ignored when
        /// `UsePointerRelativePreviewOffset` is true and a pointer-relative offset is calculated.
        pub fn preview_offset_property() -> StyledProperty<Point> {
            FerroProperty::register::<ManagedContextDragBehavior, _>("PreviewOffset", Point::new(0.0, 0.0))
        }

        /// Gets or sets the data format name used to identify the payload in managed drag
        /// operations.
        pub fn data_format_property() -> StyledProperty<String> {
            FerroProperty::register::<ManagedContextDragBehavior, _>("DataFormat", "Context".to_string())
        }

        /// Gets or sets whether to compute a pointer-relative preview offset automatically.
        pub fn use_pointer_relative_preview_offset_property() -> StyledProperty<bool> {
            FerroProperty::register::<ManagedContextDragBehavior, _>("UsePointerRelativePreviewOffset", true)
        }

        /// Gets or sets the preview window opacity.
        pub fn preview_opacity_property() -> StyledProperty<f64> {
            FerroProperty::register::<ManagedContextDragBehavior, _>("PreviewOpacity", 0.65)
        }
    }
}

/// The routes of the pointer handlers: direct, tunnel and bubble.
const ALL_ROUTES: RoutingStrategies =
    RoutingStrategies::DIRECT.union(RoutingStrategies::TUNNEL).union(RoutingStrategies::BUBBLE);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ManagedContextDragBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let ao = this.associated_object_as::<Control>();
            *this.top_level.borrow_mut() = ao.as_ref().and_then(|ao| {
                let visual: &Visual = ao;
                TopLevel::get_top_level(Some(visual))
            });
            let Some(ao) = ao else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &ao,
                InputElement::pointer_pressed_event(),
                move |_: &Interactive, e: &PointerPressedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_pressed(e);
                    }
                },
                ALL_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &ao,
                InputElement::pointer_released_event(),
                move |_: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_released(e);
                    }
                },
                ALL_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &ao,
                InputElement::pointer_moved_event(),
                move |_: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_moved(e);
                    }
                },
                ALL_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &ao,
                InputElement::pointer_capture_lost_event(),
                move |_: &Interactive, e: &PointerCaptureLostEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_capture_lost(e);
                    }
                },
                ALL_ROUTES,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.handlers.clear();
            this.detach_top_level_handlers();
            *this.top_level.borrow_mut() = None;
        }
    }
}

/// The effects of a managed drag, from the keyboard modifiers of the press that started it.
fn get_desired_effects(trigger_event: &PointerEventArgs) -> DragDropEffects {
    let key_modifiers = trigger_event.key_modifiers();
    if key_modifiers.contains(KeyModifiers::ALT) {
        DragDropEffects::LINK
    } else if key_modifiers.contains(KeyModifiers::CONTROL) {
        DragDropEffects::COPY
    } else {
        DragDropEffects::MOVE
    }
}

/// The status text of effects (not used by the reference either).
#[allow(dead_code)]
fn effects_to_status(effects: DragDropEffects) -> String {
    if effects == DragDropEffects::MOVE {
        "Move".to_string()
    } else if effects == DragDropEffects::COPY {
        "Copy".to_string()
    } else if effects == DragDropEffects::LINK {
        "Link".to_string()
    } else if effects == DragDropEffects::NONE {
        String::new()
    } else {
        format!("{effects:?}")
    }
}

/// Whether the element pressed is selected: it, its parent or a logical ancestor that is
/// selectable (`ISelectable`).
fn is_selected(control: &Control) -> bool {
    fn of(object: &FerroObject) -> Option<bool> {
        as_selectable(object).map(|selectable| selectable.is_selected())
    }

    of(control)
        .or_else(|| control.parent().and_then(|parent| of(&parent)))
        .or_else(|| {
            let element: &StyledElement = control;
            element.get_logical_ancestors().find_map(|ancestor| of(&ancestor))
        })
        .unwrap_or(false)
}

impl ManagedContextDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            drag_start_point: Cell::new(Point::default()),
            trigger_event: RefCell::new(None),
            lock: Cell::new(false),
            captured: Cell::new(false),
            top_level: RefCell::new(None),
            internal_dragging: Cell::new(false),
            internal_drag_tcs: RefCell::new(None),
            calculated_preview_offset: Cell::new(None),
            handlers: RoutedHandlers::new(),
            top_level_handlers: RoutedHandlers::new(),
            detached_subscription: RefCell::new(None),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the context value used as a drag payload when the drag starts.
    pub fn context(&self) -> Option<BoxedValue> {
        self.get_value(Self::context_property())
    }

    pub fn set_context(&self, value: Option<BoxedValue>) {
        self.set_value(Self::context_property(), value)
    }

    /// Gets or sets the template used to render the drag preview.
    pub fn preview_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::preview_template_property())
    }

    pub fn set_preview_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::preview_template_property(), value)
    }

    /// Gets or sets the minimal horizontal distance required to start dragging.
    pub fn horizontal_drag_threshold(&self) -> f64 {
        self.get_value(Self::horizontal_drag_threshold_property())
    }

    pub fn set_horizontal_drag_threshold(&self, value: f64) {
        self.set_value(Self::horizontal_drag_threshold_property(), value)
    }

    /// Gets or sets the minimal vertical distance required to start dragging.
    pub fn vertical_drag_threshold(&self) -> f64 {
        self.get_value(Self::vertical_drag_threshold_property())
    }

    pub fn set_vertical_drag_threshold(&self, value: f64) {
        self.set_value(Self::vertical_drag_threshold_property(), value)
    }

    /// Gets or sets a fixed logical offset added to the preview position.
    pub fn preview_offset(&self) -> Point {
        self.get_value(Self::preview_offset_property())
    }

    pub fn set_preview_offset(&self, value: Point) {
        self.set_value(Self::preview_offset_property(), value)
    }

    /// Gets or sets the data format name used to identify the managed payload.
    pub fn data_format(&self) -> String {
        self.get_value(Self::data_format_property())
    }

    pub fn set_data_format(&self, value: String) {
        self.set_value(Self::data_format_property(), value)
    }

    /// Gets or sets whether to compute a pointer-relative preview offset automatically.
    pub fn use_pointer_relative_preview_offset(&self) -> bool {
        self.get_value(Self::use_pointer_relative_preview_offset_property())
    }

    pub fn set_use_pointer_relative_preview_offset(&self, value: bool) {
        self.set_value(Self::use_pointer_relative_preview_offset_property(), value)
    }

    /// Gets or sets the preview window opacity.
    pub fn preview_opacity(&self) -> f64 {
        self.get_value(Self::preview_opacity_property())
    }

    pub fn set_preview_opacity(&self, value: f64) {
        self.set_value(Self::preview_opacity_property(), value)
    }

    /// Whether a managed drag of this behavior is in progress (not in the reference: for the
    /// tests).
    #[allow(dead_code)]
    pub(crate) fn is_internal_dragging(&self) -> bool {
        self.internal_dragging.get()
    }

    fn attach_top_level_handlers(&self) {
        let top_level = self.top_level.borrow().clone();
        let Some(top_level) = top_level else {
            return;
        };

        let behavior = self.to_ref().downgrade();
        self.top_level_handlers.add(
            &top_level,
            InputElement::pointer_moved_event(),
            move |_: &Interactive, e: &PointerEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_top_level_pointer_moved(e);
                }
            },
            ALL_ROUTES,
            true,
        );
        let behavior = self.to_ref().downgrade();
        self.top_level_handlers.add(
            &top_level,
            InputElement::pointer_released_event(),
            move |_: &Interactive, e: &PointerReleasedEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_top_level_pointer_released(e);
                }
            },
            ALL_ROUTES,
            true,
        );
        let behavior = self.to_ref().downgrade();
        self.top_level_handlers.add(
            &top_level,
            InputElement::pointer_capture_lost_event(),
            move |_: &Interactive, e: &PointerCaptureLostEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_top_level_pointer_capture_lost(e);
                }
            },
            ALL_ROUTES,
            true,
        );
    }

    fn detach_top_level_handlers(&self) {
        self.top_level_handlers.clear();
    }

    /// The offset of the preview: the pointer-relative one when it is enabled and calculated, the
    /// fixed one otherwise.
    fn effective_preview_offset(&self) -> Point {
        match self.calculated_preview_offset.get() {
            Some(calculated) if self.use_pointer_relative_preview_offset() => calculated,
            _ => self.preview_offset(),
        }
    }

    /// Completes the managed drag (`_internalDragTcs?.TrySetResult(true)`).
    fn complete_internal_drag(&self) {
        let tcs = self.internal_drag_tcs.borrow().clone();
        if let Some(tcs) = tcs {
            tcs.try_set_result(true);
        }
    }

    async fn start_internal_drag_async(this: Ref<Self>, trigger_event: PointerPressedEventArgs, value: BoxedValue) {
        let effects = get_desired_effects(&trigger_event);
        let associated_object = this.associated_object_as::<Control>();
        let tl = this.top_level.borrow().clone().or_else(|| {
            associated_object.as_ref().and_then(|ao| {
                let visual: &Visual = ao;
                TopLevel::get_top_level(Some(visual))
            })
        });
        let Some(tl) = tl else {
            return;
        };

        let tl_visual: &Visual = &tl;
        let client = trigger_event.get_position(Some(tl_visual));
        let preview_offset = this.effective_preview_offset();

        DragPreviewService::show(
            Some(value.clone()),
            this.preview_template(),
            &tl,
            client,
            preview_offset,
            this.preview_opacity(),
        );

        // try
        IS_DRAGGING.set(true);
        this.internal_dragging.set(true);
        let tcs = TaskCompletionSource::<bool>::new();
        *this.internal_drag_tcs.borrow_mut() = Some(tcs.clone());
        if let Some(ao) = &associated_object {
            let input_element: Ref<InputElement> = ao.clone().upcast();
            trigger_event.pointer().capture(Some(&input_element));
        }
        this.attach_top_level_handlers();
        ManagedDragDropService::instance().begin(&tl, Some(value), Some(this.data_format()), effects, client);
        if let Some(ao) = &associated_object {
            let behavior = this.downgrade();
            *this.detached_subscription.borrow_mut() = Some(ao.detached_from_visual_tree(move |_| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.associated_object_detached_from_visual_tree();
                }
            }));
        }
        let _ = tcs.task().await;

        // finally
        ManagedDragDropService::instance().end();
        let detached_subscription = this.detached_subscription.borrow_mut().take();
        if let Some(detached_subscription) = detached_subscription {
            detached_subscription.dispose();
        }
        this.detach_top_level_handlers();
        trigger_event.pointer().capture(None);
        DragPreviewService::hide();
        this.internal_dragging.set(false);
        *this.internal_drag_tcs.borrow_mut() = None;
        IS_DRAGGING.set(false);
        this.calculated_preview_offset.set(None);
    }

    fn associated_object_detached_from_visual_tree(&self) {
        self.internal_dragging.set(false);
        self.complete_internal_drag();
    }

    fn on_top_level_pointer_moved(&self, e: &PointerEventArgs) {
        let top_level = self.top_level.borrow().clone();
        let Some(top_level) = top_level.filter(|_| self.internal_dragging.get()) else {
            return;
        };

        let top_level_visual: &Visual = &top_level;
        let client = e.get_position(Some(top_level_visual));
        let preview_offset = self.effective_preview_offset();
        DragPreviewService::move_(&top_level, client, preview_offset);
        ManagedDragDropService::instance().move_(client);
    }

    fn on_top_level_pointer_released(&self, _e: &PointerReleasedEventArgs) {
        if !self.internal_dragging.get() {
            return;
        }
        self.internal_dragging.set(false);
        self.complete_internal_drag();
    }

    fn on_top_level_pointer_capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        if !self.internal_dragging.get() {
            return;
        }
        self.internal_dragging.set(false);
        self.complete_internal_drag();
    }

    fn release(&self) {
        *self.trigger_event.borrow_mut() = None;
        self.lock.set(false);
        self.calculated_preview_offset.set(None);
    }

    fn on_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let Some(ao) = self.associated_object_as::<Control>() else {
            return;
        };
        let properties = e.properties();
        if properties.is_left_button_pressed && self.is_enabled() {
            if let Some(control) = e.source().and_then(|source| source.cast::<Control>()) {
                if same_data_context(ao.data_context().as_ref(), control.data_context().as_ref()) {
                    if is_selected(&control) {
                        e.set_handled(true);
                    }

                    self.drag_start_point.set(e.get_position(None));
                    *self.trigger_event.borrow_mut() = Some(e.clone());
                    self.lock.set(true);
                    self.captured.set(true);

                    // Compute the cursor-relative preview offset if enabled in TopLevel
                    // coordinates using AssociatedObject top-left
                    if self.use_pointer_relative_preview_offset() {
                        let ao_visual: &Visual = &ao;
                        self.calculated_preview_offset.set(Some(-e.get_position(Some(ao_visual))));
                    }
                }
            }
        }
        e.set_handled(false);
    }

    fn on_pointer_released(&self, e: &PointerReleasedEventArgs) {
        if self.captured.get() {
            if e.initial_press_mouse_button() == MouseButton::Left && self.trigger_event.borrow().is_some() {
                self.release();
            }
            self.captured.set(false);
        }
    }

    fn on_pointer_moved(&self, e: &PointerEventArgs) {
        if self.associated_object_as::<Control>().is_none() {
            return;
        }
        let trigger_event = self.trigger_event.borrow().clone();
        let Some(trigger_event) = trigger_event.filter(|_| self.captured.get() && self.is_enabled()) else {
            return;
        };

        if IS_DRAGGING.get() {
            return;
        }

        let properties = e.properties();
        if !properties.is_left_button_pressed {
            return;
        }

        let point = e.get_position(None);
        if exceeds_drag_threshold(
            self.drag_start_point.get(),
            point,
            self.horizontal_drag_threshold(),
            self.vertical_drag_threshold(),
        ) {
            if self.lock.get() {
                self.lock.set(false);
            } else {
                return;
            }

            let context = self.context().or_else(|| {
                self.associated_object_as::<Control>().and_then(|associated_object| associated_object.data_context())
            });
            let Some(context) = context else {
                return;
            };

            // `async void` of the reference: the drag runs as a task of the dispatcher, started
            // now.
            let this = self.to_ref();
            drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
                Self::start_internal_drag_async(this.clone(), trigger_event, context).await;
                *this.trigger_event.borrow_mut() = None;
            }));
        }
    }

    fn on_capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        self.release();
        self.captured.set(false);
        if self.internal_dragging.get() {
            self.internal_dragging.set(false);
            self.complete_internal_drag();
        }
    }
}
