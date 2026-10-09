use crate::browser_top_level_impl::BrowserTopLevelImpl;
use crate::interop::{dom_helper, input_helper, JsObject};
use crate::windowing_platform::BrowserWindowingPlatform;
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::Interactive;
use ferroui_base::Ref;
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::{Control, TopLevel};
use std::rc::Rc;

/// Shows content in an element of the web page.
pub struct FerroView {
    top_level: Ref<EmbeddableControlRoot>,
}

impl FerroView {
    /// Creates a view over the element with the given id.
    ///
    /// `div_id` is the ID of the html element where the content should be
    /// rendered.
    ///
    /// # Panics
    /// Panics when the document has no such element.
    pub fn new(div_id: &str) -> Rc<Self> {
        match dom_helper::get_element_by_id(div_id, &BrowserWindowingPlatform::global_this()) {
            Some(host) => Self::with_host(host),
            None => panic!("Element with id '{div_id}' was not found in the html document."),
        }
    }

    /// Creates a view over an element.
    ///
    /// `host` is the object holding a div element where the content should
    /// be rendered.
    ///
    /// # Panics
    /// Panics when `host` is null or undefined.
    pub fn with_host(host: JsObject) -> Rc<Self> {
        if host.is_null() || host.is_undefined() {
            panic!("Value cannot be null. (Parameter 'host')");
        }

        let host_content = match dom_helper::create_ferro_host(&host) {
            Some(host_content) => host_content,
            None => panic!("The host of the view wasn't initialized."),
        };

        let native_controls_container = match host_content.native_host() {
            Some(native_host) => native_host,
            None => panic!("NativeHost cannot be null"),
        };
        let input_element = match host_content.input_element() {
            Some(input_element) => input_element,
            None => panic!("InputElement cannot be null"),
        };

        let browser_top_level = BrowserTopLevelImpl::new(host.clone(), native_controls_container, input_element);
        let surface = browser_top_level.surface();
        let top_level_impl: Rc<dyn ITopLevelImpl> = browser_top_level;
        let top_level = EmbeddableControlRoot::with_impl(top_level_impl);

        top_level.prepare();
        top_level.add_handler(InputElement::got_focus_event(), {
            let host = host.clone();
            move |_: &Interactive, _: &FocusChangedEventArgs| input_helper::focus_element(&host)
        });
        top_level.renderer().start(); // TODO: use Start+StopRenderer() instead.
        // Differs from upstream, which closes the splash screen at the first
        // animation frame of the top-level, on the thread that draws the
        // view. That animation frame comes before the first frame is drawn,
        // and with a render thread it is not the thread of the page that
        // draws: the first frame of the worker may be seconds away, during
        // which the page would show an empty canvas that input cannot hit.
        // The splash screen is closed when the first frame has been drawn
        // to the canvas of the view, whichever thread drew it.
        if let Some(surface) = surface {
            surface.shared().on_first_frame(Box::new(move || {
                // Try to get local splash-screen of the specific host.
                // If couldn't find - get global one by ID for compatibility.
                let splash = dom_helper::get_elements_by_class_name("ferroui-splash", &host).or_else(|| {
                    dom_helper::get_element_by_id("ferroui-splash", &BrowserWindowingPlatform::global_this())
                });
                if let Some(splash) = splash {
                    dom_helper::add_css_class(&splash, "splash-close");
                }
            }));
        }

        Rc::new(Self { top_level })
    }

    /// The content of the view.
    ///
    /// # Panics
    /// Panics when the content of the top-level was set to something that
    /// is not a control.
    pub fn content(&self) -> Option<Ref<Control>> {
        let content = self.top_level.content()?;
        match Control::from_boxed(&content) {
            Some(control) => Some(control),
            None => panic!("Unable to cast the content of the view to a control."),
        }
    }

    /// Sets the content of the view.
    pub fn set_content(&self, value: Option<Ref<Control>>) {
        self.top_level.set_content(value.map(Control::boxed));
    }

    pub(crate) fn top_level(&self) -> Ref<TopLevel> {
        self.top_level.clone().upcast()
    }

    /// Closes the view: its top-level is disposed, which releases the
    /// surface of the canvas (on the thread that renders, the render target
    /// of the canvas and what drew to it) and stops the renderer of the
    /// view. The elements of the view stay in the page. The view cannot be
    /// used afterwards.
    ///
    /// Not from upstream, whose view has no way to be closed (the top-level
    /// under it is disposable there as here). A page that shows and removes
    /// views would otherwise keep the canvas and the graphics context of
    /// every view it ever had.
    pub fn dispose(&self) {
        self.top_level.dispose();
    }
}
