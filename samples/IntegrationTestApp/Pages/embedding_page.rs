//! Port of `Pages/EmbeddingPage.xaml.cs`: the class of the document `Pages/EmbeddingPage.xaml`
//! (a class of the root namespace in the upstream sample, unlike the classes of the other
//! pages).

use crate::embedding::NativeTextBox;
use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{TextBox, UserControl};
use std::rc::Rc;

/// `NSModalResponseContinue`.
#[cfg(target_os = "macos")]
const NS_MODAL_RESPONSE_CONTINUE: isize = -1002;

#[repr(C)]
pub struct EmbeddingPage {
    base: UserControl,
}

user_control_class!(EmbeddingPage);
ferro_class_info!(EmbeddingPage {
    new: EmbeddingPage::new,
    markup: {
        methods: [
            fn Reset_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddingPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.reset_click(&sender, e.as_routed_event_args())
                },
            fn RunNativeModalSession_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddingPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.run_native_modal_session_on_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(EmbeddingPage, "/Pages/EmbeddingPage.xaml");

impl EmbeddingPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.reset_text();
        this
    }

    fn native_text_box(&self) -> Ref<NativeTextBox> {
        self.get_control::<NativeTextBox>("NativeTextBox")
    }

    fn native_text_box_in_popup(&self) -> Ref<NativeTextBox> {
        self.get_control::<NativeTextBox>("NativeTextBoxInPopup")
    }

    fn modal_result_text_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("ModalResultTextBox")
    }

    fn reset_text(&self) {
        self.native_text_box_in_popup().set_text("Native text box");
        self.native_text_box().set_text("Native text box");
    }

    fn reset_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.reset_text();
        self.modal_result_text_box().set_text(Some(""));
    }

    /// Runs a modal session of AppKit for a native window whose content is an embedded root
    /// with a button.
    #[cfg(target_os = "macos")]
    fn run_native_modal_session_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        use crate::embedding::{objc, MacHelper};

        MacHelper::ensure_initialized();

        let app = objc::send_id(objc::class("NSApplication"), "sharedApplication");
        let (modal_window, delegate) = self.create_native_window();
        let session = objc::send_ptr_id(app, "beginModalSessionForWindow:", modal_window);

        loop {
            if objc::send_isize_ptr(app, "runModalSession:", session) != NS_MODAL_RESPONSE_CONTINUE {
                break;
            }
        }

        objc::send_void_ptr(app, "endModalSession:", session);

        // The window and the object that listens to it are the ones `create_native_window`
        // created; the managed original leaves them to its collector.
        objc::send_void_id(modal_window, "setDelegate:", std::ptr::null_mut());
        objc::send_void(delegate, "release");
        objc::send_void(modal_window, "release");
    }

    /// The native modal window is a window of AppKit: nothing to run on another platform.
    #[cfg(not(target_os = "macos"))]
    fn run_native_modal_session_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {}

    /// `CreateNativeWindow`: the native window and the object that listens to its closing
    /// (the `WillClose` event of the binding of the managed original).
    ///
    /// # Panics
    /// Panics if the embedded root has no platform handle (the invalid operation exception of
    /// the managed original).
    #[cfg(target_os = "macos")]
    fn create_native_window(&self) -> (crate::embedding::objc::Id, crate::embedding::objc::Id) {
        use crate::embedding::objc::{self, CGRect, Id, Imp, Sel};
        use ferroui_controls::automation::AutomationProperties;
        use ferroui_controls::embedding::EmbeddableControlRoot;
        use ferroui_controls::platform::IPlatformHandle;
        use ferroui_controls::{Button, Control};

        /// `NSWindowStyle.Titled | NSWindowStyle.Closable`.
        const STYLE_TITLED_CLOSABLE: usize = 1 | 2;
        /// `NSBackingStore.Buffered`.
        const BACKING_BUFFERED: usize = 2;

        /// `window.WillClose += (_, _) => NSApplication.SharedApplication.StopModal();`
        extern "C" fn window_will_close(_this: Id, _selector: Sel, _notification: Id) {
            let app = objc::send_id(objc::class("NSApplication"), "sharedApplication");
            objc::send_void(app, "stopModal");
        }

        let button = Button::new();
        button.set_name(Some("ButtonInModal".to_string()));
        button.set_content(Some(Rc::new(String::from("Button")) as BoxedValue));

        AutomationProperties::set_automation_id(&button, Some("ButtonInModal"));

        let root = EmbeddableControlRoot::new();
        root.set_width(200.0);
        root.set_height(200.0);
        root.set_content(Some(Control::boxed(&button)));
        root.prepare();

        let window = objc::init_window(
            objc::send_id(objc::class("NSWindow"), "alloc"),
            CGRect { x: 0.0, y: 0.0, width: root.width(), height: root.height() },
            STYLE_TITLED_CLOSABLE,
            BACKING_BUFFERED,
            false,
        );
        // The page releases the window when the modal session has ended.
        objc::send_void_bool(window, "setReleasedWhenClosed:", false);

        objc::send_void_id(window, "setIdentifier:", objc::ns_string("ModalNativeWindow"));
        // SAFETY: the implementation takes the receiver, the selector and one object and
        // returns nothing, as its type encoding states.
        let delegate_class = unsafe {
            let will_close = std::mem::transmute::<extern "C" fn(Id, Sel, Id), Imp>(window_will_close);
            objc::declare_class(
                "IntegrationTestAppModalWindowDelegate",
                objc::class("NSObject"),
                &[("windowWillClose:", will_close, "v@:@")],
            )
        };
        let delegate = objc::send_id(objc::send_id(delegate_class, "alloc"), "init");
        objc::send_void_id(window, "setDelegate:", delegate);

        {
            // The button is the content of the embedded root: its handler holds the page
            // weakly. The native window outlives the click: the page releases it after the
            // modal session, which the close ends.
            let weak = self.to_ref().downgrade();
            let window = window as usize;
            button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.modal_result_text_box().set_text(Some("Clicked"));
                }
                objc::send_void(window as Id, "close");
            });
        }

        let Some(handle) = root.try_get_platform_handle() else {
            panic!("Could not get platform handle");
        };

        objc::send_void_id(window, "setContentView:", handle.handle() as Id);
        root.start_rendering();

        (window, delegate)
    }
}
