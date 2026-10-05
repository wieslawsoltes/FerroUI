//! Port of `Pages/TabbedPage/TabbedPageFirstLookPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageFirstLookPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, value_text};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Thickness};
use ferroui_controls::{
    ComboBox, ContentPage, Control, PageList, PageSelectionChangedEventArgs, SelectionChangedEventArgs, StackPanel,
    TabPlacement, TabbedPage, TextBlock, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPageFirstLookPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    tab_counter: Cell<i32>,
}

user_control_class!(TabbedPageFirstLookPage);
ferro_class_info!(TabbedPageFirstLookPage {
    new: TabbedPageFirstLookPage::new,
    markup: {
        methods: [
            fn OnAddTab(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_tab(&sender, e.as_routed_event_args())
                },
            fn OnRemoveTab(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_remove_tab(&sender, e.as_routed_event_args())
                },
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_placement_changed(&sender, e)
                    }
                },
            fn OnSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PageSelectionChangedEventArgs>() {
                        this.on_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(TabbedPageFirstLookPage, "/Pages/TabbedPage/TabbedPageFirstLookPage.xaml");

impl TabbedPageFirstLookPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false), tab_counter: Cell::new(3) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    fn placement_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("PlacementCombo")
    }

    /// The field `StatusText`: null until `InitializeComponent()` has returned.
    fn status_text(&self) -> Option<Ref<TextBlock>> {
        self.component_initialized.get().then(|| self.get_control::<TextBlock>("StatusText"))
    }

    /// The field `DemoTabs`: null until `InitializeComponent()` has returned.
    fn demo_tabs(&self) -> Option<Ref<TabbedPage>> {
        self.component_initialized.get().then(|| self.get_control::<TabbedPage>("DemoTabs"))
    }

    /// `(IList)DemoTabs.Pages!`.
    ///
    /// # Panics
    /// Panics if the tabbed page does not exist yet or has no list of pages (a null reference
    /// in the managed original).
    fn pages(&self) -> PageList {
        self.demo_tabs().and_then(|demo_tabs| demo_tabs.pages()).expect("the pages of the tabbed page")
    }

    fn on_add_tab(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.tab_counter.set(self.tab_counter.get() + 1);
        let idx = self.tab_counter.get();

        let title = TextBlock::new();
        title.set_text(Some(&format!("Tab {idx}")));
        title.set_font_size(24.0);
        title.set_font_weight(FontWeight::Bold);

        let text = TextBlock::new();
        text.set_text(Some(&format!("This tab was added dynamically (tab #{idx}).")));
        text.set_opacity(0.7);
        text.set_text_wrapping(TextWrapping::Wrap);

        let panel = StackPanel::new();
        panel.set_margin(Thickness::uniform(16.0));
        panel.set_spacing(8.0);
        panel.children().add(title);
        panel.children().add(text);

        let page = ContentPage::new();
        page.set_header(Some(boxed_text(&format!("Tab {idx}"))));
        page.set_content(Some(Control::boxed(panel)));

        self.pages().add(page.upcast());
        self.update_status();
    }

    fn on_remove_tab(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let pages = self.pages();
        if pages.count() > 1 {
            pages.remove_at(pages.count() - 1);
            self.update_status();
        }
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_tabs) = self.demo_tabs() else {
            return;
        };
        demo_tabs.set_tab_placement(match self.placement_combo().selected_index() {
            1 => TabPlacement::Bottom,
            2 => TabPlacement::Left,
            3 => TabPlacement::Right,
            _ => TabPlacement::Top,
        });
    }

    fn on_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &PageSelectionChangedEventArgs) {
        self.update_status();
    }

    fn update_status(&self) {
        let Some(status_text) = self.status_text() else {
            return;
        };
        let demo_tabs = self.get_control::<TabbedPage>("DemoTabs");
        let pages = self.pages();
        let page_name = demo_tabs
            .selected_page()
            .and_then(|page| page.cast::<ContentPage>())
            .and_then(|page| value_text(&page.header()))
            .unwrap_or_else(|| String::from("\u{2014}"));
        let count = pages.count();
        status_text.set_text(Some(&format!(
            "{count} tab{} | Selected: {page_name} ({})",
            if count != 1 { "s" } else { "" },
            demo_tabs.selected_index()
        )));
    }
}
