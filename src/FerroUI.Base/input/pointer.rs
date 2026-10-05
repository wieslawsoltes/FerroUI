use super::gesture_recognizers::GestureRecognizer;
use super::{
    IPointer, InputElement, PointerCaptureChangingEventArgs, PointerCaptureLostEventArgs, PointerType,
};
use crate::reactive::IDisposable;
use crate::{Ref, Visual, VisualTreeAttachmentEventArgs};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicI32, Ordering};

/// What requested a change of pointer capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureSource {
    /// The capture was requested by application code.
    Explicit,
    /// The capture was taken or released by the input system, e.g. when a
    /// pointer is pressed on or released from an element.
    Implicit,
    /// The capture changed in the platform.
    Platform,
}

static NEXT_FREE_POINTER_ID: AtomicI32 = AtomicI32::new(1000);

/// Called to capture the pointer in the platform when the effectively
/// capturing element changes.
type PlatformCapture = Box<dyn Fn(Option<&Ref<InputElement>>)>;

/// Identifies a specific input pointer and tracks the element that
/// captures it.
pub struct Pointer {
    this: Weak<Pointer>,
    id: i32,
    type_: PointerType,
    is_primary: bool,
    disposed: Cell<bool>,
    captured: RefCell<Option<Ref<InputElement>>>,
    captured_gesture_recognizer: RefCell<Option<Ref<GestureRecognizer>>>,
    capture_source: Cell<CaptureSource>,
    is_gesture_recognition_skipped: Cell<bool>,
    capture_detached: RefCell<Option<Rc<dyn IDisposable>>>,
    platform_capture: Option<PlatformCapture>,
}

impl Pointer {
    /// Returns a pointer identifier that is not in use.
    pub fn get_next_free_id() -> i32 {
        NEXT_FREE_POINTER_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// Creates a pointer.
    pub fn new(id: i32, type_: PointerType, is_primary: bool) -> Rc<Pointer> {
        Self::create(id, type_, is_primary, None)
    }

    /// Creates a pointer that captures the pointer in the platform through
    /// `platform_capture` whenever the capturing element changes.
    pub fn with_platform_capture(
        id: i32,
        type_: PointerType,
        is_primary: bool,
        platform_capture: impl Fn(Option<&Ref<InputElement>>) + 'static,
    ) -> Rc<Pointer> {
        Self::create(id, type_, is_primary, Some(Box::new(platform_capture)))
    }

    fn create(id: i32, type_: PointerType, is_primary: bool, platform_capture: Option<PlatformCapture>) -> Rc<Pointer> {
        Rc::new_cyclic(|this| Pointer {
            this: this.clone(),
            id,
            type_,
            is_primary,
            disposed: Cell::new(false),
            captured: RefCell::new(None),
            captured_gesture_recognizer: RefCell::new(None),
            capture_source: Cell::new(CaptureSource::Platform),
            is_gesture_recognition_skipped: Cell::new(false),
            capture_detached: RefCell::new(None),
            platform_capture,
        })
    }

    /// The lowest input element that is both `control1` or one of its
    /// ancestors and `control2` or one of its ancestors.
    fn find_common_parent(
        control1: Option<&Ref<InputElement>>,
        control2: Option<&Ref<InputElement>>,
    ) -> Option<Ref<InputElement>> {
        let (c1, c2) = (control1?, control2?);

        c2.get_self_and_visual_ancestors()
            .filter_map(|v| v.downcast::<InputElement>().ok())
            .find(|candidate| candidate == c1 || candidate.is_visual_ancestor_of(c1))
    }

    fn platform_capture(&self, element: Option<&Ref<InputElement>>) {
        if let Some(platform_capture) = &self.platform_capture {
            platform_capture(element);
        }
    }

    /// Notifies the pointer that its capture was lost in the platform.
    pub fn platform_capture_lost(&self) {
        self.capture_lost(CaptureSource::Platform);
    }

    pub(crate) fn capture_lost(&self, source: CaptureSource) {
        if self.disposed.get() {
            return;
        }

        self.capture_lost_core(source);
    }

    fn capture_lost_core(&self, source: CaptureSource) {
        self.capture_core(None, None, source);
        self.is_gesture_recognition_skipped.set(false);
    }

    /// Captures pointer input to the specified control, on behalf of
    /// `source`.
    pub fn capture_with_source(&self, control: Option<&Ref<InputElement>>, source: CaptureSource) {
        if self.disposed.get() {
            debug_assert!(control.is_none(), "Capturing a pointer that no longer exists.");
            return;
        }

        self.capture_core(control.cloned(), None, source);
    }

    /// The element pointer input effectively goes to: the target of the
    /// capturing gesture recognizer, or the capturing element.
    fn effective_capturer(&self) -> Option<Ref<InputElement>> {
        self.captured_gesture_recognizer().and_then(|recognizer| recognizer.target()).or_else(|| self.captured())
    }

    /// The gesture recognizer that captures the pointer, if any.
    pub fn captured_gesture_recognizer(&self) -> Option<Ref<GestureRecognizer>> {
        self.captured_gesture_recognizer.borrow().clone()
    }

    fn capture_core(
        &self,
        control: Option<Ref<InputElement>>,
        gesture_recognizer: Option<Ref<GestureRecognizer>>,
        source: CaptureSource,
    ) {
        let old_capture = self.captured();
        let old_gesture_recognizer = self.captured_gesture_recognizer();
        let old_source = self.capture_source.get();
        let old_effective_capturer = self.effective_capturer();

        // If a handler marks an implicit capture as handled, we still want
        // them to have another chance if the element is captured explicitly.
        if old_capture == control && old_gesture_recognizer == gesture_recognizer && old_source == source {
            return;
        }

        let this: Rc<dyn IPointer> = match self.this.upgrade() {
            Some(this) => this,
            None => return,
        };

        let mut common_parent = None;

        if old_capture.is_some() || control.is_some() {
            common_parent = Self::find_common_parent(control.as_ref(), old_capture.as_ref());

            // We want the capture to be cancellable even if there is no
            // currently captured element.
            let visual: &Visual = old_capture.as_ref().or(control.as_ref()).expect("one of them is set");

            for notify_target in
                visual.get_self_and_visual_ancestors().filter_map(|v| v.downcast::<InputElement>().ok())
            {
                let args = PointerCaptureChangingEventArgs::new(&notify_target, this.clone(), control.clone(), source);
                notify_target.raise_event(&args);

                if args.handled() {
                    return;
                }

                if Some(&notify_target) == common_parent.as_ref() {
                    break;
                }
            }
        }

        if old_capture.is_some() {
            let subscription = self.capture_detached.take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }

        let this_pointer = this.clone();
        if old_gesture_recognizer != gesture_recognizer {
            if let Some(old) = &old_gesture_recognizer {
                old.pointer_capture_lost_internal(&this_pointer);
            }
        }

        drop(self.captured.replace(control.clone()));
        drop(self.captured_gesture_recognizer.replace(gesture_recognizer));
        self.capture_source.set(source);

        // However, we still want to notify the platform only if the captured
        // element actually changed.
        let effective_capturer = self.effective_capturer();
        if old_effective_capturer != effective_capturer && source != CaptureSource::Platform {
            self.platform_capture(effective_capturer.as_ref());
        }

        if let Some(old_visual) = &old_capture {
            for notify_target in
                old_visual.get_self_and_visual_ancestors().filter_map(|v| v.downcast::<InputElement>().ok())
            {
                if Some(&notify_target) == common_parent.as_ref() {
                    break;
                }

                notify_target.raise_event(&PointerCaptureLostEventArgs::new(&notify_target, this.clone()));
            }
        }

        if let Some(new_visual) = &control {
            let weak = self.this.clone();
            let subscription = new_visual.detached_from_visual_tree(move |e| {
                if let Some(pointer) = weak.upgrade() {
                    pointer.on_capture_detached(e);
                }
            });
            let old = self.capture_detached.replace(Some(subscription));
            if let Some(old) = old {
                old.dispose();
            }
        }

        if self.captured.borrow().is_none() && self.captured_gesture_recognizer.borrow().is_none() {
            self.is_gesture_recognition_skipped.set(false);
        }

        // Update the pointer-over + cursor immediately following the capture
        // change.
        if self.type_ != PointerType::Touch {
            let old_input_root = old_capture.as_ref().and_then(|v| v.get_input_root());
            let new_input_root = control.as_ref().and_then(|v| v.get_input_root());

            if let Some(old_input_root) = &old_input_root {
                old_input_root.pointer_over_invalidated();
            }

            let same_root = match (&old_input_root, &new_input_root) {
                (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
                (None, None) => true,
                _ => false,
            };

            if !same_root {
                if let Some(new_input_root) = &new_input_root {
                    new_input_root.pointer_over_invalidated();
                }
            }
        }
    }

    fn get_next_capture(parent: Option<&Ref<Visual>>) -> Option<Ref<InputElement>> {
        parent.and_then(|parent| parent.find_ancestor_of_type::<InputElement>(true))
    }

    fn on_capture_detached(&self, e: &VisualTreeAttachmentEventArgs) {
        let next = Self::get_next_capture(e.attachment_point());
        self.capture(next.as_ref());
    }

    /// Captures the pointer to a gesture recognizer (or releases that
    /// capture).
    pub(crate) fn capture_gesture_recognizer(&self, gesture_recognizer: Option<&Ref<GestureRecognizer>>) {
        if self.disposed.get() {
            debug_assert!(
                gesture_recognizer.is_none(),
                "Capturing a pointer that no longer exists to a gesture recognizer."
            );
            return;
        }

        self.capture_core(None, gesture_recognizer.cloned(), CaptureSource::Explicit);
    }

    /// What requested the current capture state.
    pub fn capture_source(&self) -> CaptureSource {
        self.capture_source.get()
    }

    /// Whether gesture recognition is skipped for the pointer until it is
    /// released.
    pub fn is_gesture_recognition_skipped(&self) -> bool {
        self.is_gesture_recognition_skipped.get()
    }

    /// Sets whether gesture recognition is skipped for the pointer.
    pub fn set_is_gesture_recognition_skipped(&self, value: bool) {
        self.is_gesture_recognition_skipped.set(value)
    }

    /// Releases the pointer: its capture is lost and it can no longer be
    /// captured.
    pub fn dispose(&self) {
        if self.disposed.get() {
            return;
        }

        // Mark the pointer gone first, so a capture lost handler can't
        // capture it again. It no longer exists, so the platform is the
        // only source the release can come from.
        self.disposed.set(true);
        self.capture_lost_core(CaptureSource::Platform);
    }
}

impl IPointer for Pointer {
    fn id(&self) -> i32 {
        self.id
    }

    fn capture(&self, control: Option<&Ref<InputElement>>) {
        self.capture_with_source(control, CaptureSource::Explicit);
    }

    fn captured(&self) -> Option<Ref<InputElement>> {
        self.captured.borrow().clone()
    }

    fn type_(&self) -> PointerType {
        self.type_
    }

    fn is_primary(&self) -> bool {
        self.is_primary
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
