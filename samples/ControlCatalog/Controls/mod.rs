//! The shared controls of the catalog pages (namespace `ControlCatalog.Controls`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;
use ferroui_controls::ItemsSource;
use std::rc::Rc;

mod card_grid;
mod code_block;
mod home_item_expander;
mod sample_gallery_page;
mod sample_group;
mod sample_info;
mod sample_page;
mod sample_section;
mod section_control;
mod selectable_button;

pub use card_grid::CardGrid;
pub use code_block::{CodeBlock, CodeLanguage};
pub use home_item_expander::HomeItemExpander;
pub use sample_gallery_page::SampleGalleryPage;
pub use sample_group::SampleGroup;
pub use sample_info::{SampleGroups, SampleInfo, SampleInfoGroup};
#[allow(unused_imports)]
pub(crate) use sample_page::sample_page_class;
pub use sample_page::SamplePage;
pub use sample_section::{SampleSection, SampleStage};
pub use section_control::SectionControl;
pub use selectable_button::{SelectableButton, SelectableButtonImpl};

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[
    CardGrid::TYPE,
    CodeBlock::TYPE,
    HomeItemExpander::TYPE,
    SampleGalleryPage::TYPE,
    SampleGroup::TYPE,
    SamplePage::TYPE,
    SampleSection::TYPE,
    SectionControl::TYPE,
    SelectableButton::TYPE,
];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&SectionControl::XAML_CLASS];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <CodeLanguage as MarkupTyped>::MARKUP,
    <SampleInfo as MarkupTyped>::MARKUP,
    <SampleInfoGroup as MarkupTyped>::MARKUP,
    <SampleStage as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<SampleInfo>();
    ValueTypes::register_reference::<SampleInfoGroup>();
    ValueTypes::register_nullable::<CodeLanguage>();
    ValueTypes::register_nullable::<SampleStage>();
    // The lists of a gallery page are the items sources of the items controls of its theme.
    register_items_source::<Rc<SampleInfo>>();
    register_items_source::<Rc<SampleInfoGroup>>();
}

/// Lets bindings deliver a list of `T` to an items source property.
fn register_items_source<T: Clone + PartialEq + 'static>() {
    ValueTypes::register_conversion::<FerroList<T>, ItemsSource>(|list| Some(ItemsSource::new(Rc::new(list.clone()))));
    ValueTypes::register_conversion::<FerroList<T>, Option<ItemsSource>>(|list| {
        Some(Some(ItemsSource::new(Rc::new(list.clone()))))
    });
}
