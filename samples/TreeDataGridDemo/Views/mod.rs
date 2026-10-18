//! The views of the demo (namespace `TreeDataGridDemo.Views`): the pages of the main window,
//! which the ControlCatalog shows too.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod bring_into_view_non_uniform_rows_page;
mod core_demo_page;
mod countries_page;
mod drag_drop_page;
mod files_page;
mod find_displayed_row_index_page;
mod main_view;
mod people_xaml_page;
mod realized_count;
mod template_column_bug_page;
mod wikipedia_page;

pub use bring_into_view_non_uniform_rows_page::BringIntoViewNonUniformRowsPage;
pub use core_demo_page::CoreDemoPage;
pub use countries_page::CountriesPage;
pub use drag_drop_page::DragDropPage;
pub use files_page::FilesPage;
pub use find_displayed_row_index_page::FindDisplayedRowIndexPage;
pub use main_view::MainView;
pub use people_xaml_page::PeopleXamlPage;
pub use template_column_bug_page::TemplateColumnBugPage;
pub use wikipedia_page::WikipediaPage;
pub use realized_count::update_realized_count;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[CountriesPage::TYPE, MainView::TYPE, PeopleXamlPage::TYPE, TemplateColumnBugPage::TYPE, WikipediaPage::TYPE, DragDropPage::TYPE,
    FilesPage::TYPE,
    BringIntoViewNonUniformRowsPage::TYPE,
    CoreDemoPage::TYPE,
    FindDisplayedRowIndexPage::TYPE,
];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[
    &CountriesPage::XAML_CLASS,
    &MainView::XAML_CLASS,
    &PeopleXamlPage::XAML_CLASS,
    &TemplateColumnBugPage::XAML_CLASS,
    &WikipediaPage::XAML_CLASS,
    &DragDropPage::XAML_CLASS,
    &FilesPage::XAML_CLASS,
    &BringIntoViewNonUniformRowsPage::XAML_CLASS,
    &CoreDemoPage::XAML_CLASS,
    &FindDisplayedRowIndexPage::XAML_CLASS,
];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
