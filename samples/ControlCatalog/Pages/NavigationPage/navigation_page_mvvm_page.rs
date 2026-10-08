//! Port of `Pages/NavigationPage/NavigationPageMvvmPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageMvvmPage.xaml`.

use super::navigation_page_mvvm_navigation::{ISampleNavigationService, ISamplePageFactory, SampleNavigationService};
use super::navigation_page_mvvm_page_factory::SamplePageFactory;
use super::navigation_page_mvvm_view_models::NavigationPageMvvmShellViewModel;
use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{NavigationPage, UserControl};
use std::cell::{Cell, OnceCell};
use std::rc::Rc;

#[repr(C)]
pub struct NavigationPageMvvmPage {
    base: UserControl,
    /// Set by the constructor, once the document is loaded.
    view_model: OnceCell<Rc<NavigationPageMvvmShellViewModel>>,
    initialized: Cell<bool>,
}

user_control_class!(NavigationPageMvvmPage);
ferro_class_info!(NavigationPageMvvmPage { new: NavigationPageMvvmPage::new });
xaml_class!(NavigationPageMvvmPage, "/Pages/NavigationPage/NavigationPageMvvmPage.xaml");

impl NavigationPageMvvmPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), view_model: OnceCell::new(), initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let page_factory: Rc<dyn ISamplePageFactory> = Rc::new(SamplePageFactory);
        let navigation_service: Rc<dyn ISampleNavigationService> =
            SampleNavigationService::new(&this.demo_nav(), page_factory);
        let view_model = NavigationPageMvvmShellViewModel::new(navigation_service);
        let _ = this.view_model.set(view_model.clone());
        this.set_data_context(Some(view_model as BoxedValue));

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

    /// `async void`: nothing follows the initialization.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);
        if let Some(view_model) = self.view_model.get() {
            drop(view_model.initialize_async());
        }
    }
}
