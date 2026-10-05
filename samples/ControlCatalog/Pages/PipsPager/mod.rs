//! The samples of the `PipsPager` gallery (directory `Pages/PipsPager`): one
//! module per upstream code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod pips_pager_carousel_page;
mod pips_pager_custom_button_themes_page;
mod pips_pager_custom_colors_page;
mod pips_pager_custom_templates_page;
mod pips_pager_events_page;
mod pips_pager_getting_started_page;
mod pips_pager_large_collection_page;

pub use pips_pager_carousel_page::PipsPagerCarouselPage;
pub use pips_pager_custom_button_themes_page::PipsPagerCustomButtonThemesPage;
pub use pips_pager_custom_colors_page::PipsPagerCustomColorsPage;
pub use pips_pager_custom_templates_page::PipsPagerCustomTemplatesPage;
pub use pips_pager_events_page::PipsPagerEventsPage;
pub use pips_pager_getting_started_page::PipsPagerGettingStartedPage;
pub use pips_pager_large_collection_page::PipsPagerLargeCollectionPage;

pub(crate) const TYPES: &[&TypeInfo] = &[
    PipsPagerCarouselPage::TYPE,
    PipsPagerCustomButtonThemesPage::TYPE,
    PipsPagerCustomColorsPage::TYPE,
    PipsPagerCustomTemplatesPage::TYPE,
    PipsPagerEventsPage::TYPE,
    PipsPagerGettingStartedPage::TYPE,
    PipsPagerLargeCollectionPage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &PipsPagerCarouselPage::XAML_CLASS,
    &PipsPagerCustomButtonThemesPage::XAML_CLASS,
    &PipsPagerCustomColorsPage::XAML_CLASS,
    &PipsPagerCustomTemplatesPage::XAML_CLASS,
    &PipsPagerEventsPage::XAML_CLASS,
    &PipsPagerGettingStartedPage::XAML_CLASS,
    &PipsPagerLargeCollectionPage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
