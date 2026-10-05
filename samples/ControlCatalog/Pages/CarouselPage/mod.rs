//! The samples of the `CarouselPage and Carousel` gallery (directory `Pages/CarouselPage`): one
//! module per upstream code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod carousel_customization_page;
mod carousel_data_binding_page;
mod carousel_gallery_app_page;
mod carousel_gestures_page;
mod carousel_getting_started_page;
mod carousel_multi_item_page;
mod carousel_page_customization_page;
mod carousel_page_data_template_page;
mod carousel_page_events_page;
mod carousel_page_first_look_page;
mod carousel_page_gesture_page;
mod carousel_page_selection_page;
mod carousel_page_transitions_page;
mod carousel_transitions_page;
mod carousel_vertical_page;
mod sanctuary_main_page;
mod sanctuary_showcase_page;

pub use carousel_customization_page::CarouselCustomizationPage;
pub use carousel_data_binding_page::{CarouselCardItem, CarouselDataBindingPage};
pub use carousel_gallery_app_page::CarouselGalleryAppPage;
pub use carousel_gestures_page::CarouselGesturesPage;
pub use carousel_getting_started_page::CarouselGettingStartedPage;
pub use carousel_multi_item_page::CarouselMultiItemPage;
pub use carousel_page_customization_page::CarouselPageCustomizationPage;
pub use carousel_page_data_template_page::CarouselPageDataTemplatePage;
pub use carousel_page_events_page::CarouselPageEventsPage;
pub use carousel_page_first_look_page::CarouselPageFirstLookPage;
pub use carousel_page_gesture_page::CarouselPageGesturePage;
pub use carousel_page_selection_page::CarouselPageSelectionPage;
pub use carousel_page_transitions_page::CarouselPageTransitionsPage;
pub use carousel_transitions_page::CarouselTransitionsPage;
pub use carousel_vertical_page::CarouselVerticalPage;
pub use sanctuary_main_page::SanctuaryMainPage;
pub use sanctuary_showcase_page::SanctuaryShowcasePage;

pub(crate) const TYPES: &[&TypeInfo] = &[
    CarouselCustomizationPage::TYPE,
    CarouselDataBindingPage::TYPE,
    CarouselGalleryAppPage::TYPE,
    CarouselGesturesPage::TYPE,
    CarouselGettingStartedPage::TYPE,
    CarouselMultiItemPage::TYPE,
    CarouselPageCustomizationPage::TYPE,
    CarouselPageDataTemplatePage::TYPE,
    CarouselPageEventsPage::TYPE,
    CarouselPageFirstLookPage::TYPE,
    CarouselPageGesturePage::TYPE,
    CarouselPageSelectionPage::TYPE,
    CarouselPageTransitionsPage::TYPE,
    CarouselTransitionsPage::TYPE,
    CarouselVerticalPage::TYPE,
    SanctuaryMainPage::TYPE,
    SanctuaryShowcasePage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &CarouselCustomizationPage::XAML_CLASS,
    &CarouselDataBindingPage::XAML_CLASS,
    &CarouselGalleryAppPage::XAML_CLASS,
    &CarouselGesturesPage::XAML_CLASS,
    &CarouselGettingStartedPage::XAML_CLASS,
    &CarouselMultiItemPage::XAML_CLASS,
    &CarouselPageCustomizationPage::XAML_CLASS,
    &CarouselPageDataTemplatePage::XAML_CLASS,
    &CarouselPageEventsPage::XAML_CLASS,
    &CarouselPageFirstLookPage::XAML_CLASS,
    &CarouselPageGesturePage::XAML_CLASS,
    &CarouselPageSelectionPage::XAML_CLASS,
    &CarouselPageTransitionsPage::XAML_CLASS,
    &CarouselTransitionsPage::XAML_CLASS,
    &CarouselVerticalPage::XAML_CLASS,
    &SanctuaryMainPage::XAML_CLASS,
    &SanctuaryShowcasePage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <CarouselCardItem as MarkupTyped>::MARKUP,
];
