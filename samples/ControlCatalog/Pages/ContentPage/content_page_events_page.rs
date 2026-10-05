//! Port of `Pages/ContentPage/ContentPageEventsPage.xaml.cs`: the class of the document
//! `Pages/ContentPage/ContentPageEventsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextAlignment, TextWrapping};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{
    CheckBox, ContentPage, Control, ItemsControl, ItemsSource, NavigationPage, ScrollViewer, StackPanel, TextBlock,
    UserControl,
};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// The state of a [`Delay`]: whether its time has passed, and who waits.
#[derive(Default)]
struct DelayState {
    elapsed: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

/// `Task.Delay(duration)`: completes when a dispatcher timer of `duration`
/// has ticked.
struct Delay(Rc<DelayState>);

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.elapsed.get() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn delay(duration: Duration) -> Delay {
    let state = Rc::new(DelayState::default());
    let timer_state = state.clone();
    DispatcherTimer::run_once(
        move || {
            timer_state.elapsed.set(true);
            let waker = timer_state.waker.borrow_mut().take();
            if let Some(waker) = waker {
                waker.wake();
            }
        },
        duration,
        DispatcherPriority::DEFAULT,
    );
    Delay(state)
}

#[repr(C)]
pub struct ContentPageEventsPage {
    base: UserControl,
    page_count: Cell<i32>,
    log_items: FerroList<String>,
}

user_control_class!(ContentPageEventsPage);
ferro_class_info!(ContentPageEventsPage {
    new: ContentPageEventsPage::new,
    markup: {
        methods: [
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ContentPageEventsPage, "/Pages/ContentPage/ContentPageEventsPage.xaml");

impl ContentPageEventsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), page_count: Cell::new(0), log_items: FerroList::new() }
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

    fn block_nav_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("BlockNavCheck")
    }

    fn log_scroll_viewer(&self) -> Ref<ScrollViewer> {
        self.get_control::<ScrollViewer>("LogScrollViewer")
    }

    fn event_log_items(&self) -> Ref<ItemsControl> {
        self.get_control::<ItemsControl>("EventLogItems")
    }

    fn demo_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.event_log_items().set_items_source(Some(ItemsSource::new(Rc::new(self.log_items.clone()))));

        let root = Self::make_page("Root", "Navigate to see lifecycle events in the log below.");
        self.subscribe_events(&root);
        let _ = self.demo_nav().push_async(root);
    }

    fn subscribe_events(&self, page: &Ref<ContentPage>) {
        // The handlers belong to the page they are added to, which the navigation page of this
        // control holds: they hold this control and that page weakly.
        let header = |page: &ContentPage| {
            page.header().as_ref().map_or_else(String::new, |header| ValueTypes::to_display_string(Some(header)))
        };

        let (weak, weak_page) = (self.to_ref().downgrade(), page.downgrade());
        page.navigated_to(move |e| {
            if let (Some(this), Some(page)) = (weak.upgrade(), weak_page.upgrade()) {
                this.add_log(&format!("[{}] NavigatedTo (type: {:?})", header(&page), e.navigation_type()));
            }
        });
        let (weak, weak_page) = (self.to_ref().downgrade(), page.downgrade());
        page.navigated_from(move |_| {
            if let (Some(this), Some(page)) = (weak.upgrade(), weak_page.upgrade()) {
                this.add_log(&format!("[{}] NavigatedFrom", header(&page)));
            }
        });
        let (weak, weak_page) = (self.to_ref().downgrade(), page.downgrade());
        page.navigating(move |args| {
            let (weak, weak_page, args) = (weak.clone(), weak_page.clone(), args.clone());
            Box::pin(async move {
                let (Some(this), Some(page)) = (weak.upgrade(), weak_page.upgrade()) else {
                    return;
                };
                if this.block_nav_check().is_checked() == Some(true) {
                    args.set_cancel(true);
                    this.add_log(&format!("[{}] Navigating \u{2014} BLOCKED", header(&page)));
                    delay(Duration::from_millis(600)).await;
                    args.set_cancel(false);
                    this.add_log(&format!("[{}] Navigating \u{2014} unblocked", header(&page)));
                } else {
                    this.add_log(&format!("[{}] Navigating", header(&page)));
                }
            })
        });
    }

    /// `async void`: nothing follows the push.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.page_count.set(self.page_count.get() + 1);
        let page =
            Self::make_page(&format!("Page {}", self.page_count.get()), "Navigate back to see Navigating/NavigatedFrom.");
        self.subscribe_events(&page);
        let _ = self.demo_nav().push_async(page);
    }

    /// `async void`: nothing follows the pop.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_nav = self.demo_nav();
        let _ = start_async(async move {
            let _ = demo_nav.pop_async().await;
        });
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.log_items.clear();
    }

    fn add_log(&self, message: &str) {
        self.log_items.add(message.to_string());
        self.log_scroll_viewer().scroll_to_end();
    }

    fn make_page(header: &str, body: &str) -> Ref<ContentPage> {
        let header_text = TextBlock::new();
        header_text.set_text(Some(header));
        header_text.set_font_size(24.0);
        header_text.set_font_weight(FontWeight::Bold);
        header_text.set_horizontal_alignment(HorizontalAlignment::Center);

        let body_text = TextBlock::new();
        body_text.set_text(Some(body));
        body_text.set_font_size(13.0);
        body_text.set_opacity(0.6);
        body_text.set_text_wrapping(TextWrapping::Wrap);
        body_text.set_text_alignment(TextAlignment::Center);
        body_text.set_horizontal_alignment(HorizontalAlignment::Center);
        body_text.set_max_width(260.0);

        let content = StackPanel::new();
        content.set_horizontal_alignment(HorizontalAlignment::Center);
        content.set_vertical_alignment(VerticalAlignment::Center);
        content.set_spacing(8.0);
        content.children().add(header_text);
        content.children().add(body_text);

        let page = ContentPage::new();
        page.set_header(Some(Rc::new(header.to_string()) as BoxedValue));
        page.set_content(Some(Control::boxed(content)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        page
    }
}
