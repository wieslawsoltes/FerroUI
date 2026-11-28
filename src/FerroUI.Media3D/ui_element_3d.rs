//! WPF's `UIElement3D`, `ModelUIElement3D` and `ContainerUIElement3D`: the 3D visuals that take
//! pointer input.
//!
//! A `Viewport3D` routes the pointer input it receives to them through the 3D hit testing
//! (`controls/viewport_3d_input.rs`): an event goes to the `UIElement3D` that holds the visual
//! hit nearest under the pointer, and then up through the `UIElement3D`s that hold it, until a
//! handler marks it handled (which also marks the 2D event of the `Viewport3D` handled). The
//! pointer is over an element while the nearest hit is in it (entered and exited are raised on
//! each element alone, as WPF raises `MouseEnter` and `MouseLeave`); an element that captures
//! the pointer receives its events wherever it moves, until it releases the capture or the
//! `Viewport3D` loses its own.
//!
//! `ModelUIElement3D` shows its `Model`, `ContainerUIElement3D` its `Children`; both draw as a
//! `ModelVisual3D` with the same content does. Their changes are changes of the structure.

use crate::change::{same, Link, ListLink, Notifying, Scene3DChange};
use crate::{Model3D, RayMeshGeometry3DHitTestResult, Viewport3D, Visual3D, Visual3DCollection};
use ferroui_base::input::{IPointer, PointerEventArgs};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Ref, StyledProperty, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The pointer events of a `UIElement3D`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerEvent3D {
    Entered,
    Exited,
    Moved,
    Pressed,
    Released,
    CaptureLost,
}

/// A handler of a pointer event of a `UIElement3D`: the element whose handler it is, and the
/// arguments.
type PointerHandler = Rc<dyn Fn(&Ref<UIElement3D>, &Pointer3DEventArgs<'_>)>;

/// The handlers of the pointer events of one element.
#[derive(Default)]
struct PointerHandlers {
    list: RefCell<Vec<(PointerEvent3D, u64, PointerHandler)>>,
    next: Cell<u64>,
}

impl PointerHandlers {
    fn add(self: &Rc<Self>, event: PointerEvent3D, handler: PointerHandler) -> Subscription {
        let id = self.next.get();
        self.next.set(id + 1);
        self.list.borrow_mut().push((event, id, handler));
        let weak = Rc::downgrade(self);
        Subscription::new(Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.list.borrow_mut().retain(|(_, other, _)| *other != id);
            }
        }))
    }

    fn of(&self, event: PointerEvent3D) -> Vec<PointerHandler> {
        self.list.borrow().iter().filter(|(kind, ..)| *kind == event).map(|(_, _, handler)| handler.clone()).collect()
    }
}

/// The arguments of a pointer event of a [`UIElement3D`] (WPF routes its `MouseEventArgs`
/// through the 3D elements; here they carry the 2D event of the `Viewport3D` and the 3D hit).
pub struct Pointer3DEventArgs<'a> {
    source: Ref<UIElement3D>,
    viewport: Ref<Viewport3D>,
    pointer: Rc<dyn IPointer>,
    pointer_event: Option<&'a PointerEventArgs>,
    hit: Option<&'a RayMeshGeometry3DHitTestResult>,
    handled: Cell<bool>,
}

impl<'a> Pointer3DEventArgs<'a> {
    pub(crate) fn new(
        source: Ref<UIElement3D>,
        viewport: Ref<Viewport3D>,
        pointer: Rc<dyn IPointer>,
        pointer_event: Option<&'a PointerEventArgs>,
        hit: Option<&'a RayMeshGeometry3DHitTestResult>,
    ) -> Self {
        Self { source, viewport, pointer, pointer_event, hit, handled: Cell::new(false) }
    }

    /// The element the event was raised on first (the innermost of its route).
    pub fn source(&self) -> &Ref<UIElement3D> {
        &self.source
    }

    /// The `Viewport3D` that routed the event.
    pub fn viewport(&self) -> &Ref<Viewport3D> {
        &self.viewport
    }

    /// The pointer.
    pub fn pointer(&self) -> &Rc<dyn IPointer> {
        &self.pointer
    }

    /// The 2D event of the `Viewport3D` this event was routed from (its pointer point,
    /// modifiers and buttons); `None` for a capture lost by a release of the element.
    pub fn pointer_event(&self) -> Option<&'a PointerEventArgs> {
        self.pointer_event
    }

    /// The position of the pointer in the coordinates of the `Viewport3D`; the origin without a
    /// 2D event.
    pub fn position(&self) -> Point {
        self.pointer_event.map_or(Point::default(), |event| event.get_position(Some(&self.viewport)))
    }

    /// The nearest hit under the pointer, when there is one (it is in another element when
    /// this one captured the pointer and it is elsewhere).
    pub fn hit(&self) -> Option<&'a RayMeshGeometry3DHitTestResult> {
        self.hit
    }

    /// Whether a handler handled the event: it goes no further.
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn set_handled(&self, value: bool) {
        self.handled.set(value)
    }
}

/// WPF's `UIElement3D`: a 3D visual that takes pointer input; see the documentation of the
/// module.
#[repr(C)]
pub struct UIElement3D {
    base: Visual3D,
    handlers: Rc<PointerHandlers>,
    is_pointer_over: Cell<bool>,
    /// The viewport that last routed input to the element: where it captures the pointer.
    host: RefCell<Option<WeakRef<Viewport3D>>>,
}

ferro_class!(UIElement3D: Visual3D);
ferro_impl_classes!(UIElement3D: FerroObjectImpl);

ferro_properties! { impl UIElement3D {
    pub fn is_hit_test_visible_property() -> StyledProperty<bool> {
        FerroProperty::register::<UIElement3D, _>("IsHitTestVisible", true)
    }
} }

impl UIElement3D {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Visual3D::construct(),
            handlers: Rc::new(PointerHandlers::default()),
            is_pointer_over: Cell::new(false),
            host: RefCell::new(None),
        }
    }

    /// Whether the element (and what it holds) can be hit; when false, the hit testing and the
    /// pointer pass through it to what is behind.
    pub fn is_hit_test_visible(&self) -> bool {
        self.get_value(Self::is_hit_test_visible_property())
    }

    pub fn set_is_hit_test_visible(&self, value: bool) {
        self.set_value(Self::is_hit_test_visible_property(), value)
    }

    /// Whether the pointer is over the element (WPF's `IsMouseOver`): the nearest hit under it
    /// is in the element; while an element captures the pointer, only the capturing element and
    /// those that hold it, and only while the pointer is over it.
    pub fn is_pointer_over(&self) -> bool {
        self.is_pointer_over.get()
    }

    /// Whether the element captures a pointer (WPF's `IsMouseCaptured`).
    pub fn is_pointer_captured(&self) -> bool {
        let this = self.to_ref();
        self.host().is_some_and(|viewport| viewport.captured_element_3d().is_some_and(|element| element.ptr_eq(&this)))
    }

    /// Captures `pointer` (WPF's `CaptureMouse`): its events go to this element until it
    /// releases it or the `Viewport3D` that routes input to it loses its own capture of the
    /// pointer (which it takes). False when no viewport routed input to the element yet, when the
    /// element is not in that viewport's scene, or when the 2D capture was refused.
    pub fn capture_pointer(&self, pointer: &Rc<dyn IPointer>) -> bool {
        match self.host() {
            Some(viewport) => viewport.capture_3d(&self.to_ref(), pointer),
            None => false,
        }
    }

    /// Releases the capture of `pointer` if this element holds it (WPF's `ReleaseMouseCapture`):
    /// its capture lost event is raised, and the `Viewport3D` releases the pointer.
    pub fn release_pointer_capture(&self, pointer: &Rc<dyn IPointer>) {
        if let Some(viewport) = self.host() {
            viewport.release_capture_3d(&self.to_ref(), pointer);
        }
    }

    /// Calls `handler` when the pointer comes over the element, until the returned guard is
    /// dropped (WPF's `MouseEnter`).
    pub fn pointer_entered(
        &self,
        handler: impl Fn(&Ref<UIElement3D>, &Pointer3DEventArgs<'_>) + 'static,
    ) -> Subscription {
        self.handlers.add(PointerEvent3D::Entered, Rc::new(handler))
    }

    /// Calls `handler` when the pointer leaves the element (WPF's `MouseLeave`).
    pub fn pointer_exited(
        &self,
        handler: impl Fn(&Ref<UIElement3D>, &Pointer3DEventArgs<'_>) + 'static,
    ) -> Subscription {
        self.handlers.add(PointerEvent3D::Exited, Rc::new(handler))
    }

    /// Calls `handler` when the pointer moves over the element or one it holds, or while it
    /// captures the pointer (WPF's `MouseMove`, bubbling).
    pub fn pointer_moved(
        &self,
        handler: impl Fn(&Ref<UIElement3D>, &Pointer3DEventArgs<'_>) + 'static,
    ) -> Subscription {
        self.handlers.add(PointerEvent3D::Moved, Rc::new(handler))
    }

    /// Calls `handler` when a button is pressed over the element (WPF's `MouseDown`, bubbling).
    pub fn pointer_pressed(
        &self,
        handler: impl Fn(&Ref<UIElement3D>, &Pointer3DEventArgs<'_>) + 'static,
    ) -> Subscription {
        self.handlers.add(PointerEvent3D::Pressed, Rc::new(handler))
    }

    /// Calls `handler` when a button is released over the element (WPF's `MouseUp`, bubbling).
    pub fn pointer_released(
        &self,
        handler: impl Fn(&Ref<UIElement3D>, &Pointer3DEventArgs<'_>) + 'static,
    ) -> Subscription {
        self.handlers.add(PointerEvent3D::Released, Rc::new(handler))
    }

    /// Calls `handler` when the element loses the capture of a pointer (WPF's
    /// `LostMouseCapture`, raised on the element alone).
    pub fn pointer_capture_lost(
        &self,
        handler: impl Fn(&Ref<UIElement3D>, &Pointer3DEventArgs<'_>) + 'static,
    ) -> Subscription {
        self.handlers.add(PointerEvent3D::CaptureLost, Rc::new(handler))
    }

    /// Calls the handlers of `event` in the order they were added, until one handles it.
    pub(crate) fn raise_pointer_event(&self, event: PointerEvent3D, args: &Pointer3DEventArgs<'_>) {
        let this = self.to_ref();
        for handler in self.handlers.of(event) {
            if args.handled() {
                break;
            }
            handler(&this, args);
        }
    }

    pub(crate) fn set_is_pointer_over(&self, value: bool) {
        self.is_pointer_over.set(value);
    }

    pub(crate) fn set_host(&self, viewport: &Viewport3D) {
        let mut host = self.host.borrow_mut();
        if host.as_ref().and_then(WeakRef::upgrade).is_some_and(|current| std::ptr::eq(&*current, viewport)) {
            return;
        }
        *host = Some(viewport.to_ref().downgrade());
    }

    fn host(&self) -> Option<Ref<Viewport3D>> {
        self.host.borrow().as_ref().and_then(WeakRef::upgrade)
    }
}

// --- ModelUIElement3D --------------------------------------------------------------------------

/// WPF's `ModelUIElement3D`: an element that shows its `Model`.
#[repr(C)]
pub struct ModelUIElement3D {
    base: UIElement3D,
    model_link: Link,
}

ferro_class!(ModelUIElement3D: UIElement3D);
ferro_class_info!(ModelUIElement3D {
    new: ModelUIElement3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        content: Model,
    },
});

impl FerroObjectImpl for ModelUIElement3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::model_property().as_property() {
            this.model_link.set(this.model().as_ref(), this.notifier(), same);
            this.notifier().raise(Scene3DChange::Structure);
        }
    }
}

ferro_properties! { impl ModelUIElement3D {
    pub fn model_property() -> StyledProperty<Option<Ref<Model3D>>> {
        FerroProperty::register::<ModelUIElement3D, _>("Model", None)
    }
} }

impl ModelUIElement3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: UIElement3D::construct(), model_link: Link::default() }
    }

    /// An element without a model.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// An element showing `model`.
    pub fn with_model(model: Ref<Model3D>) -> Ref<Self> {
        let result = Self::new();
        result.set_model(Some(model));
        result
    }

    /// The model the element shows.
    pub fn model(&self) -> Option<Ref<Model3D>> {
        self.get_value(Self::model_property())
    }

    pub fn set_model(&self, value: Option<Ref<Model3D>>) {
        self.set_value(Self::model_property(), value)
    }
}

// --- ContainerUIElement3D ----------------------------------------------------------------------

/// WPF's `ContainerUIElement3D`: an element that holds child visuals (and takes the input that
/// bubbles up from them).
#[repr(C)]
pub struct ContainerUIElement3D {
    base: UIElement3D,
    children: Visual3DCollection,
    children_link: ListLink<Visual3D>,
}

ferro_class!(ContainerUIElement3D: UIElement3D);
ferro_class_info!(ContainerUIElement3D {
    new: ContainerUIElement3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        content: Children,
        properties: [
            Children: Visual3DCollection { get: ContainerUIElement3D::children },
        ],
    },
});

impl FerroObjectImpl for ContainerUIElement3D {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.children_link.set(Some(&*this.children), this.notifier(), Scene3DChange::Structure);
    }
}

impl ContainerUIElement3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: UIElement3D::construct(), children: Visual3DCollection::new(), children_link: ListLink::default() }
    }

    /// An empty container.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The child visuals (a shared handle to the collection).
    pub fn children(&self) -> Visual3DCollection {
        self.children.clone()
    }
}
