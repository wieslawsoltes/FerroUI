//! Port of `Pages/ContentPage/ContentPageFirstLookPage.xaml.cs`: the class of the document
//! `Pages/ContentPage/ContentPageFirstLookPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Color, FontWeight, SolidColorBrush, TextAlignment, TextWrapping};
use ferroui_controls::{ContentPage, Control, NavigationPage, StackPanel, TextBlock, UserControl};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

const PAGE_COLORS: [Color; 5] = [
    Color::from_rgb(0xE3, 0xF2, 0xFD), // blue
    Color::from_rgb(0xF3, 0xE5, 0xF5), // purple
    Color::from_rgb(0xE8, 0xF5, 0xE9), // green
    Color::from_rgb(0xFF, 0xF8, 0xE1), // amber
    Color::from_rgb(0xFB, 0xE9, 0xE7), // deep orange
];

#[repr(C)]
pub struct ContentPageFirstLookPage {
    base: UserControl,
    page_count: Cell<i32>,
}

user_control_class!(ContentPageFirstLookPage);
ferro_class_info!(ContentPageFirstLookPage {
    new: ContentPageFirstLookPage::new,
    markup: {
        methods: [
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnPopToRoot(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_to_root(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ContentPageFirstLookPage, "/Pages/ContentPage/ContentPageFirstLookPage.xaml");

impl ContentPageFirstLookPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), page_count: Cell::new(0) }
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

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn demo_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    /// `async void`.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        let _ = start_async(async move {
            let pushed = this.demo_nav().push_async(
                this.make_page("Root Page", "ContentPage inside a NavigationPage.\nUse the options to navigate."),
            );
            let _ = pushed.await;
            this.update_status();
        });
    }

    /// `async void`.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        let _ = start_async(async move {
            this.page_count.set(this.page_count.get() + 1);
            let page_count = this.page_count.get();
            let pushed = this.demo_nav().push_async(this.make_page(
                &format!("Page {page_count}"),
                &format!("ContentPage #{page_count}.\nNavigate back using the back button."),
            ));
            let _ = pushed.await;
            this.update_status();
        });
    }

    /// `async void`.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        let _ = start_async(async move {
            let _ = this.demo_nav().pop_async().await;
            this.update_status();
        });
    }

    /// `async void`.
    fn on_pop_to_root(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        let _ = start_async(async move {
            let _ = this.demo_nav().pop_to_root_async().await;
            this.page_count.set(0);
            this.update_status();
        });
    }

    fn update_status(&self) {
        let demo_nav = self.demo_nav();
        let header = demo_nav.current_page().and_then(|page| page.header());
        self.status_text().set_text(Some(&format!(
            "Depth: {} | Current: {}",
            demo_nav.stack_depth(),
            header.as_ref().map_or_else(String::new, |header| ValueTypes::to_display_string(Some(header)))
        )));
    }

    fn make_page(&self, header: &str, body: &str) -> Ref<ContentPage> {
        let header_text = TextBlock::new();
        header_text.set_text(Some(header));
        header_text.set_font_size(20.0);
        header_text.set_font_weight(FontWeight::SemiBold);
        header_text.set_horizontal_alignment(HorizontalAlignment::Center);

        let body_text = TextBlock::new();
        body_text.set_text(Some(body));
        body_text.set_font_size(13.0);
        body_text.set_opacity(0.7);
        body_text.set_text_wrapping(TextWrapping::Wrap);
        body_text.set_text_alignment(TextAlignment::Center);
        body_text.set_max_width(260.0);

        let content = StackPanel::new();
        content.set_horizontal_alignment(HorizontalAlignment::Center);
        content.set_vertical_alignment(VerticalAlignment::Center);
        content.set_spacing(10.0);
        content.children().add(header_text);
        content.children().add(body_text);

        let page = ContentPage::new();
        page.set_header(Some(Rc::new(header.to_string()) as BoxedValue));
        let color = PAGE_COLORS[(self.page_count.get() as usize) % PAGE_COLORS.len()];
        page.set_background(Some(SolidColorBrush::with_color(color).into()));
        page.set_content(Some(Control::boxed(content)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        page
    }
}
