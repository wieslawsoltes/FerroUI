//! Port of `Views/OverviewView.xaml.cs`: the class of the document `OverviewView.xaml`, the routed
//! overview view.

use super::bindable_view::{owned_element_property, BindableView};
use crate::markup::{reactive_user_control_class, xaml_class};
use crate::view_models::{FeatureViewModel, OverviewViewModel};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ItemsControl, ItemsSource};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactiveui::{IViewForOf, PropertyBindingMixins, ViewForMixins};
use ferroui_reactiveui_controls::{ferro_prop, ReactiveUserControl, ReactiveUserControlBase};
use ferroui_system::{prop, IDisposable};
use std::rc::Rc;

/// The view the bindings of the overview take.
type View = BindableView<ReactiveUserControlBase, OverviewViewModel>;

/// The routed overview view (`OverviewView : ReactiveUserControl<OverviewViewModel>`).
#[repr(C)]
pub struct OverviewView {
    base: ReactiveUserControlBase,
}

reactive_user_control_class!(OverviewView);
ferro_class_info!(OverviewView { new: OverviewView::new });
xaml_class!(OverviewView, "/Views/OverviewView.xaml");

impl OverviewView {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct() }
    }

    /// Creates the view (`OverviewView()`): the feature list is bound while the view is active.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        let control: Ref<ReactiveUserControlBase> = this.clone().upcast();
        let view = View::new(&control);
        let bound = view.clone();
        let _ = view.when_activated_disposables(move |disposables| Self::bind_view(&bound, disposables));
        this
    }

    /// The view typed by its view model (`ReactiveUserControl<OverviewViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<OverviewViewModel> {
        ReactiveUserControl::<OverviewViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<OverviewViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<OverviewViewModel>> {
        Self::typed(this).into_view_for_of()
    }

    /// The view model of the view (`ViewModel`).
    pub fn view_model(this: &Ref<Self>) -> Option<Rc<OverviewViewModel>> {
        Self::typed(this).view_model()
    }

    /// The list of the features (the element `FeatureItems`).
    pub fn feature_items(&self) -> Ref<ItemsControl> {
        self.get_control::<ItemsControl>("FeatureItems")
    }

    /// Binds the items while the overview is active, so that the automatic template hook resolves
    /// the rows (`BindView`).
    fn bind_view(view: &Rc<View>, disposables: &Rc<CompositeDisposable>) {
        let Some(control) = view.control() else {
            return;
        };
        let Some(this) = control.cast::<OverviewView>() else {
            return;
        };
        let items = this.feature_items();
        let binding = view.one_way_bind_select(
            None::<Rc<OverviewViewModel>>,
            prop!(OverviewViewModel => features),
            owned_element_property("FeatureItems", &items, ferro_prop!(ItemsControl => items_source), |items, value| items.set_items_source(value)),
            |features: Vec<Rc<FeatureViewModel>>| Some(ItemsSource::from_items(features.into_iter().map(|feature| Some(feature as BoxedValue)))),
        );
        if let Some(binding) = binding {
            disposables.add(binding as Rc<dyn IDisposable>);
        }
    }
}
