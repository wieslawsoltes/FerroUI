//! Port of `Pages/DrawerPage/DrawerPageEventsPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageEventsPage.xaml`.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{boxed_text, value_text};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextAlignment, TextWrapping};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, CheckBox, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt, DrawerClosingEventArgs,
    DrawerPage, StackPanel, TextBlock, UserControl,
};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct DrawerPageEventsPage {
    base: UserControl,
    /// The pages of the sections by name, in the order they were added.
    section_pages: RefCell<Vec<(&'static str, Ref<ContentPage>)>>,
    /// The subscriptions `OnLoaded` adds and `OnUnloaded` removes: the opened, the closing
    /// and the closed event of the drawer.
    drawer_handlers: RefCell<Vec<[RoutedEventHandlerToken; 3]>>,
}

ferro_class!(DrawerPageEventsPage: UserControl);
ferro_impl_classes!(
    DrawerPageEventsPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(DrawerPageEventsPage {
    new: DrawerPageEventsPage::new,
    markup: {
        methods: [
            fn OnToggle(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_toggle(&sender, e.as_routed_event_args())
                },
            fn OnSelectSection(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_select_section(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DrawerPageEventsPage, "/Pages/DrawerPage/DrawerPageEventsPage.xaml");

impl ControlImpl for DrawerPageEventsPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        // The handlers belong to a child of the page: they hold the page weakly.
        let demo_drawer = this.demo_drawer();
        let weak = this.to_ref().downgrade();
        let opened = demo_drawer.opened(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_drawer_opened(sender, e);
            }
        });
        let weak = this.to_ref().downgrade();
        let closing = demo_drawer.closing(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_closing(sender, e);
            }
        });
        let weak = this.to_ref().downgrade();
        let closed = demo_drawer.closed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_drawer_closed(sender, e);
            }
        });
        this.drawer_handlers.borrow_mut().push([opened, closing, closed]);

        // The content is set here so the initial NavigatedTo events fire (the page has no
        // visual root in the constructor, which suppresses the lifecycle events).
        let home = this.section_page("Home").expect("the page of the section Home");
        demo_drawer.set_content(Some(Control::boxed(home)));
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_unloaded(this, e);

        // One subscription of each handler is removed, as `-=` removes one.
        let handlers = this.drawer_handlers.borrow_mut().pop();
        if let Some([opened, closing, closed]) = handlers {
            let demo_drawer = this.demo_drawer();
            demo_drawer.remove_handler(DrawerPage::opened_event(), opened);
            demo_drawer.remove_handler(DrawerPage::closing_event(), closing);
            demo_drawer.remove_handler(DrawerPage::closed_event(), closed);
        }
    }
}

/// `sender as Button`.
fn as_button(sender: &Option<BoxedValue>) -> Option<Ref<Button>> {
    sender.as_ref().and_then(|sender| ValueTypes::as_object(&**sender)).and_then(|sender| sender.cast::<Button>())
}

impl DrawerPageEventsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            section_pages: RefCell::new(Vec::new()),
            drawer_handlers: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.section_pages.borrow_mut() = vec![
            ("Home", Self::create_section_page("Home")),
            ("Profile", Self::create_section_page("Profile")),
            ("Settings", Self::create_section_page("Settings")),
        ];

        for (name, page) in this.section_pages.borrow().iter() {
            // The pages are held by this control: their handlers hold it weakly.
            let label = *name;
            let weak = this.downgrade();
            page.navigated_to(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.log(&format!("{label}: NavigatedTo"));
                }
            });
            let weak = this.downgrade();
            page.navigated_from(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.log(&format!("{label}: NavigatedFrom"));
                }
            });
        }
        this
    }

    fn cancel_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("CancelCheck")
    }

    fn event_log(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("EventLog")
    }

    fn demo_drawer(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DemoDrawer")
    }

    /// `_sectionPages.TryGetValue(section, out page)`.
    fn section_page(&self, section: &str) -> Option<Ref<ContentPage>> {
        self.section_pages.borrow().iter().find(|(name, _)| *name == section).map(|(_, page)| page.clone())
    }

    fn on_drawer_opened(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.log("Opened");
    }

    fn on_drawer_closed(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.log("Closed");
    }

    fn on_toggle(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_drawer = self.demo_drawer();
        demo_drawer.set_is_open(!demo_drawer.is_open());
    }

    fn on_closing(&self, _sender: &Interactive, e: &DrawerClosingEventArgs) {
        if self.cancel_check().is_checked() == Some(true) {
            e.set_cancel(true);
            self.cancel_check().set_is_checked(Some(false));
            self.log("Closing  \u{2192}  cancelled");
        } else {
            self.log("Closing");
        }
    }

    fn on_select_section(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(btn) = as_button(sender) else {
            return;
        };
        let section = value_text(&btn.tag()).unwrap_or_else(|| String::from("Home"));
        let Some(page) = self.section_page(&section) else {
            return;
        };

        let demo_drawer = self.demo_drawer();
        let content = demo_drawer.content().and_then(|content| Control::from_boxed(&content));
        if content.is_some_and(|content| content == page) {
            demo_drawer.set_is_open(false);
            return;
        }

        self.log(&format!("\u{2192} {section}"));
        demo_drawer.set_content(Some(Control::boxed(page)));
        demo_drawer.set_is_open(false);
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.event_log().set_text(Some(""));
    }

    fn log(&self, message: &str) {
        let event_log = self.event_log();
        event_log.set_text(Some(&format!("{message}\n{}", event_log.text().unwrap_or_default())));
    }

    fn create_section_page(header: &str) -> Ref<ContentPage> {
        let title = TextBlock::new();
        title.set_text(Some(header));
        title.set_font_size(24.0);
        title.set_font_weight(FontWeight::SemiBold);
        title.set_horizontal_alignment(HorizontalAlignment::Center);

        let hint = TextBlock::new();
        hint.set_text(Some("Tap a drawer item to navigate.\nWatch the event log in the panel."));
        hint.set_text_wrapping(TextWrapping::Wrap);
        hint.set_opacity(0.6);
        hint.set_text_alignment(TextAlignment::Center);
        hint.set_font_size(13.0);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(8.0);
        panel.children().add(title);
        panel.children().add(hint);

        let page = ContentPage::new();
        page.set_header(Some(boxed_text(header)));
        page.set_content(Some(Control::boxed(panel)));
        page
    }
}
