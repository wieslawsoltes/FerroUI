//! The models of the demo (namespace `TreeDataGridDemo.Models`): one module per reference file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod countries;
mod country;
mod drag_drop_item;
mod file_tree_node_model;
mod on_this_day;
mod person;
mod template_column_item;
mod wikipedia_http_client;

pub use countries::Countries;
pub use country::Country;
pub use drag_drop_item::DragDropItem;
pub use file_tree_node_model::{DefaultComparer, FileTreeNodeComparison, FileTreeNodeModel};
pub use on_this_day::{
    OnThisDay, OnThisDayArticle, OnThisDayEvent, OnThisDayImage, OnThisDayJsonSerializerContext, OnThisDayTitles,
};
pub use person::Person;
pub use template_column_item::TemplateColumnItem;
pub use wikipedia_http_client::{
    HttpClient, HttpClientHandler, HttpMessageHandler, HttpRequestException, HttpRequestMessage, HttpResponseMessage,
    WikipediaHttpClient,
};

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <Country as MarkupTyped>::MARKUP,
    <Countries as MarkupTyped>::MARKUP,
    <Person as MarkupTyped>::MARKUP,
    <TemplateColumnItem as MarkupTyped>::MARKUP,
    <OnThisDayArticle as MarkupTyped>::MARKUP,
    <DragDropItem as MarkupTyped>::MARKUP,
    <FileTreeNodeModel as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<Country>();
    ValueTypes::register_reference::<Person>();
    ValueTypes::register_reference::<TemplateColumnItem>();
    ValueTypes::register_reference::<OnThisDayArticle>();
    ValueTypes::register_reference::<DragDropItem>();
    ValueTypes::register_reference::<FileTreeNodeModel>();
}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
