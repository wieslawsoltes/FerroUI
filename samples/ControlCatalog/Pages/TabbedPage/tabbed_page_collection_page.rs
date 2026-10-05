//! Port of `Pages/TabbedPage/TabbedPageCollectionPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageCollectionPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, parse_color};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::media::{FontWeight, SolidColorBrush, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Thickness};
use ferroui_controls::{ContentPage, Control, PageList, StackPanel, TabbedPage, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

const COLORS: [&str; 8] = ["#E53935", "#1E88E5", "#43A047", "#FB8C00", "#8E24AA", "#00ACC1", "#6D4C41", "#546E7A"];

#[repr(C)]
pub struct TabbedPageCollectionPage {
    base: UserControl,
    counter: Cell<i32>,
}

user_control_class!(TabbedPageCollectionPage);
ferro_class_info!(TabbedPageCollectionPage {
    new: TabbedPageCollectionPage::new,
    markup: {
        methods: [
            fn OnAddPage(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCollectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_page(&sender, e.as_routed_event_args())
                },
            fn OnRemoveLast(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCollectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_remove_last(&sender, e.as_routed_event_args())
                },
            fn OnClearAll(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCollectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_all(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPageCollectionPage, "/Pages/TabbedPage/TabbedPageCollectionPage.xaml");

impl TabbedPageCollectionPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), counter: Cell::new(3) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn demo_tabs(&self) -> Ref<TabbedPage> {
        self.get_control::<TabbedPage>("DemoTabs")
    }

    /// `(IList)DemoTabs.Pages!`.
    ///
    /// # Panics
    /// Panics if the tabbed page has no list of pages (a null reference in the managed
    /// original).
    fn pages(&self) -> PageList {
        self.demo_tabs().pages().expect("the pages of the tabbed page")
    }

    fn on_add_page(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let index = self.counter.get();
        self.counter.set(index + 1);
        let color = COLORS[index as usize % COLORS.len()];

        let title = TextBlock::new();
        title.set_text(Some(&format!("Tab {}", index + 1)));
        title.set_font_size(24.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_foreground(Some(SolidColorBrush::with_color(parse_color(color)).into()));

        let text = TextBlock::new();
        text.set_text(Some(&format!("This page was added dynamically (#{}).", index + 1)));
        text.set_opacity(0.7);
        text.set_text_wrapping(TextWrapping::Wrap);

        let panel = StackPanel::new();
        panel.set_margin(Thickness::uniform(16.0));
        panel.set_spacing(8.0);
        panel.children().add(title);
        panel.children().add(text);

        let page = ContentPage::new();
        page.set_header(Some(boxed_text(&format!("Tab {}", index + 1))));
        page.set_content(Some(Control::boxed(panel)));

        self.pages().add(page.upcast());
        self.demo_tabs().set_selected_index(self.pages().count() as i32 - 1);
        self.update_status();
    }

    fn on_remove_last(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let pages = self.pages();
        if pages.count() > 0 {
            pages.remove_at(pages.count() - 1);
            self.update_status();
        }
    }

    fn on_clear_all(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let pages = self.pages();
        while pages.count() > 0 {
            pages.remove_at(pages.count() - 1);
        }
        self.counter.set(0);
        self.update_status();
    }

    fn update_status(&self) {
        let count = self.pages().count();
        self.status_text().set_text(Some(&format!("{count} tab{}", if count != 1 { "s" } else { "" })));
    }
}
