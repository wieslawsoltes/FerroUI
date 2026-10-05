//! Port of `Pages/CarouselPage/CarouselPageEventsPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselPageEventsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, value_text, NavigationDemoHelper};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::FontWeight;
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    CarouselPage, ContentPage, Control, Page, PageSelectionChangedEventArgs, ScrollViewer, SelectingMultiPage,
    TextBlock, UserControl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// `Environment.NewLine`.
const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// `(page as ContentPage)?.Header ?? "—"`, as text.
fn header_text(page: Option<Ref<Page>>) -> String {
    page.and_then(|page| page.cast::<ContentPage>())
        .and_then(|page| value_text(&page.header()))
        .unwrap_or_else(|| String::from("\u{2014}"))
}

#[repr(C)]
pub struct CarouselPageEventsPage {
    base: UserControl,
    log: RefCell<Vec<String>>,
    /// The subscriptions `OnLoaded` adds and `OnUnloaded` removes.
    selection_changed: RefCell<Vec<RoutedEventHandlerToken>>,
}

user_control_class!(CarouselPageEventsPage);
ferro_class_info!(CarouselPageEventsPage {
    new: CarouselPageEventsPage::new,
    markup: {
        methods: [
            fn OnPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_previous(&sender, e.as_routed_event_args())
                },
            fn OnNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_next(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CarouselPageEventsPage, "/Pages/CarouselPage/CarouselPageEventsPage.xaml");

impl CarouselPageEventsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            log: RefCell::new(Vec::new()),
            selection_changed: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers of the events of the page itself hold it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        let weak = this.downgrade();
        this.unloaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_unloaded(sender, e);
            }
        });
        this
    }

    fn demo_carousel(&self) -> Ref<CarouselPage> {
        self.get_control::<CarouselPage>("DemoCarousel")
    }

    /// # Panics
    /// Panics if the carousel page has no list of pages (a null reference in the managed
    /// original).
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let demo_carousel = self.demo_carousel();
        let page_names = ["Home", "Explore", "Library", "Profile"];
        for (i, name) in page_names.into_iter().enumerate() {
            let text = TextBlock::new();
            text.set_text(Some(name));
            text.set_font_size(28.0);
            text.set_font_weight(FontWeight::Bold);
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);

            let page = ContentPage::new();
            page.set_header(Some(boxed_text(name)));
            page.set_background(Some(NavigationDemoHelper::get_page_brush(i as i32)));
            page.set_content(Some(Control::boxed(text)));
            page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            page.set_vertical_content_alignment(VerticalAlignment::Stretch);

            // The pages end up in the carousel page of this control: their handlers hold it weakly.
            let weak = self.to_ref().downgrade();
            page.navigated_to(move |args| {
                if let Some(this) = weak.upgrade() {
                    this.append_log(&format!("NavigatedTo: {name} (from {})", header_text(args.previous_page())));
                }
            });
            let weak = self.to_ref().downgrade();
            page.navigated_from(move |args| {
                if let Some(this) = weak.upgrade() {
                    this.append_log(&format!(
                        "NavigatedFrom: {name} (to {})",
                        header_text(args.destination_page())
                    ));
                }
            });

            demo_carousel.pages().expect("the pages of the carousel page").add(page.upcast());
        }

        let weak = self.to_ref().downgrade();
        self.selection_changed.borrow_mut().push(demo_carousel.selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        }));
    }

    fn on_selection_changed(&self, _sender: &Interactive, e: &PageSelectionChangedEventArgs) {
        self.append_log(&format!(
            "SelectionChanged: {} \u{2192} {}",
            header_text(e.previous_page()),
            header_text(e.current_page())
        ));
    }

    fn on_previous(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_carousel = self.demo_carousel();
        if demo_carousel.selected_index() > 0 {
            demo_carousel.set_selected_index(demo_carousel.selected_index() - 1);
        }
    }

    /// # Panics
    /// Panics if the carousel page has no list of pages (a null reference in the managed
    /// original).
    fn on_next(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_carousel = self.demo_carousel();
        let page_count = demo_carousel.pages().expect("the pages of the carousel page").count() as i32;
        if demo_carousel.selected_index() < page_count - 1 {
            demo_carousel.set_selected_index(demo_carousel.selected_index() + 1);
        }
    }

    fn on_unloaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        // One subscription is removed, as `-=` removes one.
        let token = self.selection_changed.borrow_mut().pop();
        if let Some(token) = token {
            self.demo_carousel().remove_handler(SelectingMultiPage::selection_changed_event(), token);
        }
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.log.borrow_mut().clear();
        self.get_control::<TextBlock>("EventLog").set_text(Some(""));
    }

    fn append_log(&self, message: &str) {
        let timestamp = DateTime::now().to_string_with("HH:mm:ss.fff");
        let text = {
            let mut log = self.log.borrow_mut();
            log.push(format!("[{timestamp}] {message}"));
            if log.len() > 50 {
                log.remove(0);
            }
            log.join(NEW_LINE)
        };

        self.get_control::<TextBlock>("EventLog").set_text(Some(&text));
        self.get_control::<ScrollViewer>("LogScrollViewer").scroll_to_end();
    }
}
