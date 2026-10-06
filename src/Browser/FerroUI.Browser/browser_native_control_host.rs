use crate::interop::{native_control_host_helper, non_null, JsObject};
use crate::js_object_control_handle::JsObjectControlHandle;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Size};
use ferroui_controls::platform::{
    INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle, INativeControlHostImpl,
    IPlatformHandle,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Hosts elements of the page as native controls of a top-level: they are
/// placed, absolutely positioned, in the element of the view that lies over
/// its canvas (`nativeHost`, created with the view).
pub(crate) struct BrowserNativeControlHost {
    this: Weak<BrowserNativeControlHost>,
    host_element: JsObject,
}

impl BrowserNativeControlHost {
    /// The host of the native controls placed in `native_control_host`.
    ///
    /// # Panics
    /// Panics when `native_control_host` is `null` or `undefined` (an
    /// `ArgumentNullException` in the managed original).
    pub(crate) fn new(native_control_host: JsObject) -> Rc<Self> {
        let Some(host_element) = non_null(native_control_host) else {
            panic!("Value cannot be null. (Parameter 'nativeControlHost')");
        };
        Rc::new_cyclic(|this| Self { this: this.clone(), host_element })
    }

    fn rc(&self) -> Rc<BrowserNativeControlHost> {
        self.this.upgrade().expect("the native control host is alive while it is used")
    }
}

impl INativeControlHostImpl for BrowserNativeControlHost {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_default_child(
        &self,
        _parent: Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn INativeControlHostDestroyableControlHandle> {
        let element = native_control_host_helper::create_default_child(None);
        Rc::new(JsObjectControlHandle::new(element))
    }

    fn create_new_attachment_with(
        &self,
        create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        // The original disposes the attachment when setting its host throws. The calls into the
        // page do not fail with a Rust error, so there is nothing to clean up here.
        let child = create(Rc::new(JsObjectControlHandle::new(self.host_element.clone())));
        let attachment_reference = native_control_host_helper::create_attachment();
        let a = Rc::new(Attachment::new(attachment_reference, &*child));
        a.set_attached_to(Some(self.rc()));
        a
    }

    fn create_new_attachment(
        &self,
        handle: Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        let attachment_reference = native_control_host_helper::create_attachment();
        let a = Rc::new(Attachment::new(attachment_reference, &*handle));
        a.set_attached_to(Some(self.rc()));
        a
    }

    fn is_compatible_with(&self, handle: &dyn IPlatformHandle) -> bool {
        handle.as_any().is::<JsObjectControlHandle>()
    }
}

/// The attachment of an element of the page to the element of a top-level.
struct Attachment {
    native: RefCell<Option<JsObject>>,
    attached_to: RefCell<Option<Rc<BrowserNativeControlHost>>>,
}

impl Attachment {
    /// # Panics
    /// Panics when `handle` is not a [`JsObjectControlHandle`] (an invalid
    /// cast in the managed original) or has been destroyed.
    fn new(native: JsObject, handle: &dyn IPlatformHandle) -> Self {
        let Some(handle) = handle.as_any().downcast_ref::<JsObjectControlHandle>() else {
            panic!("Unable to cast the platform handle to JsObjectControlHandle.");
        };
        let Some(child) = handle.object() else {
            panic!("Cannot access a disposed object. Object name: 'JsObjectControlHandle'.");
        };
        native_control_host_helper::initialize_with_child_handle(&native, &child);
        Self { native: RefCell::new(Some(native)), attached_to: RefCell::new(None) }
    }

    /// The object of the attachment.
    ///
    /// # Panics
    /// Panics when the attachment has been disposed
    /// (`ObjectDisposedException` in the managed original).
    fn check_disposed(&self) -> JsObject {
        match self.native.borrow().as_ref() {
            Some(native) => native.clone(),
            None => panic!("Cannot access a disposed object. Object name: 'Attachment'."),
        }
    }
}

impl IDisposable for Attachment {
    fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        if let Some(native) = native {
            native_control_host_helper::release_child(&native);
        }
    }
}

/// `Math.Max` of .NET: NaN when an argument is NaN.
fn math_max(val1: f64, val2: f64) -> f64 {
    if val1.is_nan() || val2.is_nan() {
        f64::NAN
    } else {
        val1.max(val2)
    }
}

impl INativeControlHostControlTopLevelAttachment for Attachment {
    fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
        self.attached_to.borrow().clone().map(|host| host as Rc<dyn INativeControlHostImpl>)
    }

    /// # Panics
    /// Panics when the attachment has been disposed, or when `value` is a
    /// host of another platform (an invalid cast in the managed original).
    fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>) {
        let native = self.check_disposed();

        let host = value.map(|value| match value.as_any().downcast_ref::<BrowserNativeControlHost>() {
            Some(host) => host.rc(),
            None => panic!("Unable to cast the native control host to BrowserNativeControlHost."),
        });
        match &host {
            None => native_control_host_helper::attach_to(&native, None),
            Some(host) => native_control_host_helper::attach_to(&native, Some(&host.host_element)),
        }
        *self.attached_to.borrow_mut() = host;
    }

    fn is_compatible_with(&self, host: &dyn INativeControlHostImpl) -> bool {
        host.as_any().is::<BrowserNativeControlHost>()
    }

    fn hide_with_size(&self, size: Size) {
        let native = self.check_disposed();
        if self.attached_to.borrow().is_none() {
            return;
        }

        native_control_host_helper::hide_with_size(&native, math_max(1.0, size.width), math_max(1.0, size.height));
    }

    /// # Panics
    /// Panics when the attachment has been disposed or is not attached to a
    /// host (`InvalidOperationException` in the managed original).
    fn show_in_bounds(&self, bounds: Rect) {
        let native = self.check_disposed();

        if self.attached_to.borrow().is_none() {
            panic!("Native control isn't attached to a toplevel");
        }

        let bounds = Rect::new(bounds.x, bounds.y, math_max(1.0, bounds.width), math_max(1.0, bounds.height));

        native_control_host_helper::show_in_bounds(&native, bounds.x, bounds.y, bounds.width, bounds.height);
    }
}

#[cfg(test)]
mod tests {
    use super::math_max;

    // Not from upstream: the host itself only runs in a page (scripts/browser/tests).
    #[test]
    fn sizes_are_at_least_one_pixel_and_nan_is_kept() {
        assert_eq!(1.0, math_max(1.0, 0.0));
        assert_eq!(1.0, math_max(1.0, -5.0));
        assert_eq!(20.5, math_max(1.0, 20.5));
        assert!(math_max(1.0, f64::NAN).is_nan());
    }
}
