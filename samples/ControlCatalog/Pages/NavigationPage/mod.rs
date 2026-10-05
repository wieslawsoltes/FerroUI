//! The samples of the `NavigationPage` gallery (directory `Pages/NavigationPage`): one module per
//! upstream file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod curved_header_home_scroll_view;
mod curved_header_profile_scroll_view;
mod ferro_flix_detail_view;
mod ferro_flix_home_view;
mod ferro_flix_search_view;
mod lavenir_dish_detail_view;
mod lavenir_menu_view;
mod lavenir_profile_view;
mod lavenir_reservations_view;
mod navigation_page_appearance_page;
mod navigation_page_attached_methods_page;
mod navigation_page_events_page;
mod navigation_page_first_look_page;
mod navigation_page_gesture_page;
mod navigation_page_modal_page;
mod navigation_page_modal_transitions_page;
mod navigation_page_stack_page;
mod navigation_page_title_page;
mod navigation_page_toolbar_page;
mod pulse_home_view;
mod pulse_login_view;
mod pulse_profile_view;
mod pulse_workout_detail_view;
mod pulse_workouts_view;
mod retro_gaming_detail_view;
mod retro_gaming_favorites_view;
mod retro_gaming_games_view;
mod retro_gaming_home_view;
mod retro_gaming_profile_view;
mod retro_gaming_search_view;

pub use curved_header_home_scroll_view::CurvedHeaderHomeScrollView;
pub use curved_header_profile_scroll_view::CurvedHeaderProfileScrollView;
pub use ferro_flix_detail_view::FerroFlixDetailView;
pub use ferro_flix_home_view::FerroFlixHomeView;
pub use ferro_flix_search_view::FerroFlixSearchView;
pub use lavenir_dish_detail_view::LAvenirDishDetailView;
pub use lavenir_menu_view::{DishSelected, LAvenirMenuView};
pub use lavenir_profile_view::LAvenirProfileView;
pub use lavenir_reservations_view::LAvenirReservationsView;
pub use navigation_page_appearance_page::NavigationPageAppearancePage;
pub use navigation_page_attached_methods_page::NavigationPageAttachedMethodsPage;
pub use navigation_page_events_page::NavigationPageEventsPage;
pub use navigation_page_first_look_page::NavigationPageFirstLookPage;
pub use navigation_page_gesture_page::NavigationPageGesturePage;
pub use navigation_page_modal_page::NavigationPageModalPage;
pub use navigation_page_modal_transitions_page::NavigationPageModalTransitionsPage;
pub use navigation_page_stack_page::NavigationPageStackPage;
pub use navigation_page_title_page::NavigationPageTitlePage;
pub use navigation_page_toolbar_page::NavigationPageToolbarPage;
pub use pulse_home_view::PulseHomeView;
pub use pulse_login_view::PulseLoginView;
pub use pulse_profile_view::PulseProfileView;
pub use pulse_workout_detail_view::PulseWorkoutDetailView;
pub use pulse_workouts_view::PulseWorkoutsView;
pub use retro_gaming_detail_view::RetroGamingDetailView;
pub use retro_gaming_favorites_view::RetroGamingFavoritesView;
pub use retro_gaming_games_view::RetroGamingGamesView;
pub use retro_gaming_home_view::RetroGamingHomeView;
pub use retro_gaming_profile_view::RetroGamingProfileView;
pub use retro_gaming_search_view::RetroGamingSearchView;

pub(crate) const TYPES: &[&TypeInfo] = &[
    CurvedHeaderHomeScrollView::TYPE,
    CurvedHeaderProfileScrollView::TYPE,
    FerroFlixDetailView::TYPE,
    FerroFlixHomeView::TYPE,
    FerroFlixSearchView::TYPE,
    LAvenirDishDetailView::TYPE,
    LAvenirMenuView::TYPE,
    LAvenirProfileView::TYPE,
    LAvenirReservationsView::TYPE,
    NavigationPageAppearancePage::TYPE,
    NavigationPageAttachedMethodsPage::TYPE,
    NavigationPageEventsPage::TYPE,
    NavigationPageFirstLookPage::TYPE,
    NavigationPageGesturePage::TYPE,
    NavigationPageModalPage::TYPE,
    NavigationPageModalTransitionsPage::TYPE,
    NavigationPageStackPage::TYPE,
    NavigationPageTitlePage::TYPE,
    NavigationPageToolbarPage::TYPE,
    PulseHomeView::TYPE,
    PulseLoginView::TYPE,
    PulseProfileView::TYPE,
    PulseWorkoutDetailView::TYPE,
    PulseWorkoutsView::TYPE,
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
    &FerroFlixDetailView::XAML_CLASS,
    &FerroFlixHomeView::XAML_CLASS,
    &FerroFlixSearchView::XAML_CLASS,
    &LAvenirDishDetailView::XAML_CLASS,
    &LAvenirMenuView::XAML_CLASS,
    &LAvenirProfileView::XAML_CLASS,
    &LAvenirReservationsView::XAML_CLASS,
    &NavigationPageAppearancePage::XAML_CLASS,
    &NavigationPageAttachedMethodsPage::XAML_CLASS,
    &NavigationPageEventsPage::XAML_CLASS,
    &NavigationPageFirstLookPage::XAML_CLASS,
    &NavigationPageGesturePage::XAML_CLASS,
    &NavigationPageModalPage::XAML_CLASS,
    &NavigationPageModalTransitionsPage::XAML_CLASS,
    &NavigationPageStackPage::XAML_CLASS,
    &NavigationPageTitlePage::XAML_CLASS,
    &NavigationPageToolbarPage::XAML_CLASS,
    &PulseHomeView::XAML_CLASS,
    &PulseLoginView::XAML_CLASS,
    &PulseProfileView::XAML_CLASS,
    &PulseWorkoutDetailView::XAML_CLASS,
    &PulseWorkoutsView::XAML_CLASS,
    &RetroGamingDetailView::XAML_CLASS,
    &RetroGamingFavoritesView::XAML_CLASS,
    &RetroGamingGamesView::XAML_CLASS,
    &RetroGamingHomeView::XAML_CLASS,
    &RetroGamingProfileView::XAML_CLASS,
    &RetroGamingSearchView::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
