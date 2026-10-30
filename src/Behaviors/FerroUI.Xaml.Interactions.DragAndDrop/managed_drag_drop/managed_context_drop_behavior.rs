//! Port of `ManagedDragDrop/ManagedContextDropBehavior.cs`.

use super::managed_data_transfer::ManagedPayloadDataTransfer;
use crate::values::boxed_object;
use crate::{IDropHandler, ManagedDragDropService};
use ferroui_base::input::{DataTransfer, DragDrop, DragEventArgs, IDataTransfer, KeyModifiers};
use ferroui_base::interactivity::{Interactive, RoutedEvent, RoutedEventArgs};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty,
    PixelPoint, Point, Ref, StaticType, StyledElementImpl, StyledProperty, Visual, WeakRef,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Arguments used by managed drop handlers to describe the drop context.
#[derive(Clone, Default)]
pub struct ManagedContextDropArgs {
    /// Gets or sets the payload transported by the managed drag operation.
    pub payload: Option<BoxedValue>,
    /// Gets or sets the data format of the [`payload`](Self::payload).
    pub data_format: Option<String>,
    /// Gets or sets the drag-drop effects requested by the initiator.
    pub effects: ferroui_base::input::DragDropEffects,
    /// Gets or sets the top-level that originated the drag.
    pub origin_top_level: Option<Ref<TopLevel>>,
    /// Gets or sets the pointer position local to the drop target control.
    pub position: Point,
    /// Gets or sets the pointer position in screen pixel coordinates.
    pub screen_position: PixelPoint,
}

const DROP_TARGET_PSEUDO_CLASS: &str = "droptarget";
const DRAG_OVER_PSEUDO_CLASS: &str = "dragover";

/// Drop target behavior that integrates with [`ManagedDragDropService`].
///
/// It mirrors the semantics of [`ContextDropBehavior`](crate::ContextDropBehavior) but works
/// entirely in-process: while a managed drag of the [`accept_data_format`](Self::accept_data_format)
/// is in progress the target gets the pseudo class `droptarget` when its handler validates the
/// payload, and `dragover` (and the [`over_class`](Self::over_class)) while the pointer is over it.
#[repr(C)]
pub struct ManagedContextDropBehavior {
    base: StyledElementBehaviorOfT,
    is_over: Cell<bool>,
    // tracks whether pseudo class is applied
    wants_drop: Cell<bool>,
    drag_drop_subscription: RefCell<Option<DragDropServiceSubscription>>,
}

ferro_class!(ManagedContextDropBehavior: StyledElementBehaviorOfT);
ferro_class_info!(ManagedContextDropBehavior {
    new: ManagedContextDropBehavior::new,
    markup: {
        attributes: [
            PseudoClasses("wants-drop", "dragover"),
        ],
    },
});
ferroui_base::ferro_impl_classes!(ManagedContextDropBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ManagedContextDropBehavior {
        /// Identifies the `AcceptDataFormat` property.
        pub fn accept_data_format_property() -> StyledProperty<String> {
            FerroProperty::register::<ManagedContextDropBehavior, _>("AcceptDataFormat", "Context".to_string())
        }

        /// Identifies the `AllowDrop` property.
        pub fn allow_drop_property() -> StyledProperty<bool> {
            FerroProperty::register::<ManagedContextDropBehavior, _>("AllowDrop", true)
        }

        /// Identifies the `OverClass` property.
        pub fn over_class_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ManagedContextDropBehavior, _>("OverClass", None)
        }

        /// Identifies the `Context` property.
        pub fn context_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ManagedContextDropBehavior, _>("Context", None)
        }

        /// Identifies the `Handler` property.
        pub fn handler_property() -> StyledProperty<Option<Rc<dyn IDropHandler>>> {
            FerroProperty::register::<ManagedContextDropBehavior, _>("Handler", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ManagedContextDropBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            let mut subscription = this.drag_drop_subscription.borrow_mut();
            if subscription.is_none() {
                *subscription = Some(DragDropServiceSubscription::new(&this.to_ref()));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.detach_managed_drag_events();

            Self::parent_on_detached_from_visual_tree(this);
        }

        fn on_detaching(this: &Self) {
            this.detach_managed_drag_events();

            Self::parent_on_detaching(this);
        }
    }
}

/// The position of the managed drag in the coordinates of `target`
/// (`tl.TranslatePoint(tl.PointToClient(svc.ScreenPosition), target) ?? default`).
fn local_position(top_level: &TopLevel, target: &Control, service: &ManagedDragDropService) -> Point {
    let p_top = top_level.point_to_client(service.screen_position());
    let target_visual: &Visual = target;
    top_level.translate_point(p_top, target_visual).unwrap_or_default()
}

/// The top level of `target`.
fn top_level_of(target: &Control) -> Option<Ref<TopLevel>> {
    let visual: &Visual = target;
    TopLevel::get_top_level(Some(visual))
}

impl ManagedContextDropBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            is_over: Cell::new(false),
            wants_drop: Cell::new(false),
            drag_drop_subscription: RefCell::new(None),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the accepted managed data format.
    pub fn accept_data_format(&self) -> String {
        self.get_value(Self::accept_data_format_property())
    }

    pub fn set_accept_data_format(&self, value: String) {
        self.set_value(Self::accept_data_format_property(), value)
    }

    /// Gets or sets a value indicating whether drop is allowed.
    pub fn allow_drop(&self) -> bool {
        self.get_value(Self::allow_drop_property())
    }

    pub fn set_allow_drop(&self, value: bool) {
        self.set_value(Self::allow_drop_property(), value)
    }

    /// Gets or sets an optional class applied while the pointer is over the target during drag.
    pub fn over_class(&self) -> Option<String> {
        self.get_value(Self::over_class_property())
    }

    pub fn set_over_class(&self, value: Option<String>) {
        self.set_value(Self::over_class_property(), value)
    }

    /// Gets or sets the context value supplied to the drop handler.
    pub fn context(&self) -> Option<BoxedValue> {
        self.get_value(Self::context_property())
    }

    pub fn set_context(&self, value: Option<BoxedValue>) {
        self.set_value(Self::context_property(), value)
    }

    /// Gets or sets the handler that receives managed drag-drop notifications.
    pub fn handler(&self) -> Option<Rc<dyn IDropHandler>> {
        self.get_value(Self::handler_property())
    }

    pub fn set_handler(&self, value: Option<Rc<dyn IDropHandler>>) {
        self.set_value(Self::handler_property(), value)
    }

    fn detach_managed_drag_events(&self) {
        let subscription = self.drag_drop_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.update_over(false);
        self.update_wants_drop(false);
    }

    /// Whether the service drags a payload of the accepted format.
    fn accepts(&self, service: &ManagedDragDropService) -> bool {
        service.is_dragging() && service.data_format().as_deref() == Some(self.accept_data_format().as_str())
    }

    /// The context of the target: the context of the behavior, or the data context of its control.
    fn target_context(&self, target: &Control) -> Option<BoxedValue> {
        self.context().or_else(|| target.data_context())
    }

    fn on_drag_started(&self) {
        self.update_over(false);
        let target = self.associated_object_as::<Control>();
        let Some(target) = target.filter(|target| target.is_effectively_visible()) else {
            self.update_wants_drop(false);
            return;
        };
        let service = ManagedDragDropService::instance();
        let handler = self.handler();
        let mut compatible = self.allow_drop() && handler.is_some() && self.accepts(&service);
        if compatible {
            let local = top_level_of(&target).map(|tl| local_position(&tl, &target, &service)).unwrap_or_default();
            compatible = match self.create_drag_event_args(DragDrop::drag_enter_event(), local, &service) {
                None => false,
                Some(e) => {
                    let sender = boxed_object(&target);
                    let target_context = self.target_context(&target);
                    // The reference treats a failing validation as invalid.
                    handler.is_some_and(|handler| {
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            handler.validate(
                                Some(&sender),
                                &e,
                                service.payload().as_ref(),
                                target_context.as_ref(),
                                None,
                            )
                        }))
                        .unwrap_or(false)
                    })
                }
            };
        }
        self.update_wants_drop(compatible);
        self.invoke_handler_enter();
    }

    fn on_drag_moved(&self) {
        let target = self.associated_object_as::<Control>();
        let Some(target) = target.filter(|_| self.allow_drop()) else {
            return;
        };
        if !target.is_effectively_visible() {
            if self.is_over.get() {
                self.update_over(false);
                self.invoke_handler_leave();
            }
            self.update_wants_drop(false);
            return;
        }

        let service = ManagedDragDropService::instance();
        if !self.accepts(&service) {
            if self.is_over.get() {
                self.update_over(false);
                self.invoke_handler_leave();
            }
            return;
        }

        let Some(tl) = top_level_of(&target) else {
            if self.is_over.get() {
                self.update_over(false);
                self.invoke_handler_leave();
            }
            return;
        };

        let p_local = local_position(&tl, &target, &service);
        let bounds = target.bounds();
        let over = p_local.x >= 0.0 && p_local.y >= 0.0 && p_local.x <= bounds.width && p_local.y <= bounds.height;
        if over != self.is_over.get() {
            self.update_over(over);
            if !over {
                self.invoke_handler_leave();
            }
        }

        if over {
            // Commands removed; rely solely on handler.
            self.invoke_handler_over(p_local, &service);
        }
    }

    fn on_drag_ended(&self) {
        let Some(target) = self.associated_object_as::<Control>() else {
            return;
        };
        if !target.is_effectively_visible() {
            if self.is_over.get() {
                self.invoke_handler_leave();
            }
            self.update_over(false);
            self.update_wants_drop(false);
            return;
        }

        // try
        let service = ManagedDragDropService::instance();
        if self.is_over.get() && self.allow_drop() {
            // Commands removed; rely solely on handler.
            self.invoke_handler_drop(&service);
        }

        // finally
        if self.is_over.get() {
            self.invoke_handler_leave();
        }
        self.update_over(false);
        self.update_wants_drop(false);
    }

    fn update_over(&self, over: bool) {
        self.is_over.set(over);
        let Some(target) = self.associated_object_as::<Control>() else {
            return;
        };
        if let Some(cls) = self.over_class().filter(|cls| !cls.trim().is_empty()) {
            target.classes().set(&cls, over);
        }
        // Apply/remove dragover pseudo class
        if over {
            target.pseudo_classes().add_pseudo(DRAG_OVER_PSEUDO_CLASS);
        } else {
            target.pseudo_classes().remove_pseudo(DRAG_OVER_PSEUDO_CLASS);
        }
    }

    fn update_wants_drop(&self, wants: bool) {
        if self.wants_drop.get() == wants {
            return;
        }
        self.wants_drop.set(wants);
        let Some(target) = self.associated_object_as::<Control>() else {
            return;
        };
        if wants {
            target.pseudo_classes().add_pseudo(DROP_TARGET_PSEUDO_CLASS);
        } else {
            target.pseudo_classes().remove_pseudo(DROP_TARGET_PSEUDO_CLASS);
        }
    }

    fn create_drag_event_args(
        &self,
        routed_event: &RoutedEvent<DragEventArgs>,
        local_position: Point,
        service: &ManagedDragDropService,
    ) -> Option<DragEventArgs> {
        // The reference returns no args when creating them fails: the format of the payload is
        // checked here instead (the only failure of `CreateDataTransfer`).
        let data_format = service.data_format();
        if service.payload().is_some() && data_format.as_deref().is_some_and(|format| format.trim().is_empty()) {
            return None;
        }
        let data = Self::create_data_transfer(service.payload(), data_format.as_deref());
        // Use Interactive (base for Control) as required by DragEventArgs
        let target: Ref<Interactive> = self.associated_object_as::<Control>()?.upcast();
        let e = DragEventArgs::new(Some(routed_event), data, target, local_position, KeyModifiers::NONE);
        // Propagate current requested effects so handlers can decide behavior
        e.set_drag_effects(service.effects());
        Some(e)
    }

    /// The data transfer of a managed payload: the payload as it is, in its format, or an empty
    /// data transfer without one.
    ///
    /// # Panics
    /// Panics if there is a payload and its format is empty or white space.
    pub(crate) fn create_data_transfer(payload: Option<BoxedValue>, format: Option<&str>) -> Rc<dyn IDataTransfer> {
        if let (Some(payload), Some(format)) = (payload, format) {
            // The public application formats of the framework are of text and bytes only. The
            // managed in-process payload is kept in a data transfer of its own so that callers can
            // still recover the original object instead of a stringified surrogate.
            return ManagedPayloadDataTransfer::create(format, payload);
        }

        DataTransfer::new()
    }

    fn invoke_handler_enter(&self) {
        let handler = self.handler();
        let target = self.associated_object_as::<Control>();
        let service = ManagedDragDropService::instance();
        let (Some(handler), Some(target)) = (handler, target) else {
            return;
        };
        if !target.is_effectively_visible() || !self.allow_drop() || !self.accepts(&service) {
            return;
        }
        let Some(tl) = top_level_of(&target) else {
            return;
        };
        let p_local = local_position(&tl, &target, &service);
        if let Some(e) = self.create_drag_event_args(DragDrop::drag_enter_event(), p_local, &service) {
            let target_context = self.target_context(&target);
            handler.enter(Some(&boxed_object(&target)), &e, service.payload().as_ref(), target_context.as_ref());
        }
    }

    fn invoke_handler_over(&self, local_position: Point, service: &ManagedDragDropService) {
        let handler = self.handler();
        let target = self.associated_object_as::<Control>();
        let (Some(handler), Some(target)) = (handler, target) else {
            return;
        };
        if !target.is_effectively_visible() {
            return;
        }
        if let Some(e) = self.create_drag_event_args(DragDrop::drag_over_event(), local_position, service) {
            let target_context = self.target_context(&target);
            handler.over(Some(&boxed_object(&target)), &e, service.payload().as_ref(), target_context.as_ref());
        }
    }

    fn invoke_handler_drop(&self, service: &ManagedDragDropService) {
        let handler = self.handler();
        let target = self.associated_object_as::<Control>();
        let (Some(handler), Some(target)) = (handler, target) else {
            return;
        };
        if !target.is_effectively_visible() || !self.is_over.get() {
            return;
        }
        let Some(tl) = top_level_of(&target) else {
            return;
        };
        let p_local = local_position(&tl, &target, service);
        if let Some(e) = self.create_drag_event_args(DragDrop::drop_event(), p_local, service) {
            let target_context = self.target_context(&target);
            IDropHandler::drop(
                &*handler,
                Some(&boxed_object(&target)),
                &e,
                service.payload().as_ref(),
                target_context.as_ref(),
            );
        }
    }

    fn invoke_handler_leave(&self) {
        let handler = self.handler();
        let target = self.associated_object_as::<Control>();
        let (Some(handler), Some(target)) = (handler, target) else {
            return;
        };
        handler.leave(Some(&boxed_object(&target)), &RoutedEventArgs::new());
    }
}

/// The subscription of a behavior to the events of the service; it holds the behavior weakly and
/// is disposed when the behavior is detached (or dropped).
struct DragDropServiceSubscription {
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl DragDropServiceSubscription {
    fn new(owner: &Ref<ManagedContextDropBehavior>) -> Self {
        let service = ManagedDragDropService::instance();

        let started: WeakRef<ManagedContextDropBehavior> = owner.downgrade();
        let moved = started.clone();
        let ended = started.clone();
        let subscriptions = vec![
            service.drag_started(move || {
                if let Some(owner) = started.upgrade() {
                    owner.on_drag_started();
                }
            }),
            service.drag_moved(move || {
                if let Some(owner) = moved.upgrade() {
                    owner.on_drag_moved();
                }
            }),
            service.drag_ended(move || {
                if let Some(owner) = ended.upgrade() {
                    owner.on_drag_ended();
                }
            }),
        ];

        Self { subscriptions: RefCell::new(subscriptions) }
    }

    fn dispose(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}

impl Drop for DragDropServiceSubscription {
    fn drop(&mut self) {
        self.dispose();
    }
}
