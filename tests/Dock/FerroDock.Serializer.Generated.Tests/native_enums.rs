//! Native semantic headers for original enum contracts used by the SDK properties.
//! The source generator reads enum kind/inheritance, not enum fields or methods.
//! Actual constants and value operations remain on the original compiled model.
use super::*;
use ferroui_build_scan::{
    model::{TypeKind as MetadataKind, TypeModel},
    scanner::{scan_crate, ScanOptions, Severity},
};

pub(super) const NAMES: [&str; 5] = [
    "DockMode",
    "PinnedDockDisplayMode",
    "DockFloatingWindowHostMode",
    "DockWindowState",
    "DockWindowOwnerMode",
];

pub(super) struct Enum {
    pub metadata: TypeModel,
    pub source: native_adapter::RustSourceDeclaration,
}

pub(super) fn read(workspace: &std::path::Path) -> Vec<Enum> {
    let mut options = ScanOptions::new(
        "ferrodock_model",
        workspace.join("src/Dock/FerroDock.Model/lib.rs"),
    );
    options.extern_crates = [
        "ferroui_system",
        "ferroui_reactiveui",
        "ferrodock_controls_recycling_model",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let scan = scan_crate(&options);
    let errors: Vec<_> = scan.diagnostics_of(Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    for file in &scan.files {
        println!("cargo::rerun-if-changed={}", file.path.display());
    }
    let source = native_adapter::read_source_declarations(&scan).unwrap();
    NAMES
        .into_iter()
        .map(|name| {
            let path = scan
                .normalise(&scan.model.crate_name, &format!("crate::core::{name}"))
                .unwrap();
            let source = source.definition(&path).unwrap().clone();
            assert_eq!(
                source.visibility,
                native_adapter::RustDeclarationVisibility::Public
            );
            assert!(source.cfg.is_empty() && source.parameters.is_empty());
            let mut metadata = TypeModel::new(
                name,
                MetadataKind::Enum,
                source.path.clone(),
                &source.module,
            );
            if source.kind == native_adapter::RustDeclarationKind::Struct {
                assert_eq!(source.transparent_field.as_ref().unwrap().text, "i32");
                // This explicit source API binding is checked against the shared scanner's
                // actual function facts. The compiler does not invent a constructor.
                let functions: Vec<_> = scan
                    .functions
                    .iter()
                    .filter(|f| f.owner == source.path.text && f.name == "from_raw")
                    .collect();
                assert_eq!(functions.len(), 1);
                let function = functions[0];
                assert_eq!(function.visibility, "pub");
                assert!(!function.receiver && function.declared_by.is_none());
                assert_eq!(function.parameters, ["i32"]);
                assert_eq!(function.return_type.as_deref(), Some("Self"));
                metadata.public_path = Some(source.path.text.clone());
                metadata.enum_i32_factory = Some(function.name.clone());
            } else {
                assert_eq!(source.kind, native_adapter::RustDeclarationKind::Enum);
            }
            Enum { metadata, source }
        })
        .collect()
}
