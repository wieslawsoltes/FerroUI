//! Test compiler bindings for the original source-generator application payload fixtures.
use super::*;
use ferrodock_serializer_system_text_json_generators_rust as native_adapter;
#[path = "native_contracts.rs"]
mod native_contracts;
pub(crate) fn compilation_from_rust(
    reference: Compilation,
) -> (Compilation, ferroui_build_scan::scanner::Scan) {
    use ferrodock_serializer_system_text_json_generators_rust::{
        apply_generation_attributes, bind_semantic_declarations, parse_generation_attributes,
        read_source_declarations, RustDeclarationKind, RustDeclarationVisibility,
        RustSemanticDeclaration, RustSemanticTypeOptions,
    };
    use ferroui_build_scan::scanner::{scan_crate, ScanOptions, Severity};
    // Import only the resolved dependency closure. No reference application type,
    // property, allocated slot, registration or discovery entry survives this boundary.
    let here = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let contracts = native_contracts::scan(&here.join("../../../src/Dock/FerroDock.Model.Ferro"));
    for file in &contracts.files {
        println!("cargo::rerun-if-changed={}", file.path.display());
    }
    let mut c = native_contracts::compilation(&reference, &contracts);
    let mut options = ScanOptions::new(
        "ferrodock_serializer_generated_tests",
        here.join("payload_metadata.rs"),
    );
    options.extern_crates = vec![
        "ferrodock_model".into(),
        "ferrodock_serializer_system_text_json".into(),
    ];
    let scan = scan_crate(&options);
    for f in &scan.files {
        println!("cargo::rerun-if-changed={}", f.path.display());
    }
    let errors: Vec<_> = scan.diagnostics_of(Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let path = here.join("json_registration.rs");
    println!("cargo::rerun-if-changed={}", path.display());
    let a = parse_generation_attributes(
        &scan,
        &scan.model.crate_name,
        "json_registration.rs",
        &fs::read_to_string(path).unwrap(),
    )
    .unwrap();
    assert!(a.activated);
    assert_eq!(a.registrations.len(), 9);
    let declarations = read_source_declarations(&scan).unwrap();
    let mut metadata = Vec::new();
    // Discover ordinary metadata-declared classes in actual source traversal order.
    for source in &declarations.types {
        if source.parameters.is_empty() {
            if let Some(m) = scan.model.types.iter().find(|m| m.rust_path == source.path) {
                metadata.push(m.clone());
            }
        }
    }
    let discovered = metadata.len();
    // Closed constructions are requested by real registrations, never old fixture slots.
    for r in &a.registrations {
        let source = declarations.definition(&r.type_).unwrap();
        if source.parameters.is_empty() || metadata.iter().any(|m| m.rust_path == r.type_) {
            continue;
        }
        metadata.push(instantiate_registered(&scan, source, &r.type_));
    }
    assert_eq!(metadata.len(), 9);
    let native_assembly = c.assemblies.len();
    c.assemblies.push(AssemblySymbol {
        name: scan.model.name.clone(),
        version: format!("{}.0", std::env::var("CARGO_PKG_VERSION").unwrap()),
        culture: None,
        public_key_token: Vec::new(),
        grants_internal_access: true,
    });
    let builtin = |name| {
        c.types
            .iter()
            .position(|t| t.namespace == "System" && t.metadata_name == name)
            .unwrap()
    };
    let object = builtin("Object");
    let string = builtin("String");
    let int = builtin("Int32");
    let normalize = |text: &str| scan.normalise(&scan.model.crate_name, text).unwrap().text;
    let dependency_types = [
        (normalize("Option<String>"), string),
        (normalize("i32"), int),
        (normalize("Option<::ferrodock_model::BoxedValue>"), object),
    ];
    let payload = metadata
        .iter()
        .position(|m| m.rust_path.text.ends_with("::Payload"))
        .unwrap();
    let payload_handle = normalize("Option<::std::rc::Rc<crate::payload_models::Payload>>");
    let options: Vec<_> = metadata
        .iter()
        .map(|m| RustSemanticTypeOptions {
            // Explicit source wire identities for these fixtures, separate from Rust names.
            metadata_name: m.name.clone(),
            namespace: m.namespace.clone(),
            assembly: Some(native_assembly),
            accessibility: Accessibility::Public,
            is_abstract: false,
            containing_type: None,
            base_type: Some(object),
            all_interfaces: Vec::new(),
            property_overrides: Vec::new(),
        })
        .collect();
    let inputs: Vec<_> = metadata
        .iter()
        .zip(&options)
        .map(|(m, o)| {
            let source = declarations.definition(&m.rust_path).unwrap();
            assert_eq!(source.kind, RustDeclarationKind::Struct);
            assert_eq!(source.visibility, RustDeclarationVisibility::Public);
            RustSemanticDeclaration {
                metadata: m,
                source,
                options: o,
            }
        })
        .collect();
    let ids = bind_semantic_declarations(&mut c, &inputs, |ty, ids| {
        if ty.text == payload_handle {
            return Some(ids[payload]);
        }
        metadata
            .iter()
            .position(|m| m.rust_path == *ty)
            .map(|i| ids[i])
            .or_else(|| {
                dependency_types
                    .iter()
                    .find(|(p, _)| p == &ty.text)
                    .map(|(_, id)| *id)
            })
    })
    .unwrap();
    c.source_types = ids[..discovered].to_vec();
    c.occupied_type_names = declarations
        .types
        .iter()
        .map(|s| s.path.text.rsplit("::").next().unwrap().to_owned())
        .collect();
    apply_generation_attributes(&mut c, &a, |ty| {
        metadata
            .iter()
            .position(|m| m.rust_path == *ty)
            .map(|i| ids[i])
    })
    .unwrap();
    (c, scan)
}
fn instantiate_registered(
    scan: &ferroui_build_scan::scanner::Scan,
    source: &ferrodock_serializer_system_text_json_generators_rust::RustSourceDeclaration,
    target: &ferroui_build_scan::model::RustType,
) -> ferroui_build_scan::model::TypeModel {
    use ferrodock_serializer_system_text_json_generators_rust::instantiate_generic_metadata;
    let attempts: Vec<_> = scan
        .model
        .types
        .iter()
        .filter(|m| m.generic.is_some())
        .map(|m| {
            instantiate_generic_metadata(m, source, target, |text| scan.normalise(&m.module, text))
        })
        .collect();
    let mut matching = attempts.iter().filter_map(|r| r.as_ref().ok());
    let result = matching
        .next()
        .unwrap_or_else(|| panic!("No native template for {target:?}: {attempts:?}"));
    assert!(
        matching.next().is_none(),
        "Ambiguous native template for {target:?}"
    );
    result.clone()
}
pub(crate) fn bindings(
    c: &Compilation,
    m: &GenerationModel,
    scan: &ferroui_build_scan::scanner::Scan,
) -> RustBindings {
    use ferrodock_serializer_system_text_json_generators_rust::{
        bind_declared_type, RustMetadataConverter, RustMetadataOptions,
    };
    let mut types = Vec::new();
    for t in &m.serializable_types {
        let symbol = &c.types[t.type_id];
        let declaration = if symbol.arity == 0 {
            scan.model
                .types
                .iter()
                .find(|d| d.rust_path.text == symbol.expression)
                .unwrap()
                .clone()
        } else {
            use ferrodock_serializer_system_text_json_generators_rust::read_source_declarations;
            let definitions = read_source_declarations(scan).unwrap();
            let concrete = ferroui_build_scan::model::RustType::resolved(&symbol.expression);
            let source = definitions.definition(&concrete).unwrap();
            instantiate_registered(scan, source, &concrete)
        };
        let normal = |text: &str| {
            let ty = scan.normalise(&declaration.module, text).unwrap();
            assert!(ty.is_resolved(), "{ty:?}");
            ty.text
        };
        let owner = &declaration.this.as_ref().unwrap().text;
        let binding = bind_declared_type(
            t.type_id,
            &declaration,
            &RustMetadataOptions {
                emitting_crate: scan.model.crate_name.clone(),
                declaring_crate: scan.model.crate_name.clone(),
                identity: identity(owner),
                object_types: vec![normal("Option<::ferrodock_model::BoxedValue>")],
                converters: vec![
                    RustMetadataConverter {
                        converter_type: normal("crate::payload_models::StringConverter"),
                        factory: "crate::payload_models::string_info()".into(),
                    },
                    RustMetadataConverter {
                        converter_type: normal("crate::payload_models::ArrayConverter"),
                        factory: "crate::payload_models::array_info()".into(),
                    },
                ],
                inherited_properties: None,
                accessor_error_mapper: Some(if symbol.metadata_name == "GeneratedReadonlyHolder" {
                    // This fixture's actual native signature returns std::io::Error.
                    native_adapter::RustAccessorErrorMapping::Map(
                        "::ferrodock_model::core::DockSerializationError::new".into(),
                    )
                } else {
                    native_adapter::RustAccessorErrorMapping::Preserve
                }),
            },
        )
        .unwrap();
        types.push(binding);
    }
    let families = m
        .polymorphisms
        .iter()
        .filter(|p| p.kind != DockBaseKind::Object)
        .map(|p| {
            assert!(p.derived_types.is_empty());
            let trait_path = match p.kind {
                DockBaseKind::Dockable => "::ferrodock_model::core::IDockable",
                DockBaseKind::Dock => "::ferrodock_model::core::IDock",
                DockBaseKind::RootDock => "::ferrodock_model::controls::IRootDock",
                DockBaseKind::DockWindow => "::ferrodock_model::core::IDockWindow",
                DockBaseKind::DocumentTemplate => "::ferrodock_model::controls::IDocumentTemplate",
                DockBaseKind::ToolTemplate => "::ferrodock_model::controls::IToolTemplate",
                _ => unreachable!(),
            };
            RustFamilyBinding {
                kind: p.kind,
                value_type: format!("::std::rc::Rc<dyn {trait_path}>"),
                runtime_type: None,
                identity: None,
                properties: Vec::new(),
                derived: Vec::new(),
            }
        })
        .collect();
    RustBindings{runtime_crate:"::ferrodock_serializer_system_text_json".into(),model_crate:"::ferrodock_model".into(),types,families,additional_metadata_collections:Vec::new(),additional_codecs:vec!["::ferrodock_serializer_system_text_json::JsonTypeInfo::optional::<String>()".into(),"::ferrodock_serializer_system_text_json::JsonTypeInfo::optional::<::std::rc::Rc<crate::payload_models::Payload>>()".into()]}
}
