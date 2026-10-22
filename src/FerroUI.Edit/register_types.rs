use ferroui_base::TypeInfo;
use ferroui_base::metadata::{FERRO_XML_NAMESPACE, MarkupAssembly, XmlnsDefinition};

pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Edit",
    crate_name: "ferroui_edit",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroUI.Edit",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// Registers the editor with runtime and compiled XAML metadata.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(&[
            ("ferroui_edit", "FerroUI.Edit"),
            ("ferroui_edit::automation", "FerroUI.Edit.Automation"),
        ]);
        TypeInfo::register_all(&[
            crate::TextEditor::TYPE,
            crate::TextArea::TYPE,
            crate::SearchPanel::TYPE,
            crate::CompletionWindow::TYPE,
            crate::automation::TextEditorAutomationPeer::TYPE,
        ]);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
    });
}
