//! Font collections, font family loading and OpenType table readers.

mod bcp47_script_resolver;
mod embedded_font_collection;
mod empty_system_font_collection;
mod family_name_collection;
mod font_code_page_coverage;
pub(crate) mod font_collection_base;
mod font_collection_key;
mod font_fallback_script_hints;
mod font_family_key;
mod font_family_loader;
mod i_font_collection;
mod open_type_tag;
mod system_font_collection;
mod unmanaged_font_memory;

pub mod tables;

#[cfg(any(test, feature = "testing"))]
pub mod testing;

#[cfg(test)]
mod font_collection_tests;

#[allow(unused_imports)] // kept for parity; nothing outside of the font fallback hints uses it yet
pub(crate) use bcp47_script_resolver::Bcp47ScriptResolver;
pub use embedded_font_collection::EmbeddedFontCollection;
pub(crate) use empty_system_font_collection::EmptySystemFontCollection;
pub use family_name_collection::FamilyNameCollection;
pub use font_code_page_coverage::FontCodePageCoverage;
pub use font_collection_base::{
    FontCollectionBase, FontCollectionBaseImpl, GlyphTypefaceCache, GlyphTypefaceMap, IFontCollectionBase,
};
pub use font_collection_key::FontCollectionKey;
pub(crate) use font_fallback_script_hints::FontFallbackScriptHints;
pub use font_family_key::FontFamilyKey;
#[allow(unused_imports)] // kept for parity; only the font collections use it
pub(crate) use font_family_loader::FontFamilyLoader;
pub use i_font_collection::IFontCollection;
pub use open_type_tag::OpenTypeTag;
pub use system_font_collection::SystemFontCollection;
#[allow(unused_imports)] // used by font backends, which are separate crates
pub(crate) use unmanaged_font_memory::UnmanagedFontMemory;
