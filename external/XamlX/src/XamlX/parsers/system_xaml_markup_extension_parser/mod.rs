//! Port of the `Parsers/SystemXamlMarkupExtensionParser/` directory.
//!
//! The scanner originates from System.Xaml (licensed to the .NET Foundation under the MIT
//! license) and is kept close to the original, as upstream does.

mod known_strings;
mod me_scanner;
mod me_scanner_shims;
#[allow(clippy::module_inception)]
mod system_xaml_markup_extension_parser;

pub use known_strings::*;
pub use me_scanner::*;
pub use me_scanner_shims::*;
pub use system_xaml_markup_extension_parser::*;
