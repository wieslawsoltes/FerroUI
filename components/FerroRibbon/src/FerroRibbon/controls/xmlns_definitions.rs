//! XML namespace declarations from RibbonSpace 9755e281, XmlnsDefinitions.cs.
//! Copyright Wiesław Šoltés. Distributed under the MIT license; see NOTICE.md.

/// Public namespace identifiers for ribbon markup.
///
/// Native markup declares its prefixes explicitly. Unprefixed ribbon controls
/// also resolve in FerroUI's default XML namespace through assembly metadata.
pub struct RibbonXmlns;

impl RibbonXmlns {
    /// Stable public XML namespace, retained from the source library.
    pub const URI: &str = "https://github.com/wieslawsoltes/RibbonSpace";
    /// Recommended prefix. A document supplies its `xmlns:rs` declaration.
    pub const PREFIX: &str = "rs";
    /// Original global namespace, also accepted as an explicit native alias.
    pub const GLOBAL_URI: &str = "http://schemas.microsoft.com/winfx/2006/xaml/presentation/global";
}

ferroui_base::ferro_markup_type!(static RibbonXmlns {
    namespace: "FerroRibbon.Controls",
    fields: [
        Uri: String => || ferroribbon::controls::RibbonXmlns::URI.to_owned(),
        Prefix: String => || ferroribbon::controls::RibbonXmlns::PREFIX.to_owned(),
        GlobalUri: String => || ferroribbon::controls::RibbonXmlns::GLOBAL_URI.to_owned(),
    ],
});
