//! Port of `Pages/TabbedPage/TabbedPageDisabledTabsPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageDisabledTabsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Button, CheckBox, ContentPage, TabbedPage, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPageDisabledTabsPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(TabbedPageDisabledTabsPage);
ferro_class_info!(TabbedPageDisabledTabsPage {
    new: TabbedPageDisabledTabsPage::new,
    markup: {
        methods: [
            fn OnTabEnabledChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageDisabledTabsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_tab_enabled_changed(&sender, e.as_routed_event_args())
                },
            fn OnGoToTab(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageDisabledTabsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_go_to_tab(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPageDisabledTabsPage, "/Pages/TabbedPage/TabbedPageDisabledTabsPage.xaml");

/// `int.TryParse(control.Tag?.ToString(), out index)`.
fn tag_index(tag: &Option<BoxedValue>) -> Option<i32> {
    value_text(tag).and_then(|tag| tag.trim().parse::<i32>().ok())
}

impl TabbedPageDisabledTabsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    fn demo_tabs(&self) -> Ref<TabbedPage> {
        self.get_control::<TabbedPage>("DemoTabs")
    }

    /// The field `StatusText`: null until `InitializeComponent()` has returned.
    fn status_text(&self) -> Option<Ref<TextBlock>> {
        self.component_initialized.get().then(|| self.get_control::<TextBlock>("StatusText"))
    }

    /// # Panics
    /// Panics if the tag of the check box is not the index of a page (the index out of range
    /// exception of the managed original).
    fn on_tab_enabled_changed(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let cb = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<CheckBox>());
        let Some(cb) = cb else {
            return;
        };
        let Some(index) = tag_index(&cb.tag()) else {
            return;
        };

        if let Some(pages) = self.demo_tabs().pages() {
            let index = usize::try_from(index).expect("the index of a page");
            if let Some(page) = pages.get(index).cast::<ContentPage>() {
                TabbedPage::set_is_tab_enabled(&page, cb.is_checked() == Some(true));
            }
        }
    }

    fn on_go_to_tab(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let btn = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>());
        let Some(btn) = btn else {
            return;
        };
        let Some(index) = tag_index(&btn.tag()) else {
            return;
        };

        let demo_tabs = self.demo_tabs();
        let before = demo_tabs.selected_index();
        demo_tabs.set_selected_index(index);
        let after = demo_tabs.selected_index();

        if let Some(status_text) = self.status_text() {
            status_text.set_text(Some(&if before == after && index != after {
                format!("Requested tab {index} (disabled) \u{2192} stayed on tab {after}")
            } else if index != after {
                format!("Requested tab {index} (disabled) \u{2192} skipped to tab {after}")
            } else {
                format!("Selected tab {after}")
            }));
        }
    }
}
