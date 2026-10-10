//! Views of the system inside a view of the framework: the native control
//! host of a top-level.
//!
//! A native control is a view that is added to the Java view of the
//! framework, a frame layout, over the surface the framework renders to,
//! and placed with the layout parameters of a frame layout.

use crate::android_view_control_handle::{AndroidViewControlHandle, ANDROID_VIEW_DESCRIPTOR};
use crate::interop::java::{call_static_void, call_void, new_object, JavaClass, JavaObject, JavaValue};
use crate::interop::natives::PLATFORM_HELPER;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Size};
use ferroui_controls::platform::{
    INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle, INativeControlHostImpl,
    IPlatformHandle,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// `View.VISIBLE` and `View.GONE`.
const VISIBLE: i32 = 0;
const GONE: i32 = 8;

pub struct AndroidNativeControlHostImpl {
    this: Weak<AndroidNativeControlHostImpl>,
    /// The Java view of the framework.
    ferro_view: JavaObject,
    context: JavaObject,
    /// `TopLevelImpl.RenderScaling` of the view.
    render_scaling: Box<dyn Fn() -> f64>,
}

impl AndroidNativeControlHostImpl {
    pub(crate) fn new(
        ferro_view: JavaObject,
        context: JavaObject,
        render_scaling: Box<dyn Fn() -> f64>,
    ) -> Rc<AndroidNativeControlHostImpl> {
        Rc::new_cyclic(|this| AndroidNativeControlHostImpl { this: this.clone(), ferro_view, context, render_scaling })
    }

    fn as_host(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
        self.this.upgrade().map(|this| this as Rc<dyn INativeControlHostImpl>)
    }
}

impl INativeControlHostImpl for AndroidNativeControlHostImpl {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_default_child(&self, _parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostDestroyableControlHandle> {
        let frame_layout = new_object(
            &JavaClass::find("android/widget/FrameLayout"),
            "(Landroid/content/Context;)V",
            &[JavaValue::Object(Some(&self.context))],
        );
        Rc::new(AndroidViewControlHandle::new(frame_layout.to_global()))
    }

    fn create_new_attachment_with(
        &self,
        create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        let parent: Rc<dyn IPlatformHandle> = Rc::new(AndroidViewControlHandle::new(self.ferro_view.clone()));
        let child = create(parent);
        // The reference disposes the attachment when attaching fails; here a failure is a
        // panic, and the attachment is released as the panic unwinds.
        let attachment = Rc::new(AndroidNativeControlAttachment::new(child));
        attachment.set_attached_to(self.as_host());
        attachment
    }

    fn create_new_attachment(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        let attachment = Rc::new(AndroidNativeControlAttachment::new(handle));
        attachment.set_attached_to(self.as_host());
        attachment
    }

    fn is_compatible_with(&self, handle: &dyn IPlatformHandle) -> bool {
        handle.handle_descriptor() == Some(ANDROID_VIEW_DESCRIPTOR)
    }
}

struct AndroidNativeControlAttachment {
    view: RefCell<Option<JavaObject>>,
    attached_to: RefCell<Option<Rc<AndroidNativeControlHostImpl>>>,
}

impl AndroidNativeControlAttachment {
    /// # Panics
    /// Panics when the handle is not the handle of a view of this backend.
    /// (The reference also takes the raw handle of a view of any other
    /// handle class; a number cannot be turned into a reference to an
    /// object safely here: recorded in docs/porting/DEVIATIONS.md.)
    fn new(child: Rc<dyn IPlatformHandle>) -> Self {
        let view = child.as_any().downcast_ref::<AndroidViewControlHandle>().and_then(AndroidViewControlHandle::view);
        let Some(view) = view else {
            panic!(
                "The handle of a native control on Android must be an AndroidViewControlHandle that still has its \
                 view."
            );
        };
        Self { view: RefCell::new(Some(view)), attached_to: RefCell::new(None) }
    }

    fn check_disposed(&self) -> JavaObject {
        match self.view.borrow().clone() {
            Some(view) => view,
            None => panic!("Cannot access a disposed object. Object name: 'AndroidNativeControlAttachment'."),
        }
    }

    /// `view.LayoutParameters = new FrameLayout.LayoutParams(width, height) { LeftMargin, TopMargin }`.
    fn set_layout(view: &JavaObject, visibility: i32, width: i32, height: i32, left_margin: i32, top_margin: i32) {
        call_void(view, "setVisibility", "(I)V", &[JavaValue::Int(visibility)]);
        call_static_void(
            &JavaClass::find(PLATFORM_HELPER),
            "setFrameLayoutParams",
            "(Landroid/view/View;IIII)V",
            &[
                JavaValue::Object(Some(view)),
                JavaValue::Int(width),
                JavaValue::Int(height),
                JavaValue::Int(left_margin),
                JavaValue::Int(top_margin),
            ],
        );
        call_void(view, "requestLayout", "()V", &[]);
    }
}

impl IDisposable for AndroidNativeControlAttachment {
    fn dispose(&self) {
        let view = self.view.borrow_mut().take();
        let attached_to = self.attached_to.borrow_mut().take();
        if let (Some(view), Some(attached_to)) = (&view, &attached_to) {
            call_void(
                &attached_to.ferro_view,
                "removeView",
                "(Landroid/view/View;)V",
                &[JavaValue::Object(Some(view))],
            );
        }
        // The reference disposes the managed peer of the view here; the reference to the
        // view is released.
        drop(view);
    }
}

impl INativeControlHostControlTopLevelAttachment for AndroidNativeControlAttachment {
    fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
        self.attached_to.borrow().clone().map(|host| host as Rc<dyn INativeControlHostImpl>)
    }

    fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>) {
        let view = self.check_disposed();

        let value = value.map(|host| match host.as_any().downcast_ref::<AndroidNativeControlHostImpl>() {
            Some(host) => host.this.upgrade().expect("the host is alive while it is named"),
            None => panic!("Unable to cast the native control host to AndroidNativeControlHostImpl."),
        });
        let old_attached_to = self.attached_to.replace(value.clone());
        match value {
            None => {
                if let Some(old_attached_to) = old_attached_to {
                    call_void(
                        &old_attached_to.ferro_view,
                        "removeView",
                        "(Landroid/view/View;)V",
                        &[JavaValue::Object(Some(&view))],
                    );
                }
            }
            Some(attached_to) => {
                call_void(
                    &attached_to.ferro_view,
                    "addView",
                    "(Landroid/view/View;)V",
                    &[JavaValue::Object(Some(&view))],
                );
            }
        }
    }

    fn is_compatible_with(&self, host: &dyn INativeControlHostImpl) -> bool {
        host.as_any().is::<AndroidNativeControlHostImpl>()
    }

    fn hide_with_size(&self, size: Size) {
        let view = self.check_disposed();
        let Some(attached_to) = self.attached_to.borrow().clone() else {
            return;
        };

        let size = size * (attached_to.render_scaling)();
        Self::set_layout(&view, GONE, 1.max(size.width as i32), 1.max(size.height as i32), 0, 0);
    }

    fn show_in_bounds(&self, bounds: Rect) {
        let view = self.check_disposed();
        let Some(attached_to) = self.attached_to.borrow().clone() else {
            panic!("The control isn't currently attached to a toplevel");
        };

        let bounds = bounds * (attached_to.render_scaling)();
        Self::set_layout(
            &view,
            VISIBLE,
            1.max(bounds.width as i32),
            1.max(bounds.height as i32),
            bounds.x as i32,
            bounds.y as i32,
        );
    }
}
