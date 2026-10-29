//! Material gallery pages, one per control family.

use crate::markup::XamlClass;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{Ref, TypeInfo};
use ferroui_controls::Control;

mod material_components_page;
mod material_interactions_page;
mod floating_actions_page;
mod app_bars_page;
mod date_pickers_page;
mod time_pickers_page;
mod buttons_page;
mod toggles_page;
mod text_input_page;
mod number_input_page;
mod combo_box_page;
mod lists_page;
mod range_page;
mod date_time_page;
mod menus_page;
mod flyouts_page;
mod tabs_page;
mod containers_page;
mod navigation_samples_page;
mod table_page;
mod dialogs_page;
mod text_calibration_page;

pub use material_components_page::MaterialComponentsPage;
pub use material_interactions_page::MaterialInteractionsPage;
pub use floating_actions_page::FloatingActionsPage;
pub use app_bars_page::AppBarsPage;
pub use date_pickers_page::DatePickersPage;
pub use time_pickers_page::TimePickersPage;
pub use buttons_page::ButtonsPage;
pub use toggles_page::TogglesPage;
pub use text_input_page::TextInputPage;
pub use number_input_page::NumberInputPage;
pub use combo_box_page::ComboBoxPage;
pub use lists_page::ListsPage;
pub use range_page::RangePage;
pub use date_time_page::DateTimePage;
pub use menus_page::MenusPage;
pub use flyouts_page::FlyoutsPage;
pub use tabs_page::TabsPage;
pub use containers_page::ContainersPage;
pub use navigation_samples_page::NavigationSamplesPage;
pub use table_page::TablePage;
pub use dialogs_page::DialogsPage;
pub use text_calibration_page::TextCalibrationPage;
pub use table_page::TableRow;

/// A page of the gallery.
#[derive(Debug)]
pub struct PageInfo {
    /// The name of the page in the names of the pictures and in the options
    /// (`FERROUI_MATERIAL_GALLERY_PAGES`).
    pub id: &'static str,
    /// The title of the page in the list of the window.
    pub title: &'static str,
    /// The rooted asset path of the document of the page.
    pub document: &'static str,
    /// Creates the page (its constructor loads its document, and panics if it does not load).
    pub create: fn() -> Ref<Control>,
}

impl PageInfo {
    /// The page with the id `id` (without regard to case).
    pub fn find(id: &str) -> Option<&'static PageInfo> {
        PAGES.iter().find(|page| page.id.eq_ignore_ascii_case(id.trim()))
    }
}

/// The pages, in the order of the list of the window.
pub static PAGES: &[PageInfo] = &[
    PageInfo { id: "material", title: "Material 3 Expressive", document: MaterialComponentsPage::DOCUMENT_PATH, create: material_components_page::create },
    PageInfo { id: "interactions", title: "Search, snackbars and sheets", document: MaterialInteractionsPage::DOCUMENT_PATH, create: material_interactions_page::create },
    PageInfo { id: "floating", title: "Floating action buttons", document: FloatingActionsPage::DOCUMENT_PATH, create: floating_actions_page::create },
    PageInfo { id: "appbars", title: "Top app bars", document: AppBarsPage::DOCUMENT_PATH, create: app_bars_page::create },
    PageInfo { id: "datepickers", title: "Material date pickers", document: DatePickersPage::DOCUMENT_PATH, create: date_pickers_page::create },
    PageInfo { id: "daterange", title: "Material date range", document: DatePickersPage::DOCUMENT_PATH, create: date_pickers_page::create_range },
    PageInfo { id: "timepickers", title: "Material time pickers", document: TimePickersPage::DOCUMENT_PATH, create: time_pickers_page::create },
    PageInfo {
        id: "buttons",
        title: "Buttons",
        document: ButtonsPage::DOCUMENT_PATH,
        create: buttons_page::create,
    },
    PageInfo {
        id: "toggles",
        title: "Check boxes and toggles",
        document: TogglesPage::DOCUMENT_PATH,
        create: toggles_page::create,
    },
    PageInfo {
        id: "text",
        title: "Text input",
        document: TextInputPage::DOCUMENT_PATH,
        create: text_input_page::create,
    },
    PageInfo {
        id: "numbers",
        title: "Numbers and suggestions",
        document: NumberInputPage::DOCUMENT_PATH,
        create: number_input_page::create,
    },
    PageInfo {
        id: "combobox",
        title: "Combo box",
        document: ComboBoxPage::DOCUMENT_PATH,
        create: combo_box_page::create,
    },
    PageInfo {
        id: "lists",
        title: "Lists and trees",
        document: ListsPage::DOCUMENT_PATH,
        create: lists_page::create,
    },
    PageInfo {
        id: "range",
        title: "Sliders, progress and scrolling",
        document: RangePage::DOCUMENT_PATH,
        create: range_page::create,
    },
    PageInfo {
        id: "datetime",
        title: "Date and time",
        document: DateTimePage::DOCUMENT_PATH,
        create: date_time_page::create,
    },
    PageInfo {
        id: "menus",
        title: "Menus and command bars",
        document: MenusPage::DOCUMENT_PATH,
        create: menus_page::create,
    },
    PageInfo {
        id: "flyouts",
        title: "Flyouts, tool tips and notifications",
        document: FlyoutsPage::DOCUMENT_PATH,
        create: flyouts_page::create,
    },
    PageInfo {
        id: "tabs",
        title: "Tabs",
        document: TabsPage::DOCUMENT_PATH,
        create: tabs_page::create,
    },
    PageInfo {
        id: "containers",
        title: "Containers",
        document: ContainersPage::DOCUMENT_PATH,
        create: containers_page::create,
    },
    PageInfo {
        id: "navigation",
        title: "Pages and navigation",
        document: NavigationSamplesPage::DOCUMENT_PATH,
        create: navigation_samples_page::create,
    },
    PageInfo {
        id: "table",
        title: "Table view",
        document: TablePage::DOCUMENT_PATH,
        create: table_page::create,
    },
    PageInfo {
        id: "dialogs",
        title: "Dialogs",
        document: DialogsPage::DOCUMENT_PATH,
        create: dialogs_page::create,
    },
    PageInfo {
        id: "textcal",
        title: "Text calibration",
        document: TextCalibrationPage::DOCUMENT_PATH,
        create: text_calibration_page::create,
    },
];

/// The classes of the namespace `MaterialThemeGallery.Pages` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[
    MaterialComponentsPage::TYPE,
    MaterialInteractionsPage::TYPE,
    FloatingActionsPage::TYPE,
    AppBarsPage::TYPE,
    DatePickersPage::TYPE,
    TimePickersPage::TYPE,
    ButtonsPage::TYPE,
    TogglesPage::TYPE,
    TextInputPage::TYPE,
    NumberInputPage::TYPE,
    ComboBoxPage::TYPE,
    ListsPage::TYPE,
    RangePage::TYPE,
    DateTimePage::TYPE,
    MenusPage::TYPE,
    FlyoutsPage::TYPE,
    TabsPage::TYPE,
    ContainersPage::TYPE,
    NavigationSamplesPage::TYPE,
    TablePage::TYPE,
    DialogsPage::TYPE,
    TextCalibrationPage::TYPE,
];

/// The classes of the namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[
    &MaterialComponentsPage::XAML_CLASS,
    &MaterialInteractionsPage::XAML_CLASS,
    &FloatingActionsPage::XAML_CLASS,
    &AppBarsPage::XAML_CLASS,
    &DatePickersPage::XAML_CLASS,
    &TimePickersPage::XAML_CLASS,
    &ButtonsPage::XAML_CLASS,
    &TogglesPage::XAML_CLASS,
    &TextInputPage::XAML_CLASS,
    &NumberInputPage::XAML_CLASS,
    &ComboBoxPage::XAML_CLASS,
    &ListsPage::XAML_CLASS,
    &RangePage::XAML_CLASS,
    &DateTimePage::XAML_CLASS,
    &MenusPage::XAML_CLASS,
    &FlyoutsPage::XAML_CLASS,
    &TabsPage::XAML_CLASS,
    &ContainersPage::XAML_CLASS,
    &NavigationSamplesPage::XAML_CLASS,
    &TablePage::XAML_CLASS,
    &DialogsPage::XAML_CLASS,
    &TextCalibrationPage::XAML_CLASS,
];

/// The types of the namespace declared with `ferro_markup_type!` (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[<TableRow as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ferroui_base::data::core::ValueTypes::register_reference::<TableRow>();
}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`): none.
pub(crate) fn register_lists() {}
