use crate::{bson_class, ferro_remote_message_guid};

pub const DESIGN_TEMPLATES_QUERY: &str = "ferroui-design-templates";
pub const DESIGN_TEMPLATES_VERSION: &str = "1";

/// A location in the exact source bytes used by the loader or the Rust view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignTemplateSource {
    pub address: String,
    /// The 64-bit source fingerprint carried as a signed BSON integer.
    pub fingerprint: i64,
    pub line: i32,
    pub column: i32,
}

bson_class!(DesignTemplateSource as "DesignTemplateSource" {
    "Address" => address: String,
    "Fingerprint" => fingerprint: i64,
    "Line" => line: i32,
    "Column" => column: i32,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignTemplateUse {
    pub owner: DesignTemplateSource,
    pub type_name: String,
    /// An opaque template still counts as a use of the owner's source.
    pub definition: Option<DesignTemplateSource>,
    /// The native property name. Older control-template catalogs omit it.
    pub member: Option<String>,
    /// Zero-based declarative column index when the Rust owner is a table.
    pub column: Option<i32>,
    /// Compiled Rust constructor, verified by an available source fingerprint.
    pub rust_factory: Option<DesignTemplateSource>,
}

bson_class!(DesignTemplateUse as "DesignTemplateUse" {
    "Owner" => owner: DesignTemplateSource,
    "TypeName" => type_name: String,
    "Definition" => definition: Option<DesignTemplateSource>,
    "Member" => member: Option<String>,
    "Column" => column: Option<i32>,
    "RustFactory" => rust_factory: Option<DesignTemplateSource>,
});

/// Effective templates captured when the application accepts a source snapshot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignTemplateCatalogMessage {
    pub revision: i64,
    pub templates: Vec<DesignTemplateUse>,
}

ferro_remote_message_guid!(
    DesignTemplateCatalogMessage,
    "5817D3A7-7AC8-47C0-A5A0-76DF2207C5F6"
);
bson_class!(DesignTemplateCatalogMessage as "DesignTemplateCatalogMessage" {
    "Revision" => revision: i64,
    "Templates" => templates: Vec<DesignTemplateUse>,
});
