//! Port of `Pages/TabbedPage/TabbedPageWithDrawerPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageWithDrawerPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::boxed_text;
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextAlignment, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    Button, ContentControl, ContentPage, Control, DrawerPage, Page, StackPanel, TabPlacement, TabbedPage, TextBlock,
    UserControl,
};
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPageWithDrawerPage {
    base: UserControl,
}

user_control_class!(TabbedPageWithDrawerPage);
ferro_class_info!(TabbedPageWithDrawerPage {
    new: TabbedPageWithDrawerPage::new,
    markup: {
        methods: [
            fn OnSectionSelected(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageWithDrawerPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_section_selected(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPageWithDrawerPage, "/Pages/TabbedPage/TabbedPageWithDrawerPage.xaml");

impl TabbedPageWithDrawerPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.show_section("Home");
            }
        });
        this
    }

    fn demo_drawer(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DemoDrawer")
    }

    fn section_host(&self) -> Ref<ContentControl> {
        self.get_control::<ContentControl>("SectionHost")
    }

    fn on_section_selected(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let section = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>())
            .and_then(|btn| btn.tag())
            .and_then(|tag| tag.downcast_ref::<String>().cloned());
        if let Some(section) = section {
            self.show_section(&section);
            self.demo_drawer().set_is_open(false);
        }
    }

    fn show_section(&self, section: &str) {
        self.section_host().set_content(Some(match section {
            "Home" => Control::boxed(Self::create_home_tabbed()),
            _ => Control::boxed(Self::create_plain_page(section)),
        }));
    }

    fn create_plain_page(section: &str) -> Ref<Control> {
        // The second element of the tuple of the original (an icon) is not used.
        let subtitle = match section {
            "Explore" => "Discover new content.",
            "Favorites" => "Items you've saved.",
            _ => "",
        };

        let title = TextBlock::new();
        title.set_text(Some(section));
        title.set_font_size(22.0);
        title.set_font_weight(FontWeight::SemiBold);
        title.set_horizontal_alignment(HorizontalAlignment::Center);

        let text = TextBlock::new();
        text.set_text(Some(subtitle));
        text.set_font_size(13.0);
        text.set_opacity(0.7);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_text_alignment(TextAlignment::Center);
        text.set_max_width(300.0);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(8.0);
        panel.children().add(title);
        panel.children().add(text);
        panel.upcast()
    }

    fn create_home_tabbed() -> Ref<TabbedPage> {
        let page = |header: &str, content: &str| -> Ref<Page> {
            let text = TextBlock::new();
            text.set_text(Some(content));
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);
            text.set_font_size(18.0);
            text.set_opacity(0.7);

            let page = ContentPage::new();
            page.set_header(Some(boxed_text(header)));
            page.set_content(Some(Control::boxed(text)));
            page.upcast()
        };

        let tabbed = TabbedPage::new();
        tabbed.set_tab_placement(TabPlacement::Bottom);
        tabbed.set_pages(Some(FerroList::from_items([
            page("Featured", "Featured content"),
            page("Recent", "Recent activity"),
            page("Popular", "Popular right now"),
        ])));
        tabbed
    }
}
