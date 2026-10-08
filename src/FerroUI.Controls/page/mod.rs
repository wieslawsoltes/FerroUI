//! Pages and page navigation.

mod bar_layout_behavior;
mod content_page;
mod default_page_data_template;
mod i_navigation;
mod modal_popped_event_args;
mod modal_pushed_event_args;
mod navigated_from_event_args;
mod navigated_to_event_args;
mod navigating_from_event_args;
mod navigation_event_args;
mod navigation_page;
mod navigation_type;
#[allow(clippy::module_inception)]
mod page;
mod page_inserted_event_args;
mod page_navigation_extensions;
mod page_navigation_host;
mod page_removed_event_args;
mod safe_area_padding_extensions;
mod carousel_page;
mod multi_page;
mod page_selection_changed_event_args;
mod selecting_multi_page;
mod tab_placement;
mod tabbed_page;
mod drawer_behavior;
mod drawer_closing_event_args;
mod drawer_layout_behavior;
mod drawer_page;
mod drawer_placement;

pub use bar_layout_behavior::BarLayoutBehavior;
pub use content_page::ContentPage;
pub(crate) use default_page_data_template::DefaultPageDataTemplate;
pub use i_navigation::INavigation;
pub(crate) use i_navigation::{completed_task, start_async};
pub use modal_popped_event_args::ModalPoppedEventArgs;
pub use modal_pushed_event_args::ModalPushedEventArgs;
pub use navigated_from_event_args::NavigatedFromEventArgs;
pub use navigated_to_event_args::NavigatedToEventArgs;
pub use navigating_from_event_args::NavigatingFromEventArgs;
pub use navigation_event_args::NavigationEventArgs;
pub use navigation_page::NavigationPage;
pub use navigation_type::NavigationType;
pub use page::{NavigatingTask, Page, PageImpl, PageImplExt, PageVTable};
pub use page_inserted_event_args::PageInsertedEventArgs;
pub use page_navigation_extensions::PageNavigationExtensions;
pub use page_navigation_host::PageNavigationHost;
pub use page_removed_event_args::PageRemovedEventArgs;
pub(crate) use safe_area_padding_extensions::SafeAreaPaddingExtensions;
pub use carousel_page::CarouselPage;
pub use multi_page::{
    MultiPage, MultiPageImpl, MultiPageImplExt, MultiPageVTable, PageList, PagesChangedHandler,
};
pub use page_selection_changed_event_args::PageSelectionChangedEventArgs;
pub use selecting_multi_page::{
    SelectingMultiPage, SelectingMultiPageImpl, SelectingMultiPageImplExt, SelectingMultiPageVTable,
};
pub use tab_placement::TabPlacement;
pub use tabbed_page::TabbedPage;
pub use drawer_behavior::DrawerBehavior;
pub use drawer_closing_event_args::DrawerClosingEventArgs;
pub use drawer_layout_behavior::DrawerLayoutBehavior;
pub use drawer_page::DrawerPage;
pub use drawer_placement::DrawerPlacement;

#[cfg(test)]
mod carousel_page_tests;
#[cfg(test)]
mod carousel_page_tests_interaction;
#[cfg(test)]
mod content_page_tests;
#[cfg(test)]
mod navigation_event_args_tests;
#[cfg(test)]
mod navigation_page_tests;
#[cfg(test)]
mod navigation_page_tests_lifecycle;
#[cfg(test)]
mod navigation_page_tests_navigating;
#[cfg(test)]
mod navigation_page_tests_stack;
#[cfg(test)]
mod page_lifetime_tests;
#[cfg(test)]
mod page_navigation_host_tests;
#[cfg(test)]
mod tabbed_page_tests;
#[cfg(test)]
mod tabbed_page_tests_data_template;
#[cfg(test)]
mod drawer_page_tests;
#[cfg(test)]
mod drawer_page_tests_lifecycle;
#[cfg(test)]
mod drawer_page_tests_templates;
