//! WinUI access keys for FerroUI: `AccessKey`, scopes, the Alt activation, the Key Tip overlay and
//! invoking a control by its key. An application that does not use the crate links none of it.

mod access_key_manager;
mod key_tip_math;
mod key_tip_overlay;

#[cfg(test)]
mod tests;

pub use access_key_manager::{AccessKeyInvokedEventArgs, AccessKeyManager};
pub use key_tip_math::KeyTipPlacementMode;
pub use key_tip_overlay::KeyTipOverlay;

// Shared by the registration at run time and the scanner of the build-time XAML metadata.
const NAMESPACES: &[(&str, &str)] = &[("ferroui_controls_keytips", "FerroUI.Controls.KeyTips")];
pub static ASSEMBLY: ferroui_base::metadata::MarkupAssembly =
    ferroui_base::metadata::MarkupAssembly {
        name: "FerroUI.Controls.KeyTips",
        crate_name: "ferroui_controls_keytips",
        xmlns_definitions: &[ferroui_base::metadata::XmlnsDefinition {
            xml_namespace: ferroui_base::metadata::FERRO_XML_NAMESPACE,
            namespace: "FerroUI.Controls.KeyTips",
        }],
        xmlns_prefixes: &[],
        metadata: &[],
    };
ferroui_base::ferro_rust_paths! {
    classes: [crate::KeyTipOverlay],
    types: [crate::AccessKeyManager, crate::KeyTipPlacementMode],
    contracts: [], generics: [], generic_contracts: [],
}
const TYPES: &[&ferroui_base::TypeInfo] = &[
    KeyTipOverlay::TYPE,
    <AccessKeyManager as ferroui_base::StaticType>::TYPE,
];

/// Registers the namespace, the assembly and the table of types of this crate and of the crates
/// it is built on. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        ferroui_base::TypeInfo::register_namespaces(NAMESPACES);
        ferroui_base::metadata::MarkupAssembly::register_table(&ASSEMBLY);
        register_rust_paths();
        ferroui_base::TypeInfo::register_table(TYPES);
        ferroui_base::metadata::MarkupType::register_table(&[
            <KeyTipPlacementMode as ferroui_base::metadata::MarkupTyped>::MARKUP,
        ]);
    });
}
