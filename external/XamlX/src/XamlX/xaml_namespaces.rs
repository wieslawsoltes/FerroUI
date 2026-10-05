//! Port of `XamlNamespaces.cs`.

/// Well-known XML namespaces.
pub struct XamlNamespaces;

impl XamlNamespaces {
    pub const XAML2006: &'static str = "http://schemas.microsoft.com/winfx/2006/xaml";
    pub const BLEND2008: &'static str = "http://schemas.microsoft.com/expression/blend/2008";
}
