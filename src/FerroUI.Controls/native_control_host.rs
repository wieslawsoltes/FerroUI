use crate::platform::{
    INativeControlHostControlTopLevelAttachment, INativeControlHostImpl, IPlatformHandle,
};
use crate::presentation_source::PresentationSource;
use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, LayoutableImpl};
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroPropertyChangedEventArgs,
    Point, Rect, Ref, StyledElementImpl, Visual, VisualImpl, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Hosts a native control of the platform inside the visual tree: the
/// native control is created for the top-level the host is attached to and
/// follows the bounds and the visibility of the host.
#[repr(C)]
pub struct NativeControlHost {
    base: Control,
    current_root: RefCell<Option<Rc<PresentationSource>>>,
    current_host: RefCell<Option<Rc<dyn INativeControlHostImpl>>>,
    attachment: RefCell<Option<Rc<dyn INativeControlHostControlTopLevelAttachment>>>,
    native_control_handle: RefCell<Option<Rc<dyn IPlatformHandle>>>,
    queued_for_destruction: Cell<bool>,
    queued_for_move_resize: Cell<bool>,
    property_changed_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    /// The subscriptions to the changes of the render transforms of the
    /// host and of its visual ancestors (not upstream: see
    /// `subscribe_render_transforms`).
    render_transform_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    native_control_handle_changed: HandlerList<dyn Fn()>,
}

ferro_class! {
    NativeControlHost: Control, virtuals NativeControlHostImpl: ControlImpl {
        /// Creates the native control as a child of `parent`. The default
        /// asks the native control host of the platform for its default
        /// child.
        fn create_native_control_core(this, parent: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>;
        /// Destroys the native control. The default destroys a handle that
        /// can be destroyed.
        fn destroy_native_control_core(this, control: Rc<dyn IPlatformHandle>);
    }
}
ferro_class_info!(NativeControlHost { new: NativeControlHost::new });

ferro_impl_classes!(
    NativeControlHost: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl
);

impl VisualImpl for NativeControlHost {
    // As upstream, the base implementation is not called.
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        *this.current_root.borrow_mut() = PresentationSource::from_root_visual(e.root_visual());
        let mut visual = Some(this.to_ref().upcast::<Visual>());
        while let Some(current) = visual {
            let weak = this.to_ref().downgrade();
            let subscription = current.property_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.property_changed_handler(e);
                }
            });
            this.property_changed_subscriptions.borrow_mut().push(subscription);

            visual = current.get_visual_parent();
        }
        this.subscribe_render_transforms();

        this.update_host();
    }

    // As upstream, the base implementation is not called.
    fn on_detached_from_visual_tree(this: &Self, _e: &VisualTreeAttachmentEventArgs) {
        *this.current_root.borrow_mut() = None;
        let subscriptions = std::mem::take(&mut *this.property_changed_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        this.unsubscribe_render_transforms();
        this.update_host();
    }
}

impl NativeControlHostImpl for NativeControlHost {
    fn create_native_control_core(this: &Self, parent: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle> {
        let Some(current_host) = this.current_host.borrow().clone() else {
            panic!("Operation is not valid due to the current state of the object.");
        };
        current_host.create_default_child(parent)
    }

    fn destroy_native_control_core(_this: &Self, control: Rc<dyn IPlatformHandle>) {
        if let Some(native_control_host_destroyable_control_handle) =
            control.as_native_control_host_destroyable_control_handle()
        {
            native_control_host_destroyable_control_handle.destroy();
        }
    }
}

impl ControlImpl for NativeControlHost {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::NativeControlHostPeer::new(this).upcast()
    }
}

fn same_host(a: Option<&Rc<dyn INativeControlHostImpl>>, b: Option<&Rc<dyn INativeControlHostImpl>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        (None, None) => true,
        _ => false,
    }
}

fn same_handle(a: Option<&Rc<dyn IPlatformHandle>>, b: Option<&Rc<dyn IPlatformHandle>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        (None, None) => true,
        _ => false,
    }
}

impl NativeControlHost {
    fn static_constructor() {
        Visual::flow_direction_property()
            .changed()
            .add_class_handler::<NativeControlHost>(Self::on_flow_direction_changed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            current_root: RefCell::new(None),
            current_host: RefCell::new(None),
            attachment: RefCell::new(None),
            native_control_handle: RefCell::new(None),
            queued_for_destruction: Cell::new(false),
            queued_for_move_resize: Cell::new(false),
            property_changed_subscriptions: RefCell::new(Vec::new()),
            render_transform_subscriptions: RefCell::new(Vec::new()),
            native_control_handle_changed: HandlerList::new(),
        }
    }

    /// Creates a native control host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The handle of the native control, once it has been created
    /// (internal upstream).
    pub fn native_control_handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        self.native_control_handle.borrow().clone()
    }

    fn set_native_control_handle(&self, value: Option<Rc<dyn IPlatformHandle>>) {
        let changed = !same_handle(self.native_control_handle.borrow().as_ref(), value.as_ref());
        if changed {
            let old = self.native_control_handle.replace(value);
            drop(old);
            let handlers = self.native_control_handle_changed.snapshot();
            for (_, handler) in handlers.iter() {
                handler();
            }
        }
    }

    /// Raised when the handle of the native control changes (internal
    /// upstream).
    pub fn native_control_handle_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.native_control_handle_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.native_control_handle_changed.remove(token);
            }
        })
    }

    fn on_flow_direction_changed(native_control_host: &NativeControlHost, _e: &FerroPropertyChangedEventArgs<'_>) {
        native_control_host.try_update_native_control_position();
    }

    fn property_changed_handler(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if !e.is_effective_value_change() {
            return;
        }
        if e.property() == Visual::bounds_property().as_property()
            || e.property() == Visual::is_visible_property().as_property()
        {
            self.enqueue_for_move_resize();
        } else if e.property() == Visual::render_transform_property().as_property() {
            // Deviation (DEVIATIONS.md, Native control host): see
            // `subscribe_render_transforms`.
            self.subscribe_render_transforms();
            self.enqueue_for_move_resize();
        } else if e.property() == Visual::render_transform_origin_property().as_property() {
            self.enqueue_for_move_resize();
        }
    }

    /// Subscribes to the changes of the render transforms of the host and
    /// of its visual ancestors, in place of the previous subscriptions.
    ///
    /// Deviation (DEVIATIONS.md, Native control host): upstream
    /// (`NativeControlHost.PropertyChangedHandler`) moves the native control
    /// only when the bounds or the visibility of the host or of an ancestor
    /// change. The position of the native control is the position of the
    /// host in the root, which the render transforms are part of, so a
    /// host whose first arrange falls into a transition that slides its
    /// page in stayed where the first frame of the transition had it. The
    /// host is moved when a render transform of the chain is set, replaced
    /// or removed, and when a mutable one changes in place (an animation);
    /// a host under no transform that changes is not looked at.
    fn subscribe_render_transforms(&self) {
        self.unsubscribe_render_transforms();
        let mut subscriptions = Vec::new();
        let mut visual = Some(self.to_ref().upcast::<Visual>());
        while let Some(current) = visual {
            let render_transform = current.render_transform();
            if let Some(mutable_transform) =
                render_transform.as_ref().and_then(|render_transform| render_transform.as_mutable_transform())
            {
                let weak = self.to_ref().downgrade();
                subscriptions.push(mutable_transform.changed(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.enqueue_for_move_resize();
                    }
                })));
            }

            visual = current.get_visual_parent();
        }
        *self.render_transform_subscriptions.borrow_mut() = subscriptions;
    }

    fn unsubscribe_render_transforms(&self) {
        let subscriptions = std::mem::take(&mut *self.render_transform_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }

    fn update_host(&self) {
        self.queued_for_move_resize.set(false);
        let current_host = self
            .current_root
            .borrow()
            .as_ref()
            .and_then(|current_root| current_root.platform_impl())
            .and_then(|platform_impl| {
                let provider: &dyn IOptionalFeatureProvider = &*platform_impl;
                provider.try_get::<dyn INativeControlHostImpl>()
            });
        *self.current_host.borrow_mut() = current_host.clone();

        if let Some(current_host) = &current_host {
            // If there is an existing attachment, ensure that we are attached to the proper host or destroy the attachment
            let attachment = self.attachment.borrow().clone();
            if let Some(attachment) = attachment {
                if !same_host(attachment.attached_to().as_ref(), Some(current_host)) {
                    if attachment.is_compatible_with(&**current_host) {
                        attachment.set_attached_to(Some(current_host.clone()));
                    } else {
                        attachment.dispose();
                        *self.attachment.borrow_mut() = None;
                    }
                }
            }

            // If there is no attachment, but the control exists,
            // attempt to attach to the current toplevel or destroy the control if it's incompatible
            // The borrow of the attachment must end before the branch: both
            // arms set the attachment.
            let has_attachment = self.attachment.borrow().is_some();
            let native_control_handle = self.native_control_handle();
            if let (false, Some(native_control_handle)) = (has_attachment, native_control_handle) {
                if current_host.is_compatible_with(&*native_control_handle) {
                    let attachment = current_host.create_new_attachment(native_control_handle);
                    *self.attachment.borrow_mut() = Some(attachment);
                } else {
                    self.destroy_native_control();
                }
            }

            // There is no control handle an no attachment, create both
            if self.native_control_handle().is_none() {
                let weak = self.to_ref().downgrade();
                let attachment = current_host.create_new_attachment_with(Rc::new(move |parent| {
                    let this = weak.upgrade().expect("the native control host is alive while its control is created");
                    let handle = this.create_native_control_core(parent);
                    this.set_native_control_handle(Some(handle.clone()));
                    handle
                }));
                *self.attachment.borrow_mut() = Some(attachment);
            }
        } else {
            // Immediately detach the control from the current toplevel if there is an existing attachment
            let attachment = self.attachment.borrow().clone();
            if let Some(attachment) = attachment {
                attachment.set_attached_to(None);
            }

            // Don't destroy the control immediately, it might be just being reparented to another TopLevel
            if self.native_control_handle().is_some() && !self.queued_for_destruction.get() {
                self.queued_for_destruction.set(true);
                let this = self.to_ref();
                Dispatcher::ui_thread().post_local(move || this.check_destruction(), DispatcherPriority::BACKGROUND);
            }
        }

        let attached_to = self.attachment.borrow().as_ref().and_then(|attachment| attachment.attached_to());
        if !same_host(attached_to.as_ref(), current_host.as_ref()) {
            return;
        }

        self.try_update_native_control_position();
    }

    fn get_absolute_bounds(&self) -> Option<Rect> {
        let current_root = self.current_root.borrow().clone();
        debug_assert!(current_root.is_some());

        let bounds = self.bounds();
        // Native window is not rendered by the framework
        let root_visual = current_root.and_then(|current_root| current_root.root_visual());
        let transform_to_visual = root_visual.and_then(|root_visual| self.transform_to_visual(&root_visual))?;
        let mut transformed_rect =
            Rect::from_position_size(Point::default(), bounds.size()).transform_to_aabb(transform_to_visual);
        // Transformed rect should be pixel-rounded if layout rounding is enabled.
        // This is important for native controls to align correctly with the visual tree.
        if self.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(self);
            let left = LayoutHelper::round_layout_value(transformed_rect.x, scale);
            let top = LayoutHelper::round_layout_value(transformed_rect.y, scale);
            let right = LayoutHelper::round_layout_value(transformed_rect.right(), scale);
            let bottom = LayoutHelper::round_layout_value(transformed_rect.bottom(), scale);
            transformed_rect = Rect::from_points(Point::new(left, top), Point::new(right, bottom));
        }

        Some(transformed_rect)
    }

    fn enqueue_for_move_resize(&self) {
        if self.queued_for_move_resize.get() {
            return;
        }
        self.queued_for_move_resize.set(true);
        let this = self.to_ref();
        Dispatcher::ui_thread().post_local(move || this.update_host(), DispatcherPriority::AFTER_RENDER);
    }

    /// Moves the native control to the bounds of the host, or hides it
    /// when the host is not visible. Returns `false` when there is no
    /// native control host or the host has no size yet.
    pub fn try_update_native_control_position(&self) -> bool {
        if self.current_host.borrow().is_none() {
            return false;
        }

        let bounds = self.get_absolute_bounds();
        let attachment = self.attachment.borrow().clone();

        match bounds {
            Some(bounds) if self.is_effectively_visible() => {
                if bounds.width == 0.0 && bounds.height == 0.0 {
                    return false;
                }
                if let Some(attachment) = attachment {
                    attachment.show_in_bounds(bounds);
                }
            }
            _ => {
                if let Some(attachment) = attachment {
                    attachment.hide_with_size(self.bounds().size());
                }
            }
        }
        true
    }

    fn check_destruction(&self) {
        self.queued_for_destruction.set(false);
        if self.current_root.borrow().is_none() {
            self.destroy_native_control();
        }
    }

    fn destroy_native_control(&self) {
        if let Some(native_control_handle) = self.native_control_handle() {
            let attachment = self.attachment.borrow_mut().take();
            if let Some(attachment) = attachment {
                attachment.dispose();
            }

            self.destroy_native_control_core(native_control_handle);
            self.set_native_control_handle(None);
        }
    }
}
