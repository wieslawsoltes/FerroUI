// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Compiles native SDK constructors/accessors onto the original four-model wire projection.
use ferrodock_serializer_system_text_json_generators::*;
use ferrodock_serializer_system_text_json_generators_rust::{
    bind_declared_properties, bind_declared_type, metadata_instance_type, project_type_binding,
    RustAccessorErrorMapping, RustMetadataOptions, RustReceiverProjection,
};
use ferroui_build_scan::{model::TypeModel, model_set::ModelSet, scanner::Scan};
use std::path::Path;

fn original_accessor(
    scan: &Scan,
    models: &ModelSet,
    declaration: &TypeModel,
    accessor: &str,
) -> String {
    let mut interfaces = Vec::new();
    let mut current = Some(declaration);
    while let Some(type_) = current {
        interfaces.extend(type_.interfaces.iter());
        current = type_.base.as_ref().map(|base| {
            scan.model
                .types
                .iter()
                .find(|type_| {
                    models.canonical(&type_.rust_path.text) == models.canonical(&base.text)
                })
                .expect("declared native base")
        });
    }
    let functions: Vec<_> = scan
        .functions
        .iter()
        .filter(|f| {
            f.name == accessor
                && f.visibility == "pub"
                && f.receiver
                && f.parameters.is_empty()
                && models.canonical(&f.owner) == models.canonical(&declaration.rust_path.text)
                && f.return_type
                    .as_ref()
                    .and_then(|ty| scan.normalise(&f.module, ty))
                    .is_some_and(|ty| {
                        ty.is_resolved()
                            && interfaces
                                .iter()
                                .any(|i| models.canonical(&i.text) == models.canonical(&ty.text))
                    })
        })
        .collect();
    assert_eq!(
        functions.len(),
        1,
        "unique original-interface accessor for {}",
        declaration.name
    );
    format!(
        "{}::{}",
        declaration.public_path.as_ref().unwrap(),
        functions[0].name
    )
}

pub(super) fn emit(here: &Path, out: &Path, scan: &Scan, models: &ModelSet) {
    let (compilation, _, mut bindings) = super::actual_model_projection::input(here);
    // Declaration order and source serialization attributes belong to native metadata.
    for name in [
        "DockableBase",
        "DockBase",
        "RootDock",
        "DocumentDock",
        "Document",
        "DockWindow",
    ] {
        let source_type = compilation
            .types
            .iter()
            .find(|t| t.metadata_name == name)
            .unwrap();
        let native_type = scan.model.types.iter().find(|t| t.name == name).unwrap();
        let actual: Vec<_> = native_type
            .properties
            .iter()
            .map(|p| {
                let ignored = p
                    .attributes
                    .iter()
                    .chain(
                        native_type
                            .property_attributes
                            .iter()
                            .filter(|(name, _)| name == &p.name)
                            .flat_map(|(_, attrs)| attrs),
                    )
                    .any(|attr| {
                        matches!(
                            attr.name.as_str(),
                            "IgnoreDataMember"
                                | "IgnoreDataMemberAttribute"
                                | "System.Runtime.Serialization.IgnoreDataMemberAttribute"
                        )
                    });
                (p.name.as_str(), ignored)
            })
            .collect();
        let expected: Vec<_> = source_type
            .properties
            .iter()
            .map(|id| {
                let p = &compilation.properties[*id];
                (p.name.as_str(), p.ignore_data_member)
            })
            .collect();
        assert_eq!(
            actual, expected,
            "native source declaration order/ignore attributes for {name}"
        );
    }
    let source_ids = super::native_model_schemas::source_ids(&compilation);
    let native = super::native_model_schemas::compilation(&compilation, scan, models, &source_ids);
    for binding in &mut bindings.types {
        binding.symbol = native.source_types[binding.symbol].unwrap();
    }
    for family in &mut bindings.families {
        for derived in &mut family.derived {
            derived.symbol = native.source_types[derived.symbol].unwrap();
        }
    }
    let schema_manifest: Vec<_> = native
        .native_types
        .iter()
        .map(|id| {
            let t = &native.compilation.types[*id];
            (t.metadata_name.clone(), t.properties.len())
        })
        .collect();
    let compilation = native.compilation;
    let model = build(&compilation).unwrap();
    assert!(model.diagnostics.is_empty());
    let mut manifest = Vec::new();
    for original in &mut bindings.types {
        let name = &compilation.types[original.symbol].metadata_name;
        let declaration = scan.model.types.iter().find(|t| &t.name == name).unwrap();
        let handles: Vec<_> = scan
            .model
            .handles
            .iter()
            .filter(|h| {
                models.canonical(&h.handle.text) == models.canonical(&original.value_type)
                    && models.canonical(&h.type_.text)
                        == models.canonical(&declaration.rust_path.text)
            })
            .collect();
        assert_eq!(
            handles.len(),
            1,
            "source-declared original model handle for {name}"
        );
        let view = metadata_instance_type(declaration).unwrap();
        assert!(
            scan.model.casts.iter().any(|cast| {
                models.canonical(&cast.from.text) == models.canonical(&handles[0].handle.text)
                    && models.canonical(&cast.to.text) == models.canonical(&view.text)
            }),
            "actual original-model to view cast for {name}"
        );
        let public = declaration.public_path.as_ref().unwrap();
        // The caller supplies this existing SDK API explicitly and checks its source facts.
        let new: Vec<_> = scan
            .functions
            .iter()
            .filter(|f| {
                f.name == "new"
                    && f.visibility == "pub"
                    && !f.receiver
                    && f.parameters.len() == 1
                    && f.return_type.as_deref() == Some("Self")
                    && models.canonical(&f.owner) == models.canonical(&declaration.rust_path.text)
            })
            .collect();
        assert_eq!(new.len(), 1, "existing typed SDK model-view constructor");
        // Explicit existing SDK API; return type and visibility are checked above.
        let model_accessor = original_accessor(scan, models, declaration, "model");
        let mut options = RustMetadataOptions {
            emitting_crate: "ferrodock_markup_tests".into(),
            declaring_crate: scan.model.crate_name.clone(),
            identity: format!(
                "|view: &{}| ::std::rc::Rc::as_ptr(&{model_accessor}(view)) as *const () as usize",
                view.text
            ),
            object_types: Vec::new(),
            converters: Vec::new(),
            inherited_properties: None,
            accessor_error_mapper: Some(RustAccessorErrorMapping::Map(
                "::ferrodock_model::core::DockSerializationError::new".into(),
            )),
        };
        let declared = bind_declared_properties(declaration, &options.property_options()).unwrap();
        let all =
            super::native_json_properties::inherited(declaration, scan, models, &mut String::new());
        options.inherited_properties = Some(all[declared.len()..].to_vec());
        let bound = bind_declared_type(original.symbol, declaration, &options).unwrap();
        assert!(
            bound.constructor.is_some(),
            "actual metadata constructor for {name}"
        );
        let projection = RustReceiverProjection {
            value_type: handles[0].handle.clone(),
            to_receiver: format!("|model: &{}| {public}::{}(model.clone())", handles[0].handle.text, new[0].name),
            from_constructor: Some(format!("|view: {}| {model_accessor}(&view).boxed_reference().downcast_ref::<{}>().cloned().expect(\"declared native constructor's original model type\")", view.text, handles[0].handle.text)),
        };
        let mut projected = project_type_binding(&bound, &projection).unwrap();
        let mut selected = Vec::new();
        let mut native_count = 0;
        for expected in &original.properties {
            let matches: Vec<_> = projected
                .properties
                .iter()
                .filter(|p| p.name == expected.name)
                .collect();
            if let [native] = matches.as_slice() {
                let expected_type = scan
                    .normalise(&scan.model.crate_name, &expected.value_type)
                    .unwrap();
                let native_type = scan
                    .normalise(&scan.model.crate_name, &native.value_type)
                    .unwrap();
                assert!(expected_type.is_resolved() && native_type.is_resolved());
                assert_eq!(
                    models.canonical(&native_type.text),
                    models.canonical(&expected_type.text),
                    "native value type for {name}.{}",
                    expected.name
                );
                selected.push((*native).clone());
                native_count += 1;
            } else {
                panic!(
                    "missing or ambiguous native accessor {name}.{}",
                    expected.name
                );
            }
        }
        projected.properties = selected;
        manifest.push((name.clone(), native_count));
        *original = projected;
    }
    let source = emit_rust(&compilation, &model, &bindings).unwrap().unwrap();
    std::fs::write(out.join("native_json_models.rs"), source).unwrap();
    let mut text = String::from("const NATIVE_MODEL_BINDINGS: &[(&str, usize)] = &[");
    for (name, count) in manifest {
        text.push_str(&format!("({name:?},{count}),"));
    }
    text.push_str("];\n");
    text.push_str("const NATIVE_MODEL_SCHEMAS: &[(&str, usize)] = &[");
    for (name, count) in schema_manifest {
        text.push_str(&format!("({name:?},{count}),"));
    }
    text.push_str("];\n");
    std::fs::write(out.join("native_json_model_bindings.rs"), text).unwrap();
}
