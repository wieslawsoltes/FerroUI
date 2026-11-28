//! Typed constants for source enum literals in the library's own compiled themes.
//! The current enum export is completed after the model compiler scans this crate;
//! a typed field lets that initial scan assign the original Core value directly.
pub struct RibbonTemplateValues;
ferroui_base::ferro_markup_type!(static RibbonTemplateValues {
    namespace:"FerroRibbon.Internal",
    fields:[LargeItemSize:ferroribbon_core::RibbonItemSize => ||ferroribbon_core::RibbonItemSize::Large],
});
