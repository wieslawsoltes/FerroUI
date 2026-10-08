//! The samples of the `NavigationPage` gallery (directory `Pages/NavigationPage`): one module per
//! upstream file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod curved_header_home_scroll_view;
mod curved_header_profile_scroll_view;
mod ferro_flix_app_page;
mod ferro_flix_detail_view;
mod ferro_flix_home_view;
mod ferro_flix_search_view;
mod lavenir_app_page;
mod lavenir_dish_detail_view;
mod lavenir_menu_view;
mod lavenir_profile_view;
mod lavenir_reservations_view;
mod navigation_page_appearance_page;
mod navigation_page_attached_methods_page;
mod navigation_page_back_button_page;
mod navigation_page_curved_header_page;
mod navigation_page_events_page;
mod navigation_page_first_look_page;
mod navigation_page_gesture_page;
mod navigation_page_interactive_header_page;
mod navigation_page_modal_page;
mod navigation_page_modal_transitions_page;
mod navigation_page_scroll_aware_page;
mod navigation_page_mvvm_navigation;
mod navigation_page_mvvm_page;
mod navigation_page_mvvm_page_factory;
mod navigation_page_mvvm_view_models;
mod navigation_page_pass_data_page;
mod navigation_page_stack_page;
mod navigation_page_title_page;
mod navigation_page_toolbar_page;
mod navigation_page_transitions_page;
mod pulse_app_page;
mod pulse_home_view;
mod pulse_login_view;
mod pulse_profile_view;
mod pulse_workout_detail_view;
mod pulse_workouts_view;
mod retro_gaming_app_page;
mod retro_gaming_detail_view;
mod retro_gaming_favorites_view;
mod retro_gaming_games_view;
mod retro_gaming_home_view;
mod retro_gaming_profile_view;
mod retro_gaming_search_view;
#[path = "Transitions/mod.rs"]
mod transitions;

pub use curved_header_home_scroll_view::CurvedHeaderHomeScrollView;
pub use curved_header_profile_scroll_view::CurvedHeaderProfileScrollView;
pub use ferro_flix_app_page::FerroFlixAppPage;
pub use ferro_flix_detail_view::FerroFlixDetailView;
pub use ferro_flix_home_view::FerroFlixHomeView;
pub use ferro_flix_search_view::FerroFlixSearchView;
pub use lavenir_app_page::LAvenirAppPage;
pub use lavenir_dish_detail_view::LAvenirDishDetailView;
pub use lavenir_menu_view::{DishSelected, LAvenirMenuView};
pub use lavenir_profile_view::LAvenirProfileView;
pub use lavenir_reservations_view::LAvenirReservationsView;
pub use navigation_page_appearance_page::NavigationPageAppearancePage;
pub use navigation_page_attached_methods_page::NavigationPageAttachedMethodsPage;
pub use navigation_page_back_button_page::NavigationPageBackButtonPage;
pub use navigation_page_curved_header_page::NavigationPageCurvedHeaderPage;
pub use navigation_page_events_page::NavigationPageEventsPage;
pub use navigation_page_first_look_page::NavigationPageFirstLookPage;
pub use navigation_page_gesture_page::NavigationPageGesturePage;
pub use navigation_page_interactive_header_page::{ContactItem, NavigationPageInteractiveHeaderPage};
pub use navigation_page_modal_page::NavigationPageModalPage;
pub use navigation_page_modal_transitions_page::NavigationPageModalTransitionsPage;
pub use navigation_page_scroll_aware_page::NavigationPageScrollAwarePage;
pub use navigation_page_mvvm_page::NavigationPageMvvmPage;
pub use navigation_page_mvvm_view_models::{NavigationPageMvvmShellViewModel, ProjectCardViewModel};
pub use navigation_page_pass_data_page::NavigationPagePassDataPage;
pub use navigation_page_stack_page::NavigationPageStackPage;
pub use navigation_page_title_page::NavigationPageTitlePage;
pub use navigation_page_toolbar_page::NavigationPageToolbarPage;
pub use navigation_page_transitions_page::NavigationPageTransitionsPage;
pub use pulse_app_page::PulseAppPage;
pub use pulse_home_view::PulseHomeView;
pub use pulse_login_view::PulseLoginView;
pub use pulse_profile_view::PulseProfileView;
pub use pulse_workout_detail_view::PulseWorkoutDetailView;
pub use pulse_workouts_view::PulseWorkoutsView;
pub use retro_gaming_app_page::RetroGamingAppPage;
pub use retro_gaming_detail_view::RetroGamingDetailView;
pub use retro_gaming_favorites_view::RetroGamingFavoritesView;
pub use retro_gaming_games_view::RetroGamingGamesView;
pub use retro_gaming_home_view::RetroGamingHomeView;
pub use retro_gaming_profile_view::RetroGamingProfileView;
pub use retro_gaming_search_view::RetroGamingSearchView;
pub use transitions::{
    CompositeTransition, FadeThroughTransition, PageSlideTransition, PageSlideTransitionAxis, ParallaxSlideTransition,
};

pub(crate) const TYPES: &[&TypeInfo] = &[
    CurvedHeaderHomeScrollView::TYPE,
    CurvedHeaderProfileScrollView::TYPE,
    FerroFlixAppPage::TYPE,
    FerroFlixDetailView::TYPE,
    FerroFlixHomeView::TYPE,
    FerroFlixSearchView::TYPE,
    LAvenirAppPage::TYPE,
    LAvenirDishDetailView::TYPE,
    LAvenirMenuView::TYPE,
    LAvenirProfileView::TYPE,
    LAvenirReservationsView::TYPE,
    NavigationPageAppearancePage::TYPE,
    NavigationPageAttachedMethodsPage::TYPE,
    NavigationPageBackButtonPage::TYPE,
    NavigationPageCurvedHeaderPage::TYPE,
    NavigationPageEventsPage::TYPE,
    NavigationPageFirstLookPage::TYPE,
    NavigationPageGesturePage::TYPE,
    NavigationPageInteractiveHeaderPage::TYPE,
    NavigationPageModalPage::TYPE,
    NavigationPageModalTransitionsPage::TYPE,
    NavigationPageScrollAwarePage::TYPE,
    NavigationPageMvvmPage::TYPE,
    NavigationPagePassDataPage::TYPE,
    NavigationPageStackPage::TYPE,
    NavigationPageTitlePage::TYPE,
    NavigationPageToolbarPage::TYPE,
    NavigationPageTransitionsPage::TYPE,
    PulseAppPage::TYPE,
    PulseHomeView::TYPE,
    PulseLoginView::TYPE,
    PulseProfileView::TYPE,
    PulseWorkoutDetailView::TYPE,
    PulseWorkoutsView::TYPE,
    RetroGamingAppPage::TYPE,
    RetroGamingDetailView::TYPE,
    RetroGamingFavoritesView::TYPE,
    RetroGamingGamesView::TYPE,
    RetroGamingHomeView::TYPE,
    RetroGamingProfileView::TYPE,
    RetroGamingSearchView::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &CurvedHeaderHomeScrollView::XAML_CLASS,
    &CurvedHeaderProfileScrollView::XAML_CLASS,
    &FerroFlixAppPage::XAML_CLASS,
    &FerroFlixDetailView::XAML_CLASS,
    &FerroFlixHomeView::XAML_CLASS,
    &FerroFlixSearchView::XAML_CLASS,
    &LAvenirAppPage::XAML_CLASS,
    &LAvenirDishDetailView::XAML_CLASS,
    &LAvenirMenuView::XAML_CLASS,
    &LAvenirProfileView::XAML_CLASS,
    &LAvenirReservationsView::XAML_CLASS,
    &NavigationPageAppearancePage::XAML_CLASS,
    &NavigationPageAttachedMethodsPage::XAML_CLASS,
    &NavigationPageBackButtonPage::XAML_CLASS,
    &NavigationPageCurvedHeaderPage::XAML_CLASS,
    &NavigationPageEventsPage::XAML_CLASS,
    &NavigationPageFirstLookPage::XAML_CLASS,
    &NavigationPageGesturePage::XAML_CLASS,
    &NavigationPageInteractiveHeaderPage::XAML_CLASS,
    &NavigationPageModalPage::XAML_CLASS,
    &NavigationPageModalTransitionsPage::XAML_CLASS,
    &NavigationPageScrollAwarePage::XAML_CLASS,
    &NavigationPageMvvmPage::XAML_CLASS,
    &NavigationPagePassDataPage::XAML_CLASS,
    &NavigationPageStackPage::XAML_CLASS,
    &NavigationPageTitlePage::XAML_CLASS,
    &NavigationPageToolbarPage::XAML_CLASS,
    &NavigationPageTransitionsPage::XAML_CLASS,
    &PulseAppPage::XAML_CLASS,
    &PulseHomeView::XAML_CLASS,
    &PulseLoginView::XAML_CLASS,
    &PulseProfileView::XAML_CLASS,
    &PulseWorkoutDetailView::XAML_CLASS,
    &PulseWorkoutsView::XAML_CLASS,
    &RetroGamingAppPage::XAML_CLASS,
    &RetroGamingDetailView::XAML_CLASS,
    &RetroGamingFavoritesView::XAML_CLASS,
    &RetroGamingGamesView::XAML_CLASS,
    &RetroGamingHomeView::XAML_CLASS,
    &RetroGamingProfileView::XAML_CLASS,
    &RetroGamingSearchView::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <NavigationPageMvvmShellViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
    <ProjectCardViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
];
