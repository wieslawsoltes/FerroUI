//! The models of the sample (namespace `VirtualizationDemo.Models`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod chat;

pub use chat::{ChatFile, ChatMessage, ChatMessageList};

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[<ChatMessage as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<ChatMessage>();
}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {
    ChatMessageList::register();
}
