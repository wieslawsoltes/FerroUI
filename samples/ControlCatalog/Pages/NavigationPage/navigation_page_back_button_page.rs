//! Port of `Pages/NavigationPage/NavigationPageBackButtonPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageBackButtonPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, value_text, NavigationDemoHelper};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::{Color, FontFamily, SolidColorBrush, TextWrapping};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    CheckBox, ContentPage, Control, NavigationPage, NavigationType, Page, RadioButton, StackPanel, TextBlock, TextBox,
    UserControl,
};
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

/// `$"{page?.Header}"`: the text of the header of a page, empty without a header.
fn header_text(page: &Page) -> String {
    value_text(&page.header()).unwrap_or_default()
}

#[repr(C)]
pub struct NavigationPageBackButtonPage {
    base: UserControl,
    initialized: Cell<bool>,
    push_count: Cell<i32>,
    /// Whether `InitializeComponent` has returned: the named elements are the
    /// fields it assigns, null while the document loads.
    component_initialized: Cell<bool>,
}

user_control_class!(NavigationPageBackButtonPage);
ferro_class_info!(NavigationPageBackButtonPage {
    new: NavigationPageBackButtonPage::new,
    markup: {
        methods: [
            fn OnGlobalBackButtonChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_global_back_button_changed(&sender, e.as_routed_event_args())
                },
            fn OnPushStandard(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_standard(&sender, e.as_routed_event_args())
                },
            fn OnPushNoBack(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_no_back(&sender, e.as_routed_event_args())
                },
            fn OnPushDisabledBack(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_disabled_back(&sender, e.as_routed_event_args())
                },
            fn OnPushCustomText(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_custom_text(&sender, e.as_routed_event_args())
                },
            fn OnPushCustomIcon(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_custom_icon(&sender, e.as_routed_event_args())
                },
            fn OnPushIconTextBack(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_icon_text_back(&sender, e.as_routed_event_args())
                },
            fn OnPushGuarded(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_guarded(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnPopToRoot(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_to_root(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageBackButtonPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageBackButtonPage, "/Pages/NavigationPage/NavigationPageBackButtonPage.xaml");

impl NavigationPageBackButtonPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            initialized: Cell::new(false),
            push_count: Cell::new(0),
            component_initialized: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);

        // The handler of an event of the control itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn demo_nav(&self) -> Option<Ref<NavigationPage>> {
        self.component_initialized.get().then(|| self.get_control::<NavigationPage>("DemoNav"))
    }

    /// `DemoNav` where the original dereferences it unconditionally (the
    /// handlers of the buttons, which run after `InitializeComponent`).
    fn nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    fn is_back_button_visible_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("IsBackButtonVisibleCheck")
    }

    fn back_content_input(&self) -> Option<Ref<TextBox>> {
        self.component_initialized.get().then(|| self.get_control::<TextBox>("BackContentInput"))
    }

    fn defer_radio(&self) -> Option<Ref<RadioButton>> {
        self.component_initialized.get().then(|| self.get_control::<RadioButton>("DeferRadio"))
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
        let demo_nav = self.nav();
        let weak = self.to_ref().downgrade();
        demo_nav.pushed(move |ev| {
            if let Some(this) = weak.upgrade() {
                this.add_log(&format!("Pushed: \"{}\"", header_text(&ev.page())));
            }
        });
        let weak = self.to_ref().downgrade();
        demo_nav.popped(move |ev| {
            if let Some(this) = weak.upgrade() {
                this.add_log(&format!("Popped: \"{}\"", header_text(&ev.page())));
            }
        });

        let page = self.create_page(
            "Home",
            "This is the root page.\nNo back button is shown here.\n\nPush pages from the config panel\nto explore back button behaviors.",
            None,
        );
        drop(demo_nav.push_async_with_transition(page, None));
    }

    fn on_global_back_button_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };
        demo_nav.set_is_back_button_visible(self.is_back_button_visible_check().is_checked() == Some(true));
        self.add_log(&format!("IsBackButtonVisible={}", if demo_nav.is_back_button_visible() { "True" } else { "False" }));
    }

    /// `async void`: nothing follows the push.
    fn on_push_standard(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let page = self.create_page(
            &format!("Page {}", self.push_count.get() + 1),
            "Standard page with default back arrow.",
            None,
        );
        drop(self.nav().push_async(page));
    }

    /// `async void`: the log line follows the push.
    fn on_push_no_back(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let page = self.create_page(
            &format!("No Back #{}", self.push_count.get() + 1),
            "IsBackButtonVisible = false\n\nThe back arrow is hidden.\nUse the Pop button to go back.",
            None,
        );
        NavigationPage::set_has_back_button(&page, false);
        self.push_then_log(page, |header| format!("HasBackButton=false on \"{header}\""));
    }

    /// `async void`: the log line follows the push.
    fn on_push_disabled_back(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let page = self.create_page(
            &format!("Disabled Back #{}", self.push_count.get() + 1),
            "IsBackButtonEnabled = false\n\nThe back arrow is visible but disabled.\nUse the Pop button to go back.",
            None,
        );
        NavigationPage::set_is_back_button_enabled(&page, false);
        self.push_then_log(page, |header| format!("IsBackButtonEnabled=false on \"{header}\""));
    }

    /// `async void`: the log line follows the push.
    fn on_push_custom_text(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let input = self.back_content_input().and_then(|input| input.text());
        let text = match input {
            Some(text) if !text.trim().is_empty() => text,
            _ => String::from("Cancel"),
        };
        let page = self.create_page(
            &format!("Text Back #{}", self.push_count.get() + 1),
            &format!("BackButtonContent = \"{text}\"\n\nThe back button shows custom text."),
            None,
        );
        NavigationPage::set_back_button_content(&page, Some(boxed_text(&text)));
        self.push_then_log(page, move |header| format!("BackButtonContent=\"{text}\" on \"{header}\""));
    }

    /// `async void`: the log line follows the push.
    fn on_push_custom_icon(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let page = self.create_page(
            &format!("Icon Back #{}", self.push_count.get() + 1),
            "BackButtonContent = PathIcon (x)\n\nThe back button shows a custom icon.",
            None,
        );
        let icon = TextBlock::new();
        icon.set_text(Some("\u{2715}"));
        icon.set_font_size(16.0);
        icon.set_vertical_alignment(VerticalAlignment::Center);
        NavigationPage::set_back_button_content(&page, Some(Control::boxed(icon)));
        self.push_then_log(page, |header| format!("BackButtonContent=icon on \"{header}\""));
    }

    /// `async void`: the log line follows the push.
    fn on_push_icon_text_back(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let page = self.create_page(
            &format!("Icon+Text Back #{}", self.push_count.get() + 1),
            "BackButtonContent = icon + text\n\nThe back button shows both icon and text.",
            None,
        );

        let icon = TextBlock::new();
        icon.set_text(Some("\u{2715}"));
        icon.set_font_size(14.0);
        icon.set_vertical_alignment(VerticalAlignment::Center);
        let label = TextBlock::new();
        label.set_text(Some("Close"));
        label.set_vertical_alignment(VerticalAlignment::Center);
        label.set_font_size(14.0);
        let content = StackPanel::new();
        content.set_orientation(Orientation::Horizontal);
        content.set_spacing(4.0);
        content.set_vertical_alignment(VerticalAlignment::Center);
        content.children().add(icon);
        content.children().add(label);

        NavigationPage::set_back_button_content(&page, Some(Control::boxed(content)));
        self.push_then_log(page, |header| format!("BackButtonContent=icon+text on \"{header}\""));
    }

    /// `async void`: the log line follows the push.
    fn on_push_guarded(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let use_async = self.defer_radio().is_some_and(|radio| radio.is_checked() == Some(true));
        let mode = if use_async { "async save" } else { "cancel" };

        let page = self.create_page(
            &format!("Guarded #{}", self.push_count.get() + 1),
            if use_async {
                "This page uses an async Navigating handler.\n\nWhen you tap back, it simulates\nan async save (1.5s) before\nallowing the navigation."
            } else {
                "This page cancels back navigation.\n\nTapping back will be blocked.\nUse the Pop button to force-pop."
            },
            Some(parse_color("#FCE4EC")),
        );

        // The handler belongs to the pushed page, which a descendant of the control holds: it
        // holds the control weakly.
        let weak = self.to_ref().downgrade();
        page.navigating(move |args| {
            let (weak, args) = (weak.clone(), args.clone());
            Box::pin(async move {
                if args.navigation_type() != NavigationType::Pop {
                    return;
                }

                if use_async {
                    if let Some(this) = weak.upgrade() {
                        this.add_log("Saving...");
                    }
                    delay(Duration::from_millis(1500)).await;
                    if let Some(this) = weak.upgrade() {
                        this.add_log("Saved, navigation allowed");
                    }
                } else {
                    args.set_cancel(true);
                    if let Some(this) = weak.upgrade() {
                        this.add_log("Navigation CANCELLED");
                    }
                }
            })
        });

        self.push_then_log(page, move |_| format!("Guarded page ({mode}) pushed"));
    }

    /// `await DemoNav.PushAsync(page); AddLog(message(page.Header))`.
    fn push_then_log(&self, page: Ref<ContentPage>, message: impl FnOnce(String) -> String + 'static) {
        let header = header_text(&page);
        let push = self.nav().push_async(page);
        let weak = self.to_ref().downgrade();
        drop(mini_mvvm::start_async(async move {
            let _ = push.await;
            if let Some(this) = weak.upgrade() {
                this.add_log(&message(header));
            }
        }));
    }

    /// `async void`: nothing follows the pop.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.nav().pop_async());
    }

    /// `async void`: nothing follows the pop.
    fn on_pop_to_root(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.nav().pop_to_root_async());
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.log_panel().children().clear();
    }

    fn add_log(&self, message: &str) {
        let text = TextBlock::new();
        text.set_text(Some(message));
        text.set_font_family(FontFamily::new("Cascadia Code,Consolas,Menlo,monospace"));
        text.set_font_size(10.0);
        text.set_text_wrapping(TextWrapping::Wrap);
        self.log_panel().children().insert(0, text);
    }

    fn create_page(&self, title: &str, body: &str, bg: Option<Color>) -> Ref<ContentPage> {
        self.push_count.set(self.push_count.get() + 1);
        let page = NavigationDemoHelper::make_page(title, body, self.push_count.get());
        if let Some(bg) = bg {
            page.set_background(Some(SolidColorBrush::with_color(bg).into()));
        }
        page
    }
}
