//! Test compiler host using the same native class-view protocol as the SDK.
use super::*;
use ferrodock_serializer_system_text_json_generators_rust::{
    bind_declared_type, project_type_binding, RustAccessorErrorMapping, RustMetadataOptions,
    RustReceiverProjection,
};
use ferroui_build_scan::{
    model::RustType,
    scanner::{scan_crate, ScanOptions, Severity},
};

pub(super) fn emit(
    c: &Compilation,
    m: &GenerationModel,
    bindings: &RustBindings,
    here: &std::path::Path,
    out: &std::path::Path,
) {
    let mut options = ScanOptions::new(
        "ferrodock_serializer_generated_tests",
        here.join("receiver_metadata.rs"),
    );
    options.extern_crates = vec![
        "ferrodock_model".into(),
        "ferrodock_serializer_system_text_json".into(),
    ];
    let scan = scan_crate(&options);
    let errors: Vec<_> = scan.diagnostics_of(Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    for file in &scan.files {
        println!("cargo::rerun-if-changed={}", file.path.display());
    }
    let declaration = scan
        .model
        .types
        .iter()
        .find(|t| t.name == "GeneratedPayload")
        .unwrap();
    let view = "::ferrodock_serializer_generated_tests::receiver_models::PayloadView";
    let original = bindings
        .types
        .iter()
        .find(|b| c.types[b.symbol].metadata_name == "GeneratedPayload")
        .unwrap();
    let native = bind_declared_type(
        original.symbol,
        declaration,
        &RustMetadataOptions {
            emitting_crate: scan.model.crate_name.clone(),
            declaring_crate: scan.model.crate_name.clone(),
            identity: format!("{view}::identity"),
            object_types: Vec::new(),
            converters: Vec::new(),
            inherited_properties: None,
            accessor_error_mapper: Some(RustAccessorErrorMapping::Preserve),
        },
    )
    .unwrap();
    let projection = RustReceiverProjection {
        value_type: RustType::resolved(&original.value_type),
        to_receiver: format!("{view}::from_model"),
        from_constructor: Some(format!("{view}::into_model")),
    };
    let projected = project_type_binding(&native, &projection).unwrap();
    let mut result = bindings.clone();
    *result
        .types
        .iter_mut()
        .find(|b| b.symbol == original.symbol)
        .unwrap() = projected;
    fs::write(
        out.join("receiver_probe.rs"),
        emit_rust(c, m, &result).unwrap().unwrap(),
    )
    .unwrap();
    let mut constructed = native.clone();
    constructed.constructor=Some(format!("{{let model=::ferrodock_serializer_generated_tests::payload_models::Payload::new();let count=::std::cell::Cell::new(0_usize);move||{{count.set(count.get()+1);model.name.replace(Some(count.get().to_string()));{view}::from_model(&model)}}}}"));
    *result
        .types
        .iter_mut()
        .find(|b| b.symbol == original.symbol)
        .unwrap() = project_type_binding(&constructed, &projection).unwrap();
    fs::write(
        out.join("receiver_constructor_probe.rs"),
        emit_rust(c, m, &result).unwrap().unwrap(),
    )
    .unwrap();
    let mut failing = native;
    failing.properties.iter_mut().find(|p|p.name=="Name").unwrap().getter=Some(format!("{{let failure=::ferrodock_model::core::DockSerializationError::new(::std::io::Error::other(\"original-accessor-failure\"));move|_:&{view}|Err::<Option<String>,_>(failure.clone())}}"));
    *result
        .types
        .iter_mut()
        .find(|b| b.symbol == original.symbol)
        .unwrap() = project_type_binding(&failing, &projection).unwrap();
    fs::write(
        out.join("receiver_error_probe.rs"),
        emit_rust(c, m, &result).unwrap().unwrap(),
    )
    .unwrap();
}
