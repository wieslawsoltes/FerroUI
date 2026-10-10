//! Port of `Pages/ScreensPage.xaml.cs`: the class of the document `Pages/ScreensPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Visual, VisualTreeAttachmentEventArgs, WeakRef};
use ferroui_controls::platform::{IPlatformHandle, Screen};
use ferroui_controls::{TextBox, TopLevel, UserControl, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct ScreensPage {
    base: UserControl,
    last_screen: RefCell<Option<Rc<Screen>>>,
    on_screen_changed_counter: Cell<i32>,
    /// `_lastWindow`: the window the page is attached to, which holds the page: held weakly.
    #[allow(dead_code)]
    last_window: RefCell<Option<WeakRef<Window>>>,
    /// The subscription to the change of the screens of the window (`Screens.Changed += ..`),
    /// which `Screens.Changed -= ..` ends.
    screens_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

user_control_class!(ScreensPage);
ferro_class_info!(ScreensPage {
    new: ScreensPage::new,
    markup: {
        methods: [
            fn OnAttachedToVisualTree(Option<BoxedValue>, VisualTreeAttachmentEventArgs) =>
                |this: &Ref<ScreensPage>, sender: Option<BoxedValue>, e: VisualTreeAttachmentEventArgs| {
                    this.on_attached_to_visual_tree(&sender, &e)
                },
            fn OnDetachedFromVisualTree(Option<BoxedValue>, VisualTreeAttachmentEventArgs) =>
                |this: &Ref<ScreensPage>, sender: Option<BoxedValue>, e: VisualTreeAttachmentEventArgs| {
                    this.on_detached_from_visual_tree(&sender, &e)
                },
            fn ScreenRefresh_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ScreensPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.screen_refresh_click(&sender, e.as_routed_event_args())
                },
            fn UpdateViewOnly_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ScreensPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.update_view_only_on_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ScreensPage, "/Pages/ScreensPage.xaml");

/// `ReferenceEquals(a, b)` of two screens.
fn reference_equals(a: &Option<Rc<Screen>>, b: &Option<Rc<Screen>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// The text of a boolean, as the managed original writes it.
fn boolean_text(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

/// `handle.ToString()`: the text of a platform handle.
fn handle_text(handle: &Rc<dyn IPlatformHandle>) -> String {
    format!("PlatformHandle {{ {} = {} }}", handle.handle_descriptor().unwrap_or_default(), handle.handle())
}

impl ScreensPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            last_screen: RefCell::new(None),
            on_screen_changed_counter: Cell::new(0),
            last_window: RefCell::new(None),
            screens_changed: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn text_box(&self, name: &str) -> Ref<TextBox> {
        self.get_control::<TextBox>(name)
    }

    /// The window the page is attached to.
    ///
    /// # Panics
    /// Panics if the page is not attached to a window (the internal exception of the managed
    /// original).
    fn window(&self) -> Ref<Window> {
        let visual: &Visual = self;
        TopLevel::get_top_level(Some(visual))
            .and_then(|top_level| top_level.cast::<Window>())
            .unwrap_or_else(|| panic!("ScreensPage is not attached to a Window."))
    }

    fn on_attached_to_visual_tree(&self, _sender: &Option<BoxedValue>, _e: &VisualTreeAttachmentEventArgs) {
        let last_window = self.window();
        *self.last_window.borrow_mut() = Some(last_window.downgrade());
        // The screens belong to the window, which holds the page: the handler holds the page
        // weakly.
        let weak = self.to_ref().downgrade();
        let subscription = last_window.screens().changed(move || {
            if let Some(this) = weak.upgrade() {
                this.on_screen_changed();
            }
        });
        *self.screens_changed.borrow_mut() = Some(subscription);

        self.on_screen_changed_counter.set(0);
        self.text_box("ScreenOnChangedCounter").set_text(Some(&self.on_screen_changed_counter.get().to_string()));
    }

    fn on_detached_from_visual_tree(&self, _sender: &Option<BoxedValue>, _e: &VisualTreeAttachmentEventArgs) {
        let subscription = self.screens_changed.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        *self.last_window.borrow_mut() = None;
    }

    fn on_screen_changed(&self) {
        self.on_screen_changed_counter.set(self.on_screen_changed_counter.get() + 1);
        self.text_box("ScreenOnChangedCounter").set_text(Some(&self.on_screen_changed_counter.get().to_string()));
    }

    /// Writes what `screen` says about itself into the text boxes of the page.
    fn show_screen(&self, screen: &Option<Rc<Screen>>) {
        let screen = screen.as_ref();
        self.text_box("ScreenName").set_text(screen.and_then(|screen| screen.display_name()).as_deref());
        self.text_box("ScreenHandle").set_text(
            screen.and_then(|screen| screen.try_get_platform_handle()).map(|handle| handle_text(&handle)).as_deref(),
        );
        self.text_box("ScreenBounds").set_text(screen.map(|screen| screen.bounds().to_string()).as_deref());
        self.text_box("ScreenWorkArea").set_text(screen.map(|screen| screen.working_area().to_string()).as_deref());
        self.text_box("ScreenScaling").set_text(screen.map(|screen| screen.scaling().to_string()).as_deref());
        self.text_box("ScreenOrientation")
            .set_text(screen.map(|screen| format!("{:?}", screen.current_orientation())).as_deref());
    }

    fn screen_refresh_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let window = self.window();
        let last_screen = self.last_screen.borrow().clone();
        let screen = window.screens().screen_from_window(&window);
        *self.last_screen.borrow_mut() = screen.clone();
        self.show_screen(&screen);
        self.text_box("ScreenSameReference").set_text(Some(boolean_text(reference_equals(&last_screen, &screen))));
    }

    fn update_view_only_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let screen = self.last_screen.borrow().clone();
        self.show_screen(&screen);
        let last_screen = self.last_screen.borrow().clone();
        self.text_box("ScreenSameReference").set_text(Some(boolean_text(reference_equals(&last_screen, &screen))));
    }
}
