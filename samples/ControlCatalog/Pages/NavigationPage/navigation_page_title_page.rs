//! Port of `Pages/NavigationPage/NavigationPageTitlePage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageTitlePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, NavigationDemoHelper};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, SolidColorBrush, TextAlignment};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, CornerRadius, Ref};
use ferroui_controls::{
    Border, ContentPage, Control, NavigationPage, Page, ProgressBar, Slider, StackPanel, TextBlock, TextBox,
    UserControl,
};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

/// `page.Header as string`.
fn string_header(page: &Page) -> Option<String> {
    page.header().and_then(|header| header.downcast_ref::<String>().cloned())
}

#[repr(C)]
pub struct NavigationPageTitlePage {
    base: UserControl,
    initialized: Cell<bool>,
    page_count: Cell<i32>,
}

user_control_class!(NavigationPageTitlePage);
ferro_class_info!(NavigationPageTitlePage {
    new: NavigationPageTitlePage::new,
    markup: {
        methods: [
            fn OnSetStringHeader(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTitlePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_set_string_header(&sender, e.as_routed_event_args())
                },
            fn OnSetSearchHeader(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTitlePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_set_search_header(&sender, e.as_routed_event_args())
                },
            fn OnSetSliderHeader(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTitlePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_set_slider_header(&sender, e.as_routed_event_args())
                },
            fn OnSetLayoutHeader(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTitlePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_set_layout_header(&sender, e.as_routed_event_args())
                },
            fn OnPushWithStringHeader(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTitlePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_with_string_header(&sender, e.as_routed_event_args())
                },
            fn OnPushWithCustomHeader(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTitlePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_with_custom_header(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTitlePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageTitlePage, "/Pages/NavigationPage/NavigationPageTitlePage.xaml");

impl NavigationPageTitlePage {
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

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    /// `async void`.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);
        let this = self.to_ref();
        drop(start_async(async move {
            let page = NavigationDemoHelper::make_page("Home", "Choose a header type and tap 'Push'.", 0);
            if this.demo_nav().push_async_with_transition(page, None).await.is_err() {
                return;
            }
            this.status_text().set_text(Some("Current: Home"));
        }));
    }

    fn on_set_string_header(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(current_page) = self.demo_nav().current_page() else {
            return;
        };
        let header = string_header(&current_page).unwrap_or_else(|| String::from("Home"));
        current_page.set_header(Some(boxed_text(&header)));
        self.status_text().set_text(Some("String Header"));
    }

    fn on_set_search_header(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(current_page) = self.demo_nav().current_page() else {
            return;
        };
        let search = TextBox::new();
        search.set_placeholder_text(Some("Search..."));
        search.set_width(200.0);
        search.set_horizontal_alignment(HorizontalAlignment::Left);
        search.set_vertical_alignment(VerticalAlignment::Center);
        current_page.set_header(Some(Control::boxed(search)));
        self.status_text().set_text(Some("Custom: Search Box"));
    }

    fn on_set_slider_header(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(current_page) = self.demo_nav().current_page() else {
            return;
        };
        let slider = Slider::new();
        slider.set_width(200.0);
        slider.set_minimum(0.0);
        slider.set_maximum(100.0);
        slider.set_range_value(50.0);
        slider.set_horizontal_alignment(HorizontalAlignment::Left);
        slider.set_vertical_alignment(VerticalAlignment::Center);
        current_page.set_header(Some(Control::boxed(slider)));
        self.status_text().set_text(Some("Custom: Slider"));
    }

    fn on_set_layout_header(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(current_page) = self.demo_nav().current_page() else {
            return;
        };
        let current_header = string_header(&current_page).unwrap_or_else(|| String::from("Page"));

        let dot = Border::new();
        dot.set_width(24.0);
        dot.set_height(24.0);
        dot.set_corner_radius(CornerRadius::uniform(12.0));
        dot.set_background(Some(SolidColorBrush::with_color(parse_color("#4CAF50")).into()));

        let title = TextBlock::new();
        title.set_text(Some(&current_header));
        title.set_font_size(14.0);
        title.set_font_weight(FontWeight::SemiBold);
        let subtitle = TextBlock::new();
        subtitle.set_text(Some("Online"));
        subtitle.set_font_size(10.0);
        subtitle.set_opacity(0.6);

        let texts = StackPanel::new();
        texts.set_spacing(0.0);
        texts.set_vertical_alignment(VerticalAlignment::Center);
        texts.children().add(title);
        texts.children().add(subtitle);

        let header = StackPanel::new();
        header.set_orientation(Orientation::Horizontal);
        header.set_spacing(8.0);
        header.set_vertical_alignment(VerticalAlignment::Center);
        header.children().add(dot);
        header.children().add(texts);
        current_page.set_header(Some(Control::boxed(header)));
        self.status_text().set_text(Some("Custom: Icon + Subtitle"));
    }

    /// `async void`.
    fn on_push_with_string_header(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            this.page_count.set(this.page_count.get() + 1);
            let page_count = this.page_count.get();
            let page = NavigationDemoHelper::make_page(
                &format!("Page {page_count}"),
                "This page uses a string Header.",
                page_count,
            );
            if this.demo_nav().push_async(page).await.is_err() {
                return;
            }
            this.status_text().set_text(Some(&format!("Pushed: \"Page {}\"", this.page_count.get())));
        }));
    }

    /// `async void`.
    fn on_push_with_custom_header(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            this.page_count.set(this.page_count.get() + 1);
            let page_count = this.page_count.get();

            let step = TextBlock::new();
            step.set_text(Some(&format!("Step {page_count}")));
            step.set_font_size(16.0);
            step.set_font_weight(FontWeight::SemiBold);
            step.set_vertical_alignment(VerticalAlignment::Center);
            let progress = ProgressBar::new();
            progress.set_width(100.0);
            progress.set_minimum(0.0);
            progress.set_maximum(100.0);
            progress.set_range_value(f64::from(page_count * 25));
            progress.set_vertical_alignment(VerticalAlignment::Center);

            let progress_header = StackPanel::new();
            progress_header.set_orientation(Orientation::Horizontal);
            progress_header.set_spacing(12.0);
            progress_header.set_vertical_alignment(VerticalAlignment::Center);
            progress_header.children().add(step);
            progress_header.children().add(progress);

            let title = TextBlock::new();
            title.set_text(Some(&format!("Step {page_count}")));
            title.set_font_size(24.0);
            title.set_font_weight(FontWeight::Bold);
            title.set_horizontal_alignment(HorizontalAlignment::Center);
            let body = TextBlock::new();
            body.set_text(Some("This page has a custom Header\nwith a progress bar."));
            body.set_font_size(14.0);
            body.set_opacity(0.6);
            body.set_horizontal_alignment(HorizontalAlignment::Center);
            body.set_text_alignment(TextAlignment::Center);

            let panel = StackPanel::new();
            panel.set_horizontal_alignment(HorizontalAlignment::Center);
            panel.set_vertical_alignment(VerticalAlignment::Center);
            panel.set_spacing(8.0);
            panel.children().add(title);
            panel.children().add(body);

            let page = ContentPage::new();
            page.set_header(Some(Control::boxed(progress_header)));
            page.set_background(Some(NavigationDemoHelper::get_page_brush(page_count)));
            page.set_content(Some(Control::boxed(panel)));
            page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            page.set_vertical_content_alignment(VerticalAlignment::Stretch);

            if this.demo_nav().push_async(page).await.is_err() {
                return;
            }
            this.status_text()
                .set_text(Some(&format!("Pushed: Custom Header (Step {})", this.page_count.get())));
        }));
    }

    /// `async void`.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            if this.demo_nav().pop_async().await.is_err() {
                return;
            }
            let header = this.demo_nav().current_page().and_then(|page| string_header(&page));
            this.status_text().set_text(Some(&format!(
                "Current: {}",
                header.unwrap_or_else(|| String::from("(custom control)"))
            )));
        }));
    }
}
