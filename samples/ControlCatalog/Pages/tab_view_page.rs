//! Not from upstream: the page of WinUI's TabView (#418), the class of `Pages/TabViewPage.xaml`.
//! The add button adds a document, a close request removes its tab, and the box sets the width
//! mode of the tabs.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{
    ComboBox, ContentPage, Control, SelectionChangedEventArgs, Symbol, SymbolIconSource, TabView, TabViewItem,
    TabViewWidthMode, TextBlock,
};
use std::rc::Rc;

#[repr(C)]
pub struct TabViewPage {
    base: ContentPage,
}

content_page_class!(TabViewPage);
ferro_class_info!(TabViewPage { new: TabViewPage::new });
xaml_class!(TabViewPage, "/Pages/TabViewPage.xaml");

/// A new document tab.
fn document(number: usize) -> Ref<TabViewItem> {
    let tab = TabViewItem::new();
    tab.set_header(Some(Rc::new(format!("Document {number}")) as BoxedValue));
    let icon = SymbolIconSource::new();
    icon.set_symbol(Symbol::Document);
    tab.set_icon_source(Some(icon.upcast()));
    let text = TextBlock::new();
    text.set_text(Some(format!("The content of Document {number}.").as_str()));
    text.set_margin(ferroui_base::Thickness::uniform(12.0));
    tab.set_content(Some(Control::boxed(text)));
    tab
}

impl TabViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let tabs = this.get_control::<TabView>("Tabs");
        tabs.add_tab_button_click(|sender| {
            let number = sender.tab_items().count();
            sender.tab_items().add(Some(Control::boxed(document(number))));
        })
        .detach();
        tabs.tab_close_requested(|sender, args| {
            let items = sender.tab_items();
            let tab = args.tab();
            let index = (0..items.count()).find(|&index| {
                items.get_at(index).and_then(|item| Control::from_boxed(&item)).is_some_and(|item| item.ptr_eq(&tab))
            });
            if let Some(index) = index {
                items.remove_at(index);
            }
        })
        .detach();
        let mode = this.get_control::<ComboBox>("WidthMode");
        let (weak_tabs, weak_mode) = (tabs.downgrade(), mode.downgrade());
        mode.add_handler(SelectingItemsControl::selection_changed_event(), move |_, _: &SelectionChangedEventArgs| {
            let (Some(tabs), Some(mode)) = (weak_tabs.upgrade(), weak_mode.upgrade()) else { return };
            tabs.set_tab_width_mode(match mode.selected_index() {
                1 => TabViewWidthMode::SizeToContent,
                2 => TabViewWidthMode::Compact,
                _ => TabViewWidthMode::Equal,
            });
        });
        this
    }
}
