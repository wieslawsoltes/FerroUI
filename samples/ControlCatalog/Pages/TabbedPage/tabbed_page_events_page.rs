//! Port of `Pages/TabbedPage/TabbedPageEventsPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageEventsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, value_text};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    ContentPage, Control, Page, PageSelectionChangedEventArgs, ScrollViewer, TabbedPage, TextBlock, UserControl,
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
pub struct TabbedPageEventsPage {
    base: UserControl,
    log: RefCell<Vec<String>>,
}

user_control_class!(TabbedPageEventsPage);
ferro_class_info!(TabbedPageEventsPage {
    new: TabbedPageEventsPage::new,
    markup: {
        methods: [
            fn OnSelectNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_select_next(&sender, e.as_routed_event_args())
                },
            fn OnSelectPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_select_previous(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPageEventsPage, "/Pages/TabbedPage/TabbedPageEventsPage.xaml");

impl TabbedPageEventsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), log: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn log_scroll_viewer(&self) -> Ref<ScrollViewer> {
        self.get_control::<ScrollViewer>("LogScrollViewer")
    }

    fn event_log(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("EventLog")
    }

    fn demo_tabs(&self) -> Ref<TabbedPage> {
        self.get_control::<TabbedPage>("DemoTabs")
    }

    /// # Panics
    /// Panics if the tabbed page has no list of pages (a null reference in the managed
    /// original).
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let demo_tabs = self.demo_tabs();
        let pages_list = demo_tabs.pages().expect("the pages of the tabbed page");
        let page_names = ["Home", "Explore", "Library", "Profile"];

        for name in page_names {
            let text = TextBlock::new();
            text.set_text(Some(&format!("{name} tab content")));
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);
            text.set_font_size(18.0);
            text.set_opacity(0.7);

            let page = ContentPage::new();
            page.set_header(Some(boxed_text(name)));
            page.set_content(Some(Control::boxed(text)));

            // The pages end up in the tabbed page of this control: their handlers hold it weakly.
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

            pages_list.add(page.upcast());
        }

        let weak = self.to_ref().downgrade();
        demo_tabs.selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        });
    }

    fn on_selection_changed(&self, _sender: &Interactive, e: &PageSelectionChangedEventArgs) {
        self.append_log(&format!(
            "SelectionChanged: {} \u{2192} {}",
            header_text(e.previous_page()),
            header_text(e.current_page())
        ));
    }

    fn on_select_next(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_tabs = self.demo_tabs();
        let next = demo_tabs.selected_index() + 1;
        if demo_tabs.pages().is_some_and(|pages| next < pages.count() as i32) {
            demo_tabs.set_selected_index(next);
        }
    }

    fn on_select_previous(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_tabs = self.demo_tabs();
        let prev = demo_tabs.selected_index() - 1;
        if prev >= 0 {
            demo_tabs.set_selected_index(prev);
        }
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.log.borrow_mut().clear();
        self.event_log().set_text(Some(""));
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

        self.event_log().set_text(Some(&text));
        self.log_scroll_viewer().scroll_to_end();
    }
}
