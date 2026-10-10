//! Port of `EmbedSample.iOS.cs`: the native control demo of iOS. The
//! first sample is a button of UIKit that counts its touches; the second
//! is a web view of WebKit showing a page.
//!
//! The button of the reference is a `UIButton` with a managed handler as
//! its target; a target is an object of the Objective-C runtime, so the
//! button here is a subclass of `UIButton` that is its own target and
//! keeps the count. The web view is created through the runtime by the
//! name of its class: WebKit has no bindings among the dependencies of
//! the workspace, and the sample needs three messages of it.

use control_catalog::pages::INativeDemoControl;
use ferroui_controls::platform::IPlatformHandle;
use ferroui_ios::UIViewControlHandle;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSObjectProtocol, NSString, NSURL};
use objc2_ui_kit::{UIButton, UIColor, UIControlEvents, UIControlState, UIView};
use std::cell::Cell;
use std::rc::Rc;

pub struct EmbedSampleIos;

define_class!(
    // SAFETY: `UIButton` may be subclassed; the class overrides nothing
    // and does not implement `Drop`.
    #[unsafe(super(UIButton))]
    #[thread_kind = MainThreadOnly]
    #[name = "FerroEmbedSampleButton"]
    #[ivars = Cell<u32>]
    struct EmbedSampleButton;

    impl EmbedSampleButton {
        #[unsafe(method(ferroTouchDown:))]
        fn touch_down(&self, _sender: &AnyObject) {
            let click_count = self.ivars().get() + 1;
            self.ivars().set(click_count);
            self.setTitle_forState(Some(&NSString::from_str(&format!("Click count {click_count}"))), UIControlState::Normal);
        }
    }

    unsafe impl NSObjectProtocol for EmbedSampleButton {}
);

impl EmbedSampleButton {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(Cell::new(0));
        // SAFETY: `init` of the superclass, on the object that was just
        // allocated and whose instance variables are set.
        let button: Retained<Self> = unsafe { msg_send![super(this), init] };
        button.setTitle_forState(Some(&NSString::from_str("Hello world")), UIControlState::Normal);
        button.setBackgroundColor(Some(&UIColor::blueColor()));
        // SAFETY: the button responds to the selector (its class declares
        // `ferroTouchDown:`, which takes the sender); a control does not
        // retain its target, which here is the control itself.
        unsafe {
            button.addTarget_action_forControlEvents(Some(&button), sel!(ferroTouchDown:), UIControlEvents::TouchDown);
        }
        button
    }
}

#[link(name = "WebKit", kind = "framework")]
extern "C" {}

/// A web view of WebKit that loads `address`; none when the class is not
/// there.
fn web_view(mtm: MainThreadMarker, address: &str) -> Option<Retained<UIView>> {
    let web_view_class = AnyClass::get(c"WKWebView")?;
    let configuration_class = AnyClass::get(c"WKWebViewConfiguration")?;
    let request_class = AnyClass::get(c"NSURLRequest")?;
    let url = NSURL::URLWithString(&NSString::from_str(address))?;
    let _ = mtm;
    // SAFETY: the messages are those of the three classes, with the
    // argument and return types their headers declare: `new` of a
    // configuration, `initWithFrame:configuration:` of a web view (a
    // rectangle and a configuration), `requestWithURL:` of a request and
    // `loadRequest:` of the web view, whose result (a navigation) is not
    // used. A web view is a view; it is created on the main thread, which
    // the marker stands for.
    unsafe {
        let configuration: Retained<AnyObject> = msg_send![configuration_class, new];
        let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(0.0, 0.0));
        let allocated: Allocated<AnyObject> = msg_send![web_view_class, alloc];
        let web_view: Option<Retained<AnyObject>> =
            msg_send![allocated, initWithFrame: frame, configuration: &*configuration];
        let web_view = web_view?;
        let request: Retained<AnyObject> = msg_send![request_class, requestWithURL: &*url];
        let _navigation: Option<Retained<AnyObject>> = msg_send![&*web_view, loadRequest: &*request];
        web_view.downcast::<UIView>().ok()
    }
}

impl INativeDemoControl for EmbedSampleIos {
    fn create_control(
        &self,
        is_second: bool,
        _parent: Rc<dyn IPlatformHandle>,
        create_default: &dyn Fn() -> Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn IPlatformHandle> {
        let Some(mtm) = MainThreadMarker::new() else {
            panic!("A native control is created on the main thread.");
        };

        if is_second {
            match web_view(mtm, "https://www.apple.com/") {
                Some(web_view) => Rc::new(UIViewControlHandle::new(web_view)),
                // The reference has no such case: its web view is a class
                // of its bindings. Without the class the default control
                // of the platform stands in.
                None => create_default(),
            }
        } else {
            let button = EmbedSampleButton::new(mtm);
            let view: Retained<UIView> = Retained::into_super(Retained::into_super(Retained::into_super(button)));
            Rc::new(UIViewControlHandle::new(view))
        }
    }
}
