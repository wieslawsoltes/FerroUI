//! Port of `Views/MainView.xaml.cs`: the class of the document `MainView.xaml`, the example shell
//! content, shared by the desktop window and the single-view platforms.

use crate::markup::{reactive_user_control_class, xaml_class};
use crate::view_models::MainViewModel;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_reactiveui::IViewForOf;
use ferroui_reactiveui_controls::{ReactiveUserControl, ReactiveUserControlBase, RoutedViewHost};
use std::rc::Rc;

/// The example shell content (`MainView : ReactiveUserControl<MainViewModel>`): the sidebar and
/// the `RoutedViewHost` that follows the router of the shell.
#[repr(C)]
pub struct MainView {
    base: ReactiveUserControlBase,
}

reactive_user_control_class!(MainView);
ferro_class_info!(MainView { new: MainView::new });
xaml_class!(MainView, "/Views/MainView.xaml");

impl MainView {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct() }
    }

    /// Creates the view (`MainView()`).
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        this
    }

    /// The view typed by its view model (`ReactiveUserControl<MainViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<MainViewModel> {
        ReactiveUserControl::<MainViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<MainViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<MainViewModel>> {
        Self::typed(this).into_view_for_of()
    }

    /// The view model of the view (`ViewModel`).
    pub fn view_model(this: &Ref<Self>) -> Option<Rc<MainViewModel>> {
        Self::typed(this).view_model()
    }

    /// The host of the routed pages (the element `RouterHost`).
    pub fn router_host(&self) -> Ref<RoutedViewHost> {
        self.get_control::<RoutedViewHost>("RouterHost")
    }
}
