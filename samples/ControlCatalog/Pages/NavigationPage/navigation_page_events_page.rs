//! Port of `Pages/NavigationPage/NavigationPageEventsPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageEventsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{value_text, NavigationDemoHelper};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::media::{FontFamily, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Thickness};
use ferroui_controls::{ContentPage, NavigationPage, Page, StackPanel, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

/// `$"{page?.Header}"`: the text of the header of a page, empty without a header.
fn header_text(page: &Page) -> String {
    value_text(&page.header()).unwrap_or_default()
}

#[repr(C)]
pub struct NavigationPageEventsPage {
    base: UserControl,
    initialized: Cell<bool>,
    page_count: Cell<i32>,
}

user_control_class!(NavigationPageEventsPage);
ferro_class_info!(NavigationPageEventsPage {
    new: NavigationPageEventsPage::new,
    markup: {
        methods: [
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnPopToRoot(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_to_root(&sender, e.as_routed_event_args())
                },
            fn OnInsertPage(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_insert_page(&sender, e.as_routed_event_args())
                },
            fn OnRemovePage(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_remove_page(&sender, e.as_routed_event_args())
                },
            fn OnPushModal(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_modal(&sender, e.as_routed_event_args())
                },
            fn OnPopModal(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_modal(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageEventsPage, "/Pages/NavigationPage/NavigationPageEventsPage.xaml");

impl NavigationPageEventsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            initialized: Cell::new(false),
            page_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the control itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn demo_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    fn log_panel(&self) -> Ref<StackPanel> {
        self.get_control::<StackPanel>("LogPanel")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);

        // The handlers belong to a child of the control: they hold the control weakly.
        let demo_nav = self.demo_nav();
        let log = |message: fn(String) -> String| {
            let weak = self.to_ref().downgrade();
            move |header: String| {
                if let Some(this) = weak.upgrade() {
                    this.add_log(&message(header));
                }
            }
        };
        let pushed = log(|header| format!("Pushed \u{2192} {header}"));
        demo_nav.pushed(move |ev| pushed(header_text(&ev.page())));
        let popped = log(|header| format!("Popped \u{2190} {header}"));
        demo_nav.popped(move |ev| popped(header_text(&ev.page())));
        let popped_to_root = log(|_| String::from("PoppedToRoot"));
        demo_nav.popped_to_root(move |_ev| popped_to_root(String::new()));
        let page_inserted = log(|header| format!("PageInserted: {header}"));
        demo_nav.page_inserted(move |ev| page_inserted(header_text(&ev.page())));
        let page_removed = log(|header| format!("PageRemoved: {header}"));
        demo_nav.page_removed(move |ev| page_removed(header_text(&ev.page())));
        let modal_pushed = log(|header| format!("ModalPushed \u{2192} {header}"));
        demo_nav.modal_pushed(move |ev| modal_pushed(header_text(&ev.modal())));
        let modal_popped = log(|header| format!("ModalPopped \u{2190} {header}"));
        demo_nav.modal_popped(move |ev| modal_popped(header_text(&ev.modal())));

        let root = NavigationDemoHelper::make_page(
            "Home",
            "Push pages and watch the event log.\nAll navigation events are captured.",
            0,
        );
        self.subscribe_page(&root);
        drop(demo_nav.push_async_with_transition(root, None));
    }

    fn subscribe_page(&self, page: &Ref<ContentPage>) {
        // The handlers belong to the page and to a descendant of the control: they hold both weakly.
        let log = |event: &'static str| {
            let weak = self.to_ref().downgrade();
            let weak_page = page.downgrade();
            move || {
                if let (Some(this), Some(page)) = (weak.upgrade(), weak_page.upgrade()) {
                    this.add_log(&format!("[{}] {event}", header_text(&page)));
                }
            }
        };
        let navigated_to = log("NavigatedTo");
        page.navigated_to(move |_e| navigated_to());
        let navigated_from = log("NavigatedFrom");
        page.navigated_from(move |_e| navigated_from());
        let navigating = log("Navigating");
        page.navigating(move |_args| {
            navigating();
            Box::pin(async {})
        });
    }

    /// `async void`: nothing follows the push.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();
        let page =
            NavigationDemoHelper::make_page(&format!("Page {page_count}"), "Navigate back to see events.", page_count);
        self.subscribe_page(&page);
        drop(self.demo_nav().push_async(page));
    }

    /// `async void`: nothing follows the pop.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.demo_nav().pop_async());
    }

    /// `async void`: nothing follows the pop.
    fn on_pop_to_root(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.demo_nav().pop_to_root_async());
    }

    fn on_insert_page(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_nav = self.demo_nav();
        let Some(current_page) = demo_nav.current_page() else {
            return;
        };
        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();
        let page = NavigationDemoHelper::make_page(
            &format!("Inserted {page_count}"),
            "Inserted below the current page.",
            page_count,
        );
        self.subscribe_page(&page);
        demo_nav.insert_page(page, current_page);
    }

    fn on_remove_page(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_nav = self.demo_nav();
        let stack = demo_nav.navigation_stack();
        if stack.len() < 2 {
            return;
        }
        demo_nav.remove_page(stack[stack.len() - 2].clone());
    }

    /// `async void`: nothing follows the push.
    fn on_push_modal(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();
        let page = NavigationDemoHelper::make_page(
            &format!("Modal {page_count}"),
            "Dismiss to see ModalPopped event.",
            page_count,
        );
        self.subscribe_page(&page);
        drop(self.demo_nav().push_modal_async(page));
    }

    /// `async void`: nothing follows the pop.
    fn on_pop_modal(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.demo_nav().pop_modal_async());
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.log_panel().children().clear();
    }

    fn add_log(&self, message: &str) {
        let text = TextBlock::new();
        text.set_text(Some(message));
        text.set_font_family(FontFamily::new("Cascadia Code,Consolas,Menlo,monospace"));
        text.set_font_size(11.0);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_margin(Thickness::symmetric(0.0, 1.0));
        self.log_panel().children().insert(0, text);
    }
}
