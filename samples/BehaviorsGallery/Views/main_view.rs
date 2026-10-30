//! Port of `Views/MainView.xaml.cs` (the class of `Views/MainView.xaml`).

use crate::controls::SingleSelectionTabControl;
use crate::markup::{user_control_class, xaml_class};
use crate::pages::{PageInfo, PAGES};
use ferroui_base::media::TextWrapping;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, FerroProperty, Ref, Thickness};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{Control, TabItem, TextBlock, UserControl};
use std::rc::Rc;

/// The view of the window: the search box, and the tab control of the pages (one tab per page of
/// the page table, in the order of the reference, sorted by header by the search behavior).
///
/// Deviation (DEVIATIONS.md, Behaviors gallery): the reference writes a tab item per page in its
/// document and creates every page with the view; here the view adds a tab item per entry of the
/// page table ([`PAGES`]) and creates the page of a tab when the tab is first selected, so that
/// the pages stay self-contained and the window opens without creating 234 pages.
#[repr(C)]
pub struct MainView {
    base: UserControl,
}

user_control_class!(MainView);
ferro_class_info!(MainView { new: MainView::new });
xaml_class!(MainView, "/Views/MainView.xaml");

/// The page of a tab item: its index in the page table, held as the tag of the item.
fn page_of(tab_item: &TabItem) -> Option<&'static PageInfo> {
    let index = tab_item
        .tag()
        .and_then(|tag| tag.downcast_ref::<i32>().copied())?;
    usize::try_from(index)
        .ok()
        .and_then(|index| PAGES.get(index))
}

impl MainView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `MainView()`: the document, then a tab item per page.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let tab_control = this.pages_tab_control();
        for (index, page) in PAGES.iter().enumerate() {
            let tab_item = TabItem::new();
            tab_item.set_header(Some(Rc::new(page.header.to_string()) as BoxedValue));
            tab_item.set_tag(Some(Rc::new(index as i32) as BoxedValue));
            tab_control.items().add(Some(Control::boxed(tab_item)));
        }

        // The page of a tab is created when the tab is first selected.
        let selected_item: &'static FerroProperty =
            SelectingItemsControl::selected_item_property().as_property();
        let weak = this.downgrade();
        tab_control.property_changed(move |e| {
            if e.property() == selected_item {
                if let Some(this) = weak.upgrade() {
                    this.create_selected_page();
                }
            }
        });
        tab_control.set_selected_index(0);
        this.create_selected_page();
        this
    }

    /// The tab control of the pages.
    pub fn pages_tab_control(&self) -> Ref<SingleSelectionTabControl> {
        self.get_control::<SingleSelectionTabControl>("PagesTabControl")
    }

    /// The tab items of the pages, in the order of the tab control (sorted by header once the
    /// view is shown).
    pub fn tab_items(&self) -> Vec<Ref<TabItem>> {
        self.pages_tab_control()
            .items()
            .to_vec()
            .into_iter()
            .flatten()
            .filter_map(|item| crate::markup::object_as::<TabItem>(&item))
            .collect()
    }

    /// Selects the tab of `page` (its page is created when it is not yet). Returns whether the
    /// page has a tab.
    pub fn select_page(&self, page: &PageInfo) -> bool {
        let Some(tab_item) = self
            .tab_items()
            .into_iter()
            .find(|item| page_of(item).is_some_and(|p| std::ptr::eq(p, page)))
        else {
            return false;
        };
        self.pages_tab_control()
            .set_selected_item(Some(Control::boxed(tab_item)));
        self.create_selected_page();
        true
    }

    /// The page of the selected tab, created when it is not yet.
    fn create_selected_page(&self) {
        let selected = self
            .pages_tab_control()
            .selected_item()
            .and_then(|item| crate::markup::object_as::<TabItem>(&item));
        let Some(tab_item) = selected else { return };
        if tab_item.content().is_some() {
            return;
        }
        if let Some(page) = page_of(&tab_item) {
            // A page that panics while it is created (a document that does not load) is shown as
            // an error and reported (the smoke run fails), instead of ending the gallery.
            let view =
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| page.create_view()))
                {
                    Ok(view) => view,
                    Err(payload) => {
                        let message = payload
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| {
                                payload
                                    .downcast_ref::<&str>()
                                    .map(|message| message.to_string())
                            })
                            .unwrap_or_else(|| "the page panicked".to_string());
                        crate::smoke::record_failure(page, message.clone());
                        let text = TextBlock::new();
                        text.set_text(Some(&format!(
                            "The page {} failed to load: {message}",
                            page.view_name()
                        )));
                        text.set_text_wrapping(TextWrapping::Wrap);
                        text.set_margin(Thickness::uniform(24.0));
                        text.upcast()
                    }
                };
            tab_item.set_content(Some(Control::boxed(view)));
        }
    }
}
