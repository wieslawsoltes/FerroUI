//! Port of `Views/ReactiveUI/ReactiveNavigationView` of the reference sample: the class of
//! `pages/ReactiveNavigationView.xaml`, the navigation actions of the ReactiveUI behaviors on a
//! router whose pages a routed view host shows.

use crate::markup::xaml_class;
use crate::pages::detail_page_view::DetailPageView;
use crate::pages::home_page_view::HomePageView;
use crate::pages::PageInfo;
use crate::view_models::reactive_ui::detail_page_view_model::DetailPageViewModel;
use crate::view_models::reactive_ui::home_page_view_model::HomePageViewModel;
use crate::view_models::reactive_ui::reactive_navigation_view_model::ReactiveNavigationViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, FerroObject, Ref, TypeInfo};
use ferroui_controls::{Button, Control};
use ferroui_reactiveui::{IViewForOf, RoutableViewModel};
use ferroui_reactiveui_controls::{ReactiveUserControl, ReactiveUserControlBase};
use ferroui_xaml_interactions::EventTriggerBehavior;
use ferroui_xaml_interactions_reactiveui::{NavigateToAction, NavigateToAndResetAction};
use ferroui_xaml_interactivity::Interaction;
use std::rc::Rc;

/// The page (`ReactiveNavigationView : ReactiveUserControl<ReactiveNavigationViewModel>`).
#[repr(C)]
pub struct ReactiveNavigationView {
    base: ReactiveUserControlBase,
}

::ferroui_base::ferro_class!(ReactiveNavigationView: ReactiveUserControlBase);
::ferroui_base::ferro_impl_classes!(
    ReactiveNavigationView: ::ferroui_base::FerroObjectImpl,
    ::ferroui_base::StyledElementImpl,
    ::ferroui_base::VisualImpl,
    ::ferroui_base::layout::LayoutableImpl,
    ::ferroui_base::interactivity::InteractiveImpl,
    ::ferroui_base::input::InputElementImpl,
    ::ferroui_controls::ControlImpl,
    ::ferroui_controls::primitives::TemplatedControlImpl,
    ::ferroui_controls::ContentControlImpl
);
ferro_class_info!(ReactiveNavigationView {
    new: ReactiveNavigationView::new
});
xaml_class!(
    ReactiveNavigationView,
    "/pages/ReactiveNavigationView.xaml"
);

impl ReactiveNavigationView {
    pub fn construct() -> Self {
        Self {
            base: ReactiveUserControlBase::construct(),
        }
    }

    /// `ReactiveNavigationView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        let view_model = ReactiveNavigationViewModel::new();
        this.set_data_context(Some(view_model.clone() as BoxedValue));

        // Deviation (DEVIATIONS.md, Behaviors gallery): markup has no type arguments for the
        // generic actions (`NavigateToAction x:TypeArguments="vm:DetailPageViewModel"`,
        // `NavigateToAndResetAction x:TypeArguments="vm:HomePageViewModel"`); they are created
        // here with their view model types, with the router of the view model the document binds
        // (`Router="{Binding Router}"`), and added to the click triggers of their buttons.
        let navigate_to = NavigateToAction::new::<DetailPageViewModel>();
        navigate_to.set_router(Some(view_model.router()));
        Self::add_click_action(&this, "NavigateToDetailsButton", navigate_to.upcast());

        let navigate_to_and_reset = NavigateToAndResetAction::new::<HomePageViewModel>();
        navigate_to_and_reset.set_router(Some(view_model.router()));
        Self::add_click_action(
            &this,
            "NavigateToHomeAndResetButton",
            navigate_to_and_reset.upcast(),
        );
        this
    }

    /// Adds `action` to the click trigger of the button named `button`.
    fn add_click_action(this: &Ref<Self>, button: &str, action: Ref<FerroObject>) {
        let Some(button) = this.find_control::<Button>(button) else {
            return;
        };
        let behaviors = Interaction::get_behaviors(&button);
        let trigger = behaviors
            .snapshot()
            .iter()
            .find_map(|behavior| behavior.cast::<EventTriggerBehavior>());
        if let Some(actions) = trigger.and_then(|trigger| trigger.actions()) {
            actions.list().add(action);
        }
    }

    /// The view typed by its view model (`ReactiveUserControl<ReactiveNavigationViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<ReactiveNavigationViewModel> {
        ReactiveUserControl::<ReactiveNavigationViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<ReactiveNavigationViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<ReactiveNavigationViewModel>> {
        Self::typed(this).into_view_for_of()
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ReactiveNavigationView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the views of its two pages.
pub(crate) const TYPES: &[&TypeInfo] = &[
    ReactiveNavigationView::TYPE,
    HomePageView::TYPE,
    DetailPageView::TYPE,
];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Reactive Navigation",
    reference: "Views/ReactiveUI/ReactiveNavigationView",
    create: Some(create),
    types: TYPES,
    classes: &[
        &ReactiveNavigationView::XAML_CLASS,
        &HomePageView::XAML_CLASS,
        &DetailPageView::XAML_CLASS,
    ],
    markup_types: &[
        <ReactiveNavigationViewModel as MarkupTyped>::MARKUP,
        <HomePageViewModel as MarkupTyped>::MARKUP,
        <DetailPageViewModel as MarkupTyped>::MARKUP,
    ],
    register_value_types,
};

/// What the untyped value conversions must know about the view models of the page: the page view
/// models as routable view models (the `ViewModel` of the navigation actions).
fn register_value_types() {
    ValueTypes::register_reference::<ReactiveNavigationViewModel>();
    ValueTypes::register_reference::<HomePageViewModel>();
    ValueTypes::register_cast::<Rc<HomePageViewModel>, RoutableViewModel>(|view_model| {
        view_model.clone()
    });
    ValueTypes::register_cast::<Rc<HomePageViewModel>, Option<RoutableViewModel>>(|view_model| {
        Some(view_model.clone() as RoutableViewModel)
    });
    ValueTypes::register_reference::<DetailPageViewModel>();
    ValueTypes::register_cast::<Rc<DetailPageViewModel>, RoutableViewModel>(|view_model| {
        view_model.clone()
    });
    ValueTypes::register_cast::<Rc<DetailPageViewModel>, Option<RoutableViewModel>>(
        |view_model| Some(view_model.clone() as RoutableViewModel),
    );
}
