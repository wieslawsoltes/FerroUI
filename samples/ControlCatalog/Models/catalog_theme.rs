//! Port of `Models/CatalogTheme.cs`.

use ferroui_base::ferro_markup_enum;

/// The theme of the catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CatalogTheme {
    Fluent = 0,
    Simple = 1,
}

ferro_markup_enum!(CatalogTheme { Fluent, Simple });
