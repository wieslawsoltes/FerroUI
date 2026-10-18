//! Not in the original: the class of the document `Pages/TreeDataGridPage.xaml`, which shows the
//! pages of the TreeDataGrid demo (`treedatagrid-demo`, the port of the demo of the TreeDataGrid
//! control): the view of its tabs, with the view models of its pages.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Border, ContentPage};
use treedatagrid_demo::view_models::MainWindowViewModel;
use treedatagrid_demo::views::MainView;

#[repr(C)]
pub struct TreeDataGridPage {
    base: ContentPage,
}

content_page_class!(TreeDataGridPage);
ferro_class_info!(TreeDataGridPage { new: TreeDataGridPage::new });
xaml_class!(TreeDataGridPage, "/Pages/TreeDataGridPage.xaml");

impl TreeDataGridPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let view = MainView::new();
        view.set_data_context(Some(MainWindowViewModel::new() as BoxedValue));
        this.demo_host().set_child(Some(view.upcast()));
        this
    }

    /// The element `DemoHost` of the document.
    fn demo_host(&self) -> Ref<Border> {
        self.get_control::<Border>("DemoHost")
    }
}
