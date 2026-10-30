// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Compile real SDK property callbacks across its declared base-view hierarchy.
use ferrodock_serializer_system_text_json_generators_rust::{
    bind_declared_properties, metadata_instance_type, project_property_bindings,
    RustAccessorErrorMapping, RustPropertyMetadataOptions,
};
use ferroui_build_scan::{
    model::{RustType, TypeModel},
    model_set::ModelSet,
    scanner::{scan_crate, Scan, ScanOptions, Severity},
};
use std::{fmt::Write, path::PathBuf};

pub(super) fn inherited(
    declaration: &TypeModel,
    scan: &Scan,
    models: &ModelSet,
    out: &mut String,
) -> Vec<ferrodock_serializer_system_text_json_generators::RustPropertyBinding> {
    writeln!(out, "{:?},", declaration.name).unwrap();
    let error_map = RustAccessorErrorMapping::Map(
        "::ferrodock_model::core::DockSerializationError::new".into(),
    );
    let mut properties = bind_declared_properties(
        declaration,
        &RustPropertyMetadataOptions {
            emitting_crate: "ferrodock_markup_tests",
            declaring_crate: &scan.model.crate_name,
            object_types: &[],
            converters: &[],
            accessor_error_mapper: Some(&error_map),
        },
    )
    .unwrap();
    if let Some(base) = &declaration.base {
        let base = scan
            .model
            .types
            .iter()
            .find(|t| models.canonical(&t.rust_path.text) == models.canonical(&base.text))
            .expect("declared base metadata");
        let source = &metadata_instance_type(declaration).unwrap();
        let target = &metadata_instance_type(base).unwrap();
        assert!(
            scan.model.casts.iter().any(|cast| {
                models.canonical(&cast.from.text) == models.canonical(&source.text)
                    && models.canonical(&cast.to.text) == models.canonical(&target.text)
            }),
            "missing actual registered base cast"
        );
        // Find the real public callable by its owner and exact return type, never by name.
        let functions: Vec<_> = scan
            .functions
            .iter()
            .filter(|f| {
                f.visibility == "pub"
                    && f.receiver
                    && f.parameters.is_empty()
                    && models.canonical(&f.owner) == models.canonical(&declaration.rust_path.text)
                    && f.return_type
                        .as_ref()
                        .and_then(|ty| scan.normalise(&f.module, ty))
                        .is_some_and(|ty| {
                            ty.is_resolved()
                                && models.canonical(&ty.text) == models.canonical(&target.text)
                        })
            })
            .collect();
        assert_eq!(
            functions.len(),
            1,
            "unique typed base conversion for {}",
            declaration.name
        );
        let call = format!(
            "{}::{}",
            declaration.public_path.as_ref().unwrap(),
            functions[0].name
        );
        let base_properties = inherited(base, scan, models, out);
        properties
            .extend(project_property_bindings(&base_properties, target, source, &call).unwrap());
    }
    properties
}

pub(super) fn emit() {
    let here = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = here.join("../../../src/Dock/FerroDock.Model.ReactiveUI.Ferro/lib.rs");
    let mut options = ScanOptions::new("ferrodock_model_reactiveui_ferro", root);
    options.extern_crates = [
        "ferrodock_model_ferro",
        "ferrodock_model",
        "ferrodock_model_reactiveui",
        "ferrodock_controls_recycling",
        "ferroui_base",
        "ferroui_reactiveui",
        "ferroui_system",
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
    let (dependencies, files) =
        ModelSet::read(&ferroui_build_scan::export::dependencies_from_env()).unwrap();
    for file in files {
        println!("cargo::rerun-if-changed={}", file.display());
    }
    let mut assemblies = vec![scan.model.clone()];
    assemblies.extend(
        dependencies
            .into_iter()
            .filter(|m| m.crate_name != scan.model.crate_name),
    );
    let models = ModelSet::new(assemblies);
    let root = scan
        .model
        .types
        .iter()
        .find(|t| t.name == "RootDock")
        .unwrap();
    let mut hierarchy = String::new();
    let properties = inherited(root, &scan, &models, &mut hierarchy);
    let mut out = format!("const HIERARCHY: &[&str] = &[{hierarchy}];\n");
    // This probe deliberately selects only scalar properties, not a complete JSON schema.
    // Selection is in original source serializer order (the supplied C# model wire fixture).
    let names = [
        "PinnedDockDisplayMode",
        "FloatingWindowHostMode",
        "Id",
        "Title",
        "Proportion",
        "Dock",
        "DockingState",
        "CanClose",
        "CanPin",
        "PinnedDockDisplayModeOverride",
        "CanFloat",
        "CanDrag",
        "CanDrop",
        "CanDockAsDocument",
    ];
    let mut selected = Vec::new();
    for name in names {
        let matches: Vec<_> = properties.iter().filter(|p| p.name == name).collect();
        assert_eq!(
            matches.len(),
            1,
            "selected scalar {name} must have one declared owner"
        );
        selected.push(matches[0].clone());
    }
    let owner = "::std::rc::Rc<::ferrodock_model_reactiveui::controls::RootDock>";
    let projected = project_property_bindings(&selected, &metadata_instance_type(root).unwrap(), &RustType::resolved(owner), "|model: &::std::rc::Rc<::ferrodock_model_reactiveui::controls::RootDock>| ::ferrodock_model_reactiveui_ferro::reactive_metadata::RootDockReference::new(model.clone())").unwrap();
    writeln!(out, "fn properties() -> Vec<::ferrodock_serializer_system_text_json::JsonPropertyInfo<{owner}>> {{vec![").unwrap();
    for p in projected {
        assert!(p.converter.is_none() && !p.object_payload);
        let mut get = p.getter.unwrap();
        let mut set = p.setter.unwrap();
        if !p.getter_fallible {
            get = format!("{{let original={get};move|v: &{owner}|Ok(original(v))}}");
        }
        if !p.setter_fallible {
            set = format!(
                "{{let original={set};move|v: &{owner},item: {}|{{original(v,item);Ok(())}}}}",
                p.value_type
            );
        }
        writeln!(out, "::ferrodock_serializer_system_text_json::JsonPropertyInfo::try_new::<{}>({:?},{get},{set}),", p.value_type, p.name).unwrap();
    }
    out.push_str("]}\n");
    let target = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(target.join("native_json_properties.rs"), out).unwrap();
    super::native_json_models::emit(&here, &target, &scan, &models);
}
