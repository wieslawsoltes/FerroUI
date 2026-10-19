//! Port of `Views/MetricsView.xaml.cs`: the class of the document `MetricsView.xaml`, the routed
//! metrics view.

use super::bindable_view::BindableView;
use crate::markup::{reactive_user_control_class, xaml_class};
use crate::view_models::MetricsViewModel;
use ferroui_base::data::BindingValue;
use ferroui_base::{ferro_class_info, instantiate, Ref, Visual};
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::{Border, Slider};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{IViewForOf, ViewForMixins};
use ferroui_reactiveui_controls::{FerroObjectReactiveExtensions, ReactiveUserControl, ReactiveUserControlBase};
use ferroui_system::{IObservable, IObserver};
use std::rc::Rc;

/// The minimum preview opacity (`MinimumPreviewOpacity`).
const MINIMUM_PREVIEW_OPACITY: f64 = 0.35;

/// The preview opacity range (`PreviewOpacityRange`).
const PREVIEW_OPACITY_RANGE: f64 = 0.65;

/// The maximum percentage value (`MaximumPercentage`).
const MAXIMUM_PERCENTAGE: f64 = 100.0;

/// The view the activation of the metrics view takes.
type View = BindableView<ReactiveUserControlBase, MetricsViewModel>;

/// The routed metrics view (`MetricsView : ReactiveUserControl<MetricsViewModel>`).
#[repr(C)]
pub struct MetricsView {
    base: ReactiveUserControlBase,
}

reactive_user_control_class!(MetricsView);
ferro_class_info!(MetricsView { new: MetricsView::new });
xaml_class!(MetricsView, "/Views/MetricsView.xaml");

impl MetricsView {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct() }
    }

    /// Creates the view (`MetricsView()`): while the view is active, the value of the threshold
    /// slider drives the opacity of the preview, through the property subjects of the two controls.
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

    /// The view typed by its view model (`ReactiveUserControl<MetricsViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<MetricsViewModel> {
        ReactiveUserControl::<MetricsViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<MetricsViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<MetricsViewModel>> {
        Self::typed(this).into_view_for_of()
    }

    /// The view model of the view (`ViewModel`).
    pub fn view_model(this: &Ref<Self>) -> Option<Rc<MetricsViewModel>> {
        Self::typed(this).view_model()
    }

    /// The threshold slider (the element `ThresholdSlider`).
    pub fn threshold_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("ThresholdSlider")
    }

    /// The threshold preview (the element `ThresholdPreview`).
    pub fn threshold_preview(&self) -> Ref<Border> {
        self.get_control::<Border>("ThresholdPreview")
    }

    /// Drives the opacity of the preview from the value of the slider while the view is active
    /// (the activation block of the constructor of the reference).
    fn bind_view(view: &Rc<View>, disposables: &Rc<CompositeDisposable>) {
        let Some(this) = view.control().and_then(|control| control.cast::<MetricsView>()) else {
            return;
        };
        let preview = this.threshold_preview().get_binding_subject(Visual::opacity_property());
        let values: Rc<dyn IObservable<f64>> = this.threshold_slider().get_subject(RangeBase::value_property());
        disposables.add(
            values
                .select(|value: f64| BindingValue::new(MINIMUM_PREVIEW_OPACITY + (value / MAXIMUM_PERCENTAGE * PREVIEW_OPACITY_RANGE)))
                .subscribe_next(move |opacity| preview.on_next(opacity)),
        );
    }
}
