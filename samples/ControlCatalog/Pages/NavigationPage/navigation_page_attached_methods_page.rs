//! Port of `Pages/NavigationPage/NavigationPageAttachedMethodsPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageAttachedMethodsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, NavigationDemoHelper};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextAlignment, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    CheckBox, ComboBox, ContentPage, Control, NavigationPage, StackPanel, TextBlock, TextBox, UserControl,
};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct NavigationPageAttachedMethodsPage {
    base: UserControl,
    initialized: Cell<bool>,
    page_count: Cell<i32>,
}

user_control_class!(NavigationPageAttachedMethodsPage);
ferro_class_info!(NavigationPageAttachedMethodsPage {
    new: NavigationPageAttachedMethodsPage::new,
    markup: {
        methods: [
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAttachedMethodsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAttachedMethodsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnPopToRoot(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAttachedMethodsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_to_root(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageAttachedMethodsPage, "/Pages/NavigationPage/NavigationPageAttachedMethodsPage.xaml");

impl NavigationPageAttachedMethodsPage {
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

    fn has_nav_bar_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("HasNavBarCheck")
    }

    fn has_back_button_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("HasBackButtonCheck")
    }

    fn back_button_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("BackButtonCombo")
    }

    fn header_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("HeaderCombo")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);

        let title = TextBlock::new();
        title.set_text(Some("Root Page"));
        title.set_font_size(24.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_horizontal_alignment(HorizontalAlignment::Center);
        let body = TextBlock::new();
        body.set_text(Some(
            "Configure per-page settings on the right,\nthen press \"Push Page with Above Settings\"\nto apply them to a new page.",
        ));
        body.set_font_size(13.0);
        body.set_text_wrapping(TextWrapping::Wrap);
        body.set_horizontal_alignment(HorizontalAlignment::Center);
        body.set_text_alignment(TextAlignment::Center);
        body.set_opacity(0.65);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(12.0);
        panel.children().add(title);
        panel.children().add(body);

        let page = ContentPage::new();
        page.set_header(Some(boxed_text("Root Page")));
        page.set_content(Some(Control::boxed(panel)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        drop(self.demo_nav().push_async_with_transition(page, None));
    }

    /// `async void`: nothing follows the push.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();

        let title = TextBlock::new();
        title.set_text(Some(&format!("Page {page_count}")));
        title.set_font_size(26.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_horizontal_alignment(HorizontalAlignment::Center);
        let summary = TextBlock::new();
        summary.set_text(Some(&self.build_summary()));
        summary.set_font_size(12.0);
        summary.set_text_wrapping(TextWrapping::Wrap);
        summary.set_horizontal_alignment(HorizontalAlignment::Center);
        summary.set_text_alignment(TextAlignment::Center);
        summary.set_opacity(0.6);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(10.0);
        panel.children().add(title);
        panel.children().add(summary);

        let page = ContentPage::new();
        page.set_background(Some(NavigationDemoHelper::get_page_brush(page_count - 1)));
        page.set_content(Some(Control::boxed(panel)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);

        NavigationPage::set_has_navigation_bar(&page, self.has_nav_bar_check().is_checked() == Some(true));
        NavigationPage::set_has_back_button(&page, self.has_back_button_check().is_checked() == Some(true));

        if self.back_button_combo().selected_index() > 0 {
            let content: Option<BoxedValue> = match self.back_button_combo().selected_index() {
                1 => Some(boxed_text("\u{2190} Back")),
                2 => Some(boxed_text("Cancel")),
                _ => {
                    let check = TextBlock::new();
                    check.set_text(Some("\u{2713}"));
                    check.set_font_size(14.0);
                    check.set_vertical_alignment(VerticalAlignment::Center);
                    let done = TextBlock::new();
                    done.set_text(Some("Done"));
                    done.set_vertical_alignment(VerticalAlignment::Center);

                    let content = StackPanel::new();
                    content.set_orientation(Orientation::Horizontal);
                    content.set_spacing(4.0);
                    content.set_vertical_alignment(VerticalAlignment::Center);
                    content.children().add(check);
                    content.children().add(done);
                    Some(Control::boxed(content))
                }
            };
            NavigationPage::set_back_button_content(&page, content);
        }

        if self.header_combo().selected_index() == 3 {
            NavigationPage::set_has_navigation_bar(&page, false);
        } else {
            let title_view: Option<BoxedValue> = match self.header_combo().selected_index() {
                1 => {
                    let title = TextBlock::new();
                    title.set_text(Some(&format!("Page {page_count}")));
                    title.set_font_size(14.0);
                    title.set_font_weight(FontWeight::SemiBold);
                    let subtitle = TextBlock::new();
                    subtitle.set_text(Some("Custom subtitle"));
                    subtitle.set_font_size(10.0);
                    subtitle.set_opacity(0.5);

                    let title_view = StackPanel::new();
                    title_view.set_spacing(0.0);
                    title_view.set_vertical_alignment(VerticalAlignment::Center);
                    title_view.children().add(title);
                    title_view.children().add(subtitle);
                    Some(Control::boxed(title_view))
                }
                2 => {
                    let search = TextBox::new();
                    search.set_placeholder_text(Some("Search\u{2026}"));
                    search.set_width(200.0);
                    search.set_vertical_alignment(VerticalAlignment::Center);
                    search.set_font_size(13.0);
                    Some(Control::boxed(search))
                }
                _ => Some(boxed_text(&format!("Page {page_count}"))),
            };
            NavigationPage::set_header(&page, title_view);
        }

        drop(self.demo_nav().push_async(page));
    }

    /// `async void`: nothing follows the pop.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.demo_nav().pop_async());
    }

    /// `async void`.
    fn on_pop_to_root(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            if this.demo_nav().pop_to_root_async().await.is_err() {
                return;
            }
            this.page_count.set(0);
        }));
    }

    fn build_summary(&self) -> String {
        let shown = |check: Ref<CheckBox>| if check.is_checked() == Some(true) { "shown" } else { "hidden" };
        let nav_bar = shown(self.has_nav_bar_check());
        let back_btn = shown(self.has_back_button_check());
        let back_label = match self.back_button_combo().selected_index() {
            1 => "\u{2190} Back",
            2 => "Cancel",
            3 => "\u{2713} Done",
            _ => "default icon",
        };
        let header_label = match self.header_combo().selected_index() {
            1 => "Title+subtitle",
            2 => "Search box",
            3 => "hidden",
            _ => "string",
        };
        format!("NavBar: {nav_bar}\nBack button: {back_btn}\nBack content: {back_label}\nHeader: {header_label}")
    }
}
