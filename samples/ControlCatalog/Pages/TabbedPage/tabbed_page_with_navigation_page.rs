//! Port of `Pages/TabbedPage/TabbedPageWithNavigationPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageWithNavigationPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, value_text};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextAlignment, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Thickness};
use ferroui_controls::{
    ComboBox, ContentPage, Control, ListBox, NavigationPage, SelectionChangedEventArgs, StackPanel, TabPlacement,
    TabbedPage, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPageWithNavigationPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    initialized: Cell<bool>,
}

user_control_class!(TabbedPageWithNavigationPage);
ferro_class_info!(TabbedPageWithNavigationPage {
    new: TabbedPageWithNavigationPage::new,
    markup: {
        methods: [
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageWithNavigationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_placement_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(TabbedPageWithNavigationPage, "/Pages/TabbedPage/TabbedPageWithNavigationPage.xaml");

impl TabbedPageWithNavigationPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            initialized: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    /// `async void`: the navigation pages of the tabs get their root pages one after the
    /// other.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);

        let browse_nav = self.get_control::<NavigationPage>("BrowseNav");
        let search_nav = self.get_control::<NavigationPage>("SearchNav");
        let account_nav = self.get_control::<NavigationPage>("AccountNav");

        let pushed =
            browse_nav.push_async_with_transition(Self::create_list_page("Browse", "Items", &browse_nav), None);
        drop(start_async(async move {
            if pushed.await.is_err() {
                return;
            }
            let page = Self::create_list_page("Search", "Results", &search_nav);
            if search_nav.push_async_with_transition(page, None).await.is_err() {
                return;
            }
            let page = Self::create_list_page("Account", "Options", &account_nav);
            let _ = account_nav.push_async_with_transition(page, None).await;
        }));
    }

    fn create_list_page(tab_name: &'static str, list_title: &str, nav: &Ref<NavigationPage>) -> Ref<ContentPage> {
        let list = ListBox::new();
        list.set_margin(Thickness::uniform(8.0));
        for number in 1..=5 {
            list.items().add(Some(boxed_text(&format!("{list_title} item {number}"))));
        }

        // The handler belongs to the list, which ends up in a page of `nav`: it holds both weakly.
        let (weak_list, weak_nav) = (list.downgrade(), nav.downgrade());
        list.selection_changed(move |_, args: &SelectionChangedEventArgs| {
            let (Some(list), Some(nav)) = (weak_list.upgrade(), weak_nav.upgrade()) else {
                return;
            };
            let Some(added) = args.added_items().first() else {
                return;
            };

            let item = value_text(added).unwrap_or_default();
            list.set_selected_item(None);

            let title = TextBlock::new();
            title.set_text(Some(&item));
            title.set_font_size(20.0);
            title.set_font_weight(FontWeight::SemiBold);
            title.set_horizontal_alignment(HorizontalAlignment::Center);

            let text = TextBlock::new();
            text.set_text(Some(&format!("Detail view for \"{item}\" in the {tab_name} tab.")));
            text.set_font_size(13.0);
            text.set_opacity(0.7);
            text.set_text_wrapping(TextWrapping::Wrap);
            text.set_text_alignment(TextAlignment::Center);
            text.set_max_width(280.0);

            let panel = StackPanel::new();
            panel.set_horizontal_alignment(HorizontalAlignment::Center);
            panel.set_vertical_alignment(VerticalAlignment::Center);
            panel.set_spacing(8.0);
            panel.children().add(title);
            panel.children().add(text);

            let detail = ContentPage::new();
            detail.set_header(Some(boxed_text(&item)));
            detail.set_content(Some(Control::boxed(panel)));

            drop(nav.push_async_with_transition(detail, nav.page_transition()));
        });

        let page = ContentPage::new();
        page.set_header(Some(boxed_text(tab_name)));
        page.set_content(Some(Control::boxed(list)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        NavigationPage::set_has_navigation_bar(&page, false);
        page
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.component_initialized.get() {
            return;
        }
        self.get_control::<TabbedPage>("DemoTabs").set_tab_placement(
            if self.get_control::<ComboBox>("PlacementCombo").selected_index() == 0 {
                TabPlacement::Top
            } else {
                TabPlacement::Bottom
            },
        );
    }
}
