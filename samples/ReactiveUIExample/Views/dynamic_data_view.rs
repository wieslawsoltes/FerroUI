//! The class of the document `DynamicDataView.xaml`, the routed view of the DynamicData page.
//!
//! Addition of the port with no upstream counterpart (docs/porting/reactive.md, section 7.3): the
//! reference example has no DynamicData page. The view binds as the command lab of the reference
//! does: the search and the name of a new feature with `Bind`, the rows that `sort_and_bind` fills
//! with `OneWayBind`, and the command that adds a feature with `BindCommand`, inside its activation.

use super::bindable_view::{element_control, element_property, owned_element_property, BindableView};
use crate::markup::{reactive_user_control_class, xaml_class};
use crate::view_models::{DynamicDataViewModel, FeatureItemViewModel};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, ItemsControl, ItemsSource, ListBox, TextBox};
use ferroui_dynamic_data::binding::ObservableCollectionExtended;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactiveui::{settable_prop, BindCommandMixins, IViewForOf, PropertyBindingMixins, ViewForMixins};
use ferroui_reactiveui_controls::{ferro_prop, ReactiveUserControl, ReactiveUserControlBase};
use ferroui_system::{prop, IDisposable};
use std::rc::Rc;

/// The view the bindings of the page take.
type View = BindableView<ReactiveUserControlBase, DynamicDataViewModel>;

/// The routed view of the DynamicData page (`DynamicDataView : ReactiveUserControl<DynamicDataViewModel>`).
#[repr(C)]
pub struct DynamicDataView {
    base: ReactiveUserControlBase,
}

reactive_user_control_class!(DynamicDataView);
ferro_class_info!(DynamicDataView { new: DynamicDataView::new });
xaml_class!(DynamicDataView, "/Views/DynamicDataView.xaml");

impl DynamicDataView {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct() }
    }

    /// Creates the view: the bindings are created on each activation and disposed on
    /// deactivation.
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

    /// The view typed by its view model (`ReactiveUserControl<DynamicDataViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<DynamicDataViewModel> {
        ReactiveUserControl::<DynamicDataViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<DynamicDataViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<DynamicDataViewModel>> {
        Self::typed(this).into_view_for_of()
    }

    /// The view model of the view (`ViewModel`).
    pub fn view_model(this: &Ref<Self>) -> Option<Rc<DynamicDataViewModel>> {
        Self::typed(this).view_model()
    }

    /// The search input (the element `SearchTextBox`).
    pub fn search_text_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("SearchTextBox")
    }

    /// The list of the rows (the element `FeatureList`).
    pub fn feature_list(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("FeatureList")
    }

    /// The input of the name of a new feature (the element `NewFeatureTextBox`).
    pub fn new_feature_text_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("NewFeatureTextBox")
    }

    /// The button that adds a feature (the element `AddFeatureButton`).
    pub fn add_feature_button(&self) -> Ref<Button> {
        self.get_control::<Button>("AddFeatureButton")
    }

    /// Creates the bindings owned by this activation.
    fn bind_view(view: &Rc<View>, disposables: &Rc<CompositeDisposable>) {
        let Some(this) = view.control().and_then(|control| control.cast::<DynamicDataView>()) else {
            return;
        };
        let add = |binding: Option<Rc<dyn IDisposable>>| {
            if let Some(binding) = binding {
                disposables.add(binding);
            }
        };
        let text_box = |name: &str, element: &Ref<TextBox>| {
            element_property(name, element, ferro_prop!(TextBox => text), |text: &Ref<TextBox>, value: Option<String>| {
                text.set_value(TextBox::text_property(), value)
            })
        };
        let none = None::<Rc<DynamicDataViewModel>>;

        add(view
            .bind_select(
                none.clone(),
                settable_prop!(DynamicDataViewModel => search_text),
                text_box("SearchTextBox", &this.search_text_box()),
                Some,
                |text: Option<String>| text.unwrap_or_default(),
            )
            .map(|binding| binding as Rc<dyn IDisposable>));

        add(view
            .bind_select(
                none.clone(),
                settable_prop!(DynamicDataViewModel => new_feature_name),
                text_box("NewFeatureTextBox", &this.new_feature_text_box()),
                Some,
                |text: Option<String>| text.unwrap_or_default(),
            )
            .map(|binding| binding as Rc<dyn IDisposable>));

        let list: Ref<ItemsControl> = this.feature_list().upcast();
        add(view
            .one_way_bind_select(
                none,
                prop!(DynamicDataViewModel => items),
                owned_element_property("FeatureList", &list, ferro_prop!(ItemsControl => items_source), |items, value| items.set_items_source(value)),
                |items: ObservableCollectionExtended<Rc<FeatureItemViewModel>>| Some(ItemsSource::new(Rc::new(items.collection().clone()))),
            )
            .map(|binding| binding as Rc<dyn IDisposable>));

        disposables.add(view.bind_command(prop!(DynamicDataViewModel => add_feature), element_control("AddFeatureButton", &this.add_feature_button()), None));
    }
}
