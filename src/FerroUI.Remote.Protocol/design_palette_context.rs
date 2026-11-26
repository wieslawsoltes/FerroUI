use crate::{bson_class, ferro_remote_message_guid};

pub const DESIGN_PALETTE_QUERY: &str = "ferroui-design-palette";
pub const DESIGN_PALETTE_VERSION: &str = "1";

/// Native theme metadata for an accepted XAML source snapshot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignPaletteContextMessage {
    pub revision: i64,
    /// High contrast followed by its inherited variant keys in lookup order.
    pub contrast: Option<Vec<String>>,
    pub assemblies: Vec<String>,
}

ferro_remote_message_guid!(
    DesignPaletteContextMessage,
    "D2C95A63-AF9C-4AA2-958B-8F49D3D61AD6"
);
bson_class!(DesignPaletteContextMessage as "DesignPaletteContextMessage" {
    "Revision" => revision: i64,
    "Contrast" => contrast: Option<Vec<String>>,
    "Assemblies" => assemblies: Vec<String>,
});
