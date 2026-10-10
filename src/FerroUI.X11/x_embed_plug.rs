//! A control tree of the framework as an XEmbed plug: a window another
//! toolkit embeds in one of its sockets (the port of `XEmbedPlug.cs`).

use crate::x11_platform::FerroX11Platform;
use crate::x11_window::X11Window;
use crate::x11_window_modes::XEmbedClientWindowMode;
use crate::xlib::{self, XID};
use ferroui_base::media::Color;
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherPriority};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, PixelSize, Ref};
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::platform::ITopLevelImpl;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An XEmbed plug.
pub struct XEmbedPlug {
    root: RefCell<Option<Ref<EmbeddableControlRoot>>>,
    background_color: Cell<Color>,
    window: Rc<X11Window>,
    platform: Rc<FerroX11Platform>,
}

impl XEmbedPlug {
    fn new(parent_xid: Option<XID>) -> Self {
        let platform = FerroLocator::current()
            .get_service::<FerroX11Platform>()
            .unwrap_or_else(|| panic!("The X11 platform is not registered"));
        let window = X11Window::with_mode(&platform, None, Box::new(XEmbedClientWindowMode::new()), false);
        let platform_impl: Rc<dyn ITopLevelImpl> = window.clone();
        let root = EmbeddableControlRoot::with_impl(platform_impl);
        root.prepare();
        let this =
            Self { root: RefCell::new(Some(root)), background_color: Cell::new(Color::default()), window, platform };
        if let Some(parent_xid) = parent_xid {
            xlib::x_reparent_window(this.platform.display(), this.handle(), parent_xid, 0, 0);
        }

        // Make sure that the newly created XID is visible for other clients
        xlib::x_sync(this.platform.display(), false);
        this
    }

    fn root(&self) -> Ref<EmbeddableControlRoot> {
        self.root.borrow().clone().unwrap_or_else(|| panic!("Cannot access a disposed object: XEmbedPlug"))
    }

    fn mode(&self) -> &XEmbedClientWindowMode {
        self.window.mode().x_embed().expect("the window of a plug has the mode of an embedded window")
    }

    /// The window of the plug on the server.
    pub fn handle(&self) -> XID {
        let handle = self.root().try_get_platform_handle().expect("the root of a plug has a platform handle");
        handle.handle() as XID
    }

    pub fn content(&self) -> Option<BoxedValue> {
        self.root().content()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.root().set_content(value);
    }

    pub fn background_color(&self) -> Color {
        self.background_color.get()
    }

    pub fn set_background_color(&self, value: Color) {
        self.background_color.set(value);
        let display = self.platform.display();
        xlib::x_set_window_background(display, self.handle(), (value.to_uint32() | 0xff00_0000) as i32 as _);
        xlib::x_flush(display);
    }

    pub fn scale_factor(&self) -> f64 {
        self.mode().scaling(&self.window)
    }

    pub fn set_scale_factor(&self, value: f64) {
        self.mode().set_scaling(&self.window, value);
    }

    pub fn process_interactive_resize(&self, size: PixelSize) {
        let events = self.platform.dispatcher_impl();
        events.event_dispatcher().dispatch_x11_events(&CancellationToken::none());
        self.mode().process_interactive_resize(&self.window, size);
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::UI_THREAD_RENDER));
    }

    pub fn dispose(&self) {
        let root = self.root.borrow_mut().take();
        if let Some(root) = root {
            root.stop_rendering();
            root.dispose();
        }
    }

    /// A plug that is not embedded yet: an embedder takes it by its
    /// [`handle`](Self::handle).
    pub fn create() -> XEmbedPlug {
        Self::new(None)
    }

    /// A plug in the window `embedder_xid`.
    ///
    /// # Panics
    /// When `embedder_xid` is 0 (the reference throws an argument
    /// exception).
    pub fn create_in(embedder_xid: XID) -> XEmbedPlug {
        if embedder_xid == 0 {
            panic!("Value does not fall within the expected range.");
        }
        Self::new(Some(embedder_xid))
    }
}
