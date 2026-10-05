//! Port of `Pages/RefreshContainerPage.xaml.cs`: the class of the document
//! `Pages/RefreshContainerPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::RefreshContainerViewModel;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Button, ContentPage, RefreshContainer, RefreshRequestedEventArgs};
use mini_mvvm::start_async;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct RefreshContainerPage {
    base: ContentPage,
    view_model: RefCell<Option<Rc<RefreshContainerViewModel>>>,
}

content_page_class!(RefreshContainerPage);
ferro_class_info!(RefreshContainerPage {
    new: RefreshContainerPage::new,
    markup: {
        methods: [
            fn RefreshContainerPage_RefreshRequested(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<RefreshContainerPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RefreshRequestedEventArgs>() {
                        this.refresh_container_page_refresh_requested(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(RefreshContainerPage, "/Pages/RefreshContainerPage.xaml");

impl RefreshContainerPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), view_model: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The button is a descendant of the page: its handler holds the page weakly.
        let weak = this.downgrade();
        this.refresh_button().click(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.refresh_button_click(sender, e);
            }
        });

        let view_model = RefreshContainerViewModel::new();
        *this.view_model.borrow_mut() = Some(view_model.clone());

        this.set_data_context(Some(view_model as BoxedValue));
        this
    }

    fn refresh_button(&self) -> Ref<Button> {
        self.get_control::<Button>("RefreshButton")
    }

    fn refresh(&self) -> Ref<RefreshContainer> {
        self.get_control::<RefreshContainer>("Refresh")
    }

    fn refresh_button_click(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.refresh().request_refresh();
    }

    /// `async void`: the deferral is completed once the view model has
    /// added its item.
    ///
    /// # Panics
    /// Panics if the view model does not exist yet (a null reference in the
    /// managed original).
    fn refresh_container_page_refresh_requested(&self, _sender: &Option<BoxedValue>, e: &RefreshRequestedEventArgs) {
        let deferral = e.get_deferral();
        let view_model = self.view_model.borrow().clone().expect("the view model of the page");

        drop(start_async(async move {
            view_model.add_to_top().await;

            deferral.complete();
        }));
    }
}
