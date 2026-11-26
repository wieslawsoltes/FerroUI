//! Not from upstream: the page of WinUI's BreadcrumbBar (docs/porting/winui-controls.md, section
//! 10.11), the class of `Pages/BreadcrumbBarPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::collections::FerroList;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{BreadcrumbBar, Button, ContentPage, ItemsSource, TextBlock};
use std::rc::Rc;

/// The folders of the path of the page.
const FOLDERS: &[&str] = &["Home", "Documents", "Design", "Northwind", "Images", "Folder1", "Folder2", "Folder3"];

#[repr(C)]
pub struct BreadcrumbBarPage {
    base: ContentPage,
}

content_page_class!(BreadcrumbBarPage);
ferro_class_info!(BreadcrumbBarPage { new: BreadcrumbBarPage::new });
xaml_class!(BreadcrumbBarPage, "/Pages/BreadcrumbBarPage.xaml");

impl BreadcrumbBarPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.wire();
        this
    }

    /// The path, whose clicks go back to the clicked folder and whose reset restores it; the narrow
    /// and the templated bars show the same folders.
    fn wire(&self) {
        let folders = Rc::new(FerroList::<String>::new());
        folders.add_range(FOLDERS.iter().map(|folder| folder.to_string()));
        let path = self.get_control::<BreadcrumbBar>("Path");
        path.set_items_source(Some(folders.clone().into()));
        let clicked = self.get_control::<TextBlock>("Clicked").downgrade();
        let items = folders.clone();
        path.item_clicked(move |_, args| {
            if let Some(clicked) = clicked.upgrade() {
                let name = args.item().and_then(|item| item.downcast_ref::<String>().cloned()).unwrap_or_default();
                clicked.set_text(Some(&format!("ItemClicked: {name} at index {}", args.index())));
            }
            let keep = (args.index() + 1).max(0) as usize;
            while items.count() > keep {
                items.remove_at(items.count() - 1);
            }
        })
        .detach();
        self.get_control::<Button>("Reset").click(move |_, _| {
            folders.clear();
            folders.add_range(FOLDERS.iter().map(|folder| folder.to_string()));
        });
        for name in ["Narrow", "Templated"] {
            let bar = self.get_control::<BreadcrumbBar>(name);
            bar.set_items_source(Some(ItemsSource::from_strs(FOLDERS.iter().copied())));
        }
    }
}
