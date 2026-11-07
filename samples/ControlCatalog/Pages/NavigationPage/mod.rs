//! The samples of the `NavigationPage` gallery (directory `Pages/NavigationPage`): one module per
//! upstream file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

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
mod navigation_page_performance_page;
mod navigation_page_stack_page;
mod navigation_page_title_page;
mod navigation_page_toolbar_page;
mod navigation_page_transitions_page;
#[path = "Transitions/mod.rs"]
mod transitions;

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
pub use navigation_page_performance_page::NavigationPagePerformancePage;
pub use navigation_page_stack_page::NavigationPageStackPage;
pub use navigation_page_title_page::NavigationPageTitlePage;
pub use navigation_page_toolbar_page::NavigationPageToolbarPage;
pub use navigation_page_transitions_page::NavigationPageTransitionsPage;
pub use transitions::{
    CompositeTransition, FadeThroughTransition, PageSlideTransition, PageSlideTransitionAxis, ParallaxSlideTransition,
};

pub(crate) const TYPES: &[&TypeInfo] = &[
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
    NavigationPagePerformancePage::TYPE,
    NavigationPageStackPage::TYPE,
    NavigationPageTitlePage::TYPE,
    NavigationPageToolbarPage::TYPE,
    NavigationPageTransitionsPage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
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
    &NavigationPagePerformancePage::XAML_CLASS,
    &NavigationPageStackPage::XAML_CLASS,
    &NavigationPageTitlePage::XAML_CLASS,
    &NavigationPageToolbarPage::XAML_CLASS,
    &NavigationPageTransitionsPage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <NavigationPageMvvmShellViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
    <ProjectCardViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
];
