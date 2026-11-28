//! The pointer input of a `Viewport3D`, routed to its `UIElement3D`s through the 3D hit testing
//! (`ui_element_3d.rs` describes the rules; `docs/porting/media3d.md`, section 3.17).
//!
//! The viewport receives the 2D pointer events of the input system (the 2D hit testing hits it
//! where one of its models is: its custom hit test is the 3D hit test). For each it hit tests
//! the scene at the position of the pointer, updates which elements the pointer is over (exited
//! on the elements it left, innermost first, then entered on those it came over, outermost
//! first), and raises a move, press or release on the route of the nearest hit (or of the
//! element that captures the pointer), innermost first, until a handler handles it.

use crate::hit_testing::{hit_test_route, route_of};
use crate::ui_element_3d::{Pointer3DEventArgs, PointerEvent3D};
use crate::{RayMeshGeometry3DHitTestResult, UIElement3D, Viewport3D};
use ferroui_base::input::{IPointer, InputElement, PointerEventArgs};
use ferroui_base::{Ref, Visual};
use std::cell::RefCell;
use std::rc::Rc;

/// The element that captures a pointer, and the route of its events.
#[derive(Clone)]
struct Capture3D {
    element: Ref<UIElement3D>,
    route: Vec<Ref<UIElement3D>>,
    pointer: Rc<dyn IPointer>,
}

/// The state of the routing of one viewport.
#[derive(Default)]
pub(crate) struct Viewport3DInput {
    /// The elements the pointer is over, innermost first.
    over: RefCell<Vec<Ref<UIElement3D>>>,
    captured: RefCell<Option<Capture3D>>,
}

impl Viewport3DInput {
    /// Routes a 2D pointer event of the viewport: `event` is the 3D event to raise (none for the
    /// pointer coming over the viewport, which only updates what it is over).
    pub(crate) fn route(&self, viewport: &Viewport3D, e: &PointerEventArgs, event: Option<PointerEvent3D>) {
        let visual: &Visual = viewport;
        let position = e.get_position(Some(visual));
        let (hit, hit_route) = match hit_test_route(viewport.viewport_3d_visual(), position) {
            Some((hit, route)) => (Some(hit), route),
            None => (None, Vec::new()),
        };
        let captured = self.captured.borrow().clone();
        let over = match &captured {
            // Under a capture the pointer is over the capturing element (and those that hold it)
            // only while the nearest hit is in it.
            Some(capture) => {
                if hit_route.iter().any(|element| element.ptr_eq(&capture.element)) {
                    capture.route.clone()
                } else {
                    Vec::new()
                }
            }
            None => hit_route.clone(),
        };
        self.update_over(viewport, over, e.pointer(), Some(e), hit.as_ref());
        let Some(event) = event else { return };
        let route = match captured {
            Some(capture) => capture.route,
            None => hit_route,
        };
        if raise_on_route(viewport, &route, event, e.pointer(), Some(e), hit.as_ref()) {
            e.set_handled(true);
        }
    }

    /// The pointer left the viewport: it is over none of its elements.
    pub(crate) fn exited(&self, viewport: &Viewport3D, e: &PointerEventArgs) {
        self.update_over(viewport, Vec::new(), e.pointer(), Some(e), None);
    }

    /// The viewport lost the 2D capture of `pointer`: the element that captured it through the
    /// viewport loses it too.
    pub(crate) fn capture_lost(&self, viewport: &Viewport3D, pointer: &Rc<dyn IPointer>) {
        let lost = {
            let mut captured = self.captured.borrow_mut();
            match captured.as_ref() {
                Some(capture) if capture.pointer.id() == pointer.id() => captured.take(),
                _ => None,
            }
        };
        if let Some(capture) = lost {
            raise_capture_lost(viewport, &capture);
        }
    }

    /// The element that captures a pointer through the viewport.
    pub(crate) fn captured_element(&self) -> Option<Ref<UIElement3D>> {
        self.captured.borrow().as_ref().map(|capture| capture.element.clone())
    }

    /// `element` captures `pointer`; see [`UIElement3D::capture_pointer`].
    pub(crate) fn capture(
        &self,
        viewport: &Viewport3D,
        element: &Ref<UIElement3D>,
        pointer: &Rc<dyn IPointer>,
    ) -> bool {
        let Some(route) = route_of(viewport.viewport_3d_visual(), element) else { return false };
        let input: Ref<InputElement> = viewport.to_ref().upcast();
        if !pointer.captured().is_some_and(|captured| captured.ptr_eq(&input)) {
            pointer.capture(Some(&input));
            if !pointer.captured().is_some_and(|captured| captured.ptr_eq(&input)) {
                // A handler of the 2D capture refused it.
                return false;
            }
        }
        let capture = Capture3D { element: element.clone(), route, pointer: pointer.clone() };
        let previous = self.captured.replace(Some(capture));
        if let Some(previous) = previous {
            if !previous.element.ptr_eq(element) {
                raise_capture_lost(viewport, &previous);
            }
        }
        true
    }

    /// `element` releases its capture of `pointer`, if it holds it.
    pub(crate) fn release(&self, viewport: &Viewport3D, element: &Ref<UIElement3D>, pointer: &Rc<dyn IPointer>) {
        let released = {
            let mut captured = self.captured.borrow_mut();
            match captured.as_ref() {
                Some(capture) if capture.element.ptr_eq(element) && capture.pointer.id() == pointer.id() => {
                    captured.take()
                }
                _ => None,
            }
        };
        let Some(capture) = released else { return };
        raise_capture_lost(viewport, &capture);
        let input: Ref<InputElement> = viewport.to_ref().upcast();
        if pointer.captured().is_some_and(|captured| captured.ptr_eq(&input)) {
            pointer.capture(None);
        }
    }

    /// Makes `over` the elements the pointer is over, raising exited and entered on the
    /// elements that changed.
    fn update_over(
        &self,
        viewport: &Viewport3D,
        over: Vec<Ref<UIElement3D>>,
        pointer: &Rc<dyn IPointer>,
        e: Option<&PointerEventArgs>,
        hit: Option<&RayMeshGeometry3DHitTestResult>,
    ) {
        let previous = self.over.replace(over.clone());
        let contains = |list: &[Ref<UIElement3D>], element: &Ref<UIElement3D>| list.iter().any(|e| e.ptr_eq(element));
        for element in previous.iter().filter(|element| !contains(&over, element)) {
            element.set_is_pointer_over(false);
            let args = Pointer3DEventArgs::new(element.clone(), viewport.to_ref(), pointer.clone(), e, hit);
            element.raise_pointer_event(PointerEvent3D::Exited, &args);
        }
        for element in over.iter().rev().filter(|element| !contains(&previous, element)) {
            element.set_host(viewport);
            element.set_is_pointer_over(true);
            let args = Pointer3DEventArgs::new(element.clone(), viewport.to_ref(), pointer.clone(), e, hit);
            element.raise_pointer_event(PointerEvent3D::Entered, &args);
        }
    }
}

/// Raises `event` on the elements of `route`, innermost first, until one handles it; whether
/// one did.
fn raise_on_route(
    viewport: &Viewport3D,
    route: &[Ref<UIElement3D>],
    event: PointerEvent3D,
    pointer: &Rc<dyn IPointer>,
    e: Option<&PointerEventArgs>,
    hit: Option<&RayMeshGeometry3DHitTestResult>,
) -> bool {
    let Some(source) = route.first() else { return false };
    let args = Pointer3DEventArgs::new(source.clone(), viewport.to_ref(), pointer.clone(), e, hit);
    for element in route {
        element.set_host(viewport);
        element.raise_pointer_event(event, &args);
        if args.handled() {
            return true;
        }
    }
    false
}

fn raise_capture_lost(viewport: &Viewport3D, capture: &Capture3D) {
    let args = Pointer3DEventArgs::new(capture.element.clone(), viewport.to_ref(), capture.pointer.clone(), None, None);
    capture.element.raise_pointer_event(PointerEvent3D::CaptureLost, &args);
}
