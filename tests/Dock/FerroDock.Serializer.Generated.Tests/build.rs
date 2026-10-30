//! Compiles emitted descriptors over the original engine's actual model accessors.
use actual_model_projection::{fixture_input, identity};
use ferrodock_serializer_system_text_json_generators::*;
use std::{fs, path::PathBuf};
#[path = "../support/json_actual_model_projection.rs"]
mod actual_model_projection;
mod payload_bindings;
mod receiver_bindings;
fn main() {
    let here = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let (compilation, model, bindings) = actual_model_projection::input(&here);
    let text = emit_rust(&compilation, &model, &bindings).unwrap().unwrap();
    assert!(emit_rust(
        &compilation,
        &model,
        &RustBindings {
            types: Vec::new(),
            ..bindings.clone()
        }
    )
    .is_err());
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("generated_json.rs");
    fs::write(output, text).unwrap();
    let rows: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(here.join("../data/json-generator-semantics.json")).unwrap(),
    )
    .unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "generated-payloads")
        .unwrap();
    let (c, scan) =
        payload_bindings::compilation_from_rust(fixture_input::compilation(&row["input"]));
    let m = build(&c).unwrap();
    assert!(m.diagnostics.is_empty());
    let b = payload_bindings::bindings(&c, &m, &scan);
    receiver_bindings::emit(
        &c,
        &m,
        &b,
        &here,
        &PathBuf::from(std::env::var_os("OUT_DIR").unwrap()),
    );
    let source = emit_rust(&c, &m, &b).unwrap().unwrap();
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("payloads.rs"),
        source,
    )
    .unwrap();
    // Independent consumer of the remaining plain readonly fallible signature.
    // The supplied descriptor omits this setter; the original model is unchanged.
    let mut readonly = b.clone();
    let binding = readonly
        .types
        .iter_mut()
        .find(|b| c.types[b.symbol].metadata_name == "GeneratedTypeNamed")
        .unwrap();
    let property = binding
        .properties
        .iter_mut()
        .find(|p| p.name == "Name")
        .unwrap();
    assert!(property.getter_fallible);
    property.setter = None;
    property.setter_fallible = false;
    let source = emit_rust(&c, &m, &readonly).unwrap().unwrap();
    assert!(source.contains("::try_readonly::<"));
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("readonly_probe.rs"),
        source,
    )
    .unwrap();
    let mut stateful = b.clone();
    let name_property = stateful
        .types
        .iter_mut()
        .find(|b| c.types[b.symbol].metadata_name == "GeneratedTypeNamed")
        .unwrap()
        .properties
        .iter_mut()
        .find(|p| p.name == "Name")
        .unwrap();
    assert!(name_property.getter_fallible && !name_property.setter_fallible);
    name_property.value_type = "::core::option::Option<::std::string::String>".into();
    let binding = stateful
        .types
        .iter_mut()
        .find(|b| c.types[b.symbol].metadata_name == "GeneratedPayload")
        .unwrap();
    let owner = binding.value_type.clone();
    let property = binding
        .properties
        .iter_mut()
        .find(|p| p.name == "Self")
        .unwrap();
    assert!(!property.getter_fallible && property.setter_fallible);
    property.getter=Some(format!("{{let sequence=::std::cell::Cell::new(0_usize);move|v:&{owner}|{{sequence.set(sequence.get()+1);if sequence.get()==1 {{None}}else{{Some(v.clone())}}}}}}"));
    let source = emit_rust(&c, &m, &stateful).unwrap().unwrap();
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("stateful_probe.rs"),
        source,
    )
    .unwrap();
}
