// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Test compiler host: actual SDK class schemas, with explicit foreign type bindings.
use super::native_adapter::{
    bind_semantic_declarations, import_semantic_dependencies, read_source_declarations,
    RustSemanticDeclaration, RustSemanticTypeOptions,
};
use ferrodock_serializer_system_text_json_generators::*;
use ferroui_build_scan::{model::RustType, model::TypeModel, model_set::ModelSet, scanner::Scan};

pub(super) const NAMES: [&str; 7] = [
    "ReactiveBase",
    "DockableBase",
    "DockBase",
    "RootDock",
    "DocumentDock",
    "Document",
    "DockWindow",
];

pub(super) struct NativeModelCompilation {
    pub compilation: Compilation,
    pub source_types: Vec<Option<TypeId>>,
    pub native_types: Vec<TypeId>,
}

// Resolved symbol identities are supplied independently of the discarded class schemas.
pub(super) fn source_ids(reference: &Compilation) -> [TypeId; 7] {
    NAMES.map(|name| {
        let mut found = reference.types.iter().enumerate().filter(|(_, t)| {
            t.metadata_name == name && t.namespace.starts_with("Dock.Model.ReactiveUI.")
        });
        let id = found.next().unwrap().0;
        assert!(found.next().is_none(), "ambiguous class identity {name}");
        id
    })
}

fn named(c: &Compilation, excluded: &[TypeId], namespace: &str, name: &str) -> TypeId {
    let mut found = c
        .types
        .iter()
        .enumerate()
        .filter(|(id, _)| !excluded.contains(id))
        .filter(|(_, t)| t.namespace == namespace && t.metadata_name == name);
    let id = found
        .next()
        .unwrap_or_else(|| panic!("missing foreign type {namespace}.{name}"))
        .0;
    assert!(
        found.next().is_none(),
        "ambiguous foreign type {namespace}.{name}"
    );
    id
}

fn constructed(
    c: &Compilation,
    excluded: &[TypeId],
    namespace: &str,
    name: &str,
    argument: TypeId,
) -> TypeId {
    let mut found = c
        .types
        .iter()
        .enumerate()
        .filter(|(id, _)| !excluded.contains(id))
        .filter(|(_, t)| {
            t.namespace == namespace && t.metadata_name == name && t.type_arguments == [argument]
        });
    let id = found
        .next()
        .unwrap_or_else(|| panic!("missing foreign construction {namespace}.{name}"))
        .0;
    assert!(found.next().is_none(), "ambiguous foreign construction");
    id
}

// Logical class abstraction and source interfaces are explicit compiler configuration.
// Rust carrier handles can expose extra interfaces or omit optional model contracts.
fn class_contract(name: &str) -> (&'static str, bool, &'static [(&'static str, &'static str)]) {
    match name {
        "ReactiveBase" => ("Core", true, &[]),
        "DockableBase" => (
            "Core",
            true,
            &[
                ("Core", "IDockable"),
                ("Core", "IDockSelectorInfo"),
                ("Core", "IDockableDockingRestrictions"),
            ],
        ),
        "DockBase" => ("Core", true, &[("Core", "IDock")]),
        "RootDock" => ("Controls", false, &[("Controls", "IRootDock")]),
        "DocumentDock" => (
            "Controls",
            false,
            &[
                ("Controls", "IDocumentDock"),
                ("Controls", "IDocumentDockFactory"),
            ],
        ),
        "Document" => (
            "Controls",
            false,
            &[
                ("Controls", "IMdiDocument"),
                ("Core", "IDockingWindowState"),
            ],
        ),
        "DockWindow" => ("Core", false, &[("Core", "IDockWindow")]),
        _ => panic!("unconfigured native class {name}"),
    }
}

fn foreign_bindings(
    c: &Compilation,
    excluded: &[TypeId],
    scan: &Scan,
    models: &ModelSet,
) -> Vec<(String, TypeId)> {
    let mut bindings = Vec::new();
    let mut add = |text: &str, id| {
        let ty = scan.normalise(&scan.model.crate_name, text).unwrap();
        assert!(ty.is_resolved(), "{ty:?}");
        bindings.push((models.canonical(&ty.text), id));
    };
    let named = |ns, name| named(c, excluded, ns, name);
    for (rust, name) in [
        ("String", "String"),
        ("Option<String>", "String"),
        ("bool", "Boolean"),
        ("i32", "Int32"),
        ("f64", "Double"),
        ("Option<::ferroui_base::BoxedValue>", "Object"),
    ] {
        add(rust, named("System", name));
    }
    add(
        "Option<bool>",
        constructed(
            c,
            excluded,
            "System",
            "Nullable`1",
            named("System", "Boolean"),
        ),
    );
    for name in [
        "DockFloatingWindowHostMode",
        "DockMode",
        "DockOperationMask",
        "DockRect",
        "DockWindowOwnerMode",
        "DockWindowState",
        "DockingWindowState",
        "DocumentCloseButtonShowMode",
        "DocumentLayoutMode",
        "DocumentTabLayout",
        "MdiWindowState",
        "PinnedDockDisplayMode",
    ] {
        add(
            &format!("::ferrodock_model::core::{name}"),
            named("Dock.Model.Core", name),
        );
    }
    for name in ["PinnedDockDisplayMode", "DockRect"] {
        add(
            &format!("Option<::ferrodock_model::core::{name}>"),
            constructed(
                c,
                excluded,
                "System",
                "Nullable`1",
                named("Dock.Model.Core", name),
            ),
        );
    }
    for (rust, wire, dyn_) in [
        ("DockCapabilityOverrides", "DockCapabilityOverrides", ""),
        ("DockCapabilityPolicyImpl", "DockCapabilityPolicy", "dyn "),
    ] {
        add(
            &format!("Option<::std::rc::Rc<{dyn_}::ferrodock_model::core::{rust}>>"),
            named("Dock.Model.Core", wire),
        );
    }
    for (module, namespace, names) in [
        (
            "core",
            "Dock.Model.Core",
            &[
                "IDockable",
                "IDock",
                "IDockWindow",
                "IFactory",
                "IHostWindow",
            ][..],
        ),
        (
            "controls",
            "Dock.Model.Controls",
            &[
                "IRootDock",
                "IToolDock",
                "IDocumentDock",
                "IDocumentDockFactory",
                "IDocument",
                "IMdiDocument",
            ][..],
        ),
    ] {
        for name in names {
            let handle = format!("::std::rc::Rc<dyn ::ferrodock_model::{module}::{name}>");
            let id = named(namespace, name);
            add(&handle, id);
            add(&format!("Option<{handle}>"), id);
        }
    }
    for name in ["IDockable", "IDockWindow"] {
        let id = named("Dock.Model.Core", name);
        add(&format!("Option<::ferrodock_model::collections::ListHandle<Option<::std::rc::Rc<dyn ::ferrodock_model::core::{name}>>>>"), constructed(c, excluded, "System.Collections.Generic", "IList`1", id));
    }
    let command = c.required_symbols[6].unwrap();
    add("::std::rc::Rc<dyn ::ferroui_reactiveui::ICommand>", command);
    add(
        "Option<::std::rc::Rc<dyn ::ferroui_reactiveui::ICommand>>",
        command,
    );
    add(
        "Option<::ferrodock_model_ferro::document_factory_reference::DocumentFactoryReference>",
        constructed(
            c,
            excluded,
            "System",
            "Func`1",
            named("Dock.Model.Core", "IDockable"),
        ),
    );
    let reactive_object = scan
        .model
        .types
        .iter()
        .find(|t| t.name == "ReactiveObject")
        .unwrap();
    add(
        &reactive_object.rust_path.text,
        named("ReactiveUI", "ReactiveObject"),
    );
    bindings
}

fn ordered<'a>(scan: &'a Scan, models: &ModelSet) -> Vec<&'a TypeModel> {
    fn visit<'a>(
        t: &'a TypeModel,
        scan: &'a Scan,
        models: &ModelSet,
        active: &mut Vec<String>,
        out: &mut Vec<&'a TypeModel>,
    ) {
        if out.iter().any(|done| done.rust_path == t.rust_path) {
            return;
        }
        assert!(
            !active.contains(&t.rust_path.text),
            "cyclic native base graph"
        );
        active.push(t.rust_path.text.clone());
        if let Some(base) = &t.base {
            let base = scan
                .model
                .types
                .iter()
                .find(|b| models.canonical(&b.rust_path.text) == models.canonical(&base.text))
                .unwrap();
            if NAMES.contains(&base.name.as_str()) {
                visit(base, scan, models, active, out);
            }
        }
        active.pop();
        out.push(t);
    }
    let mut out = Vec::new();
    for t in scan
        .model
        .types
        .iter()
        .filter(|t| NAMES.contains(&t.name.as_str()))
    {
        visit(t, scan, models, &mut Vec::new(), &mut out);
    }
    assert_eq!(out.len(), NAMES.len());
    out
}

pub(super) fn compilation(
    reference: &Compilation,
    scan: &Scan,
    models: &ModelSet,
    source_ids: &[TypeId; 7],
) -> NativeModelCompilation {
    let metadata = ordered(scan, models);
    let replaced: Vec<_> = metadata
        .iter()
        .map(|t| source_ids[NAMES.iter().position(|name| *name == t.name).unwrap()])
        .collect();
    let bindings = foreign_bindings(reference, source_ids, scan, models);
    let mut roots: Vec<_> = bindings.iter().map(|(_, id)| *id).collect();
    let direct: Vec<Vec<_>> = metadata
        .iter()
        .map(|t| {
            class_contract(&t.name)
                .2
                .iter()
                .map(|(ns, name)| named(reference, source_ids, &format!("Dock.Model.{ns}"), name))
                .collect()
        })
        .collect();
    roots.extend(direct.iter().flatten().copied());
    let source = read_source_declarations(scan).unwrap();
    let mut source_types = Vec::new();
    let mut native_types = Vec::new();
    let mut compilation =
        import_semantic_dependencies(reference, &roots, &replaced, |working, ids| {
            source_types = ids.source_types.clone();
            native_types = ids.native_types.clone();
            let mut bindings: Vec<_> = bindings
                .iter()
                .map(|(path, id)| (path.clone(), ids.source_types[*id].unwrap()))
                .collect();
            bindings.extend(
                metadata
                    .iter()
                    .zip(&ids.native_types)
                    .map(|(t, id)| (models.canonical(&t.rust_path.text), *id)),
            );
            let resolve = |ty: &RustType| {
                let ty = scan.normalise(&scan.model.crate_name, &ty.text)?;
                if !ty.is_resolved() {
                    return None;
                }
                let path = models.canonical(&ty.text);
                bindings.iter().find(|(p, _)| *p == path).map(|(_, id)| *id)
            };
            let assembly = working.assemblies.len();
            working.assemblies.push(AssemblySymbol {
                name: scan.model.name.clone(),
                version: format!("{}.0", env!("CARGO_PKG_VERSION")),
                culture: None,
                public_key_token: Vec::new(),
                grants_internal_access: false,
            });
            let mut options: Vec<RustSemanticTypeOptions> = Vec::new();
            for (index, t) in metadata.iter().enumerate() {
                let base_type = t
                    .base
                    .as_ref()
                    .map(|base| resolve(base).expect("resolved actual SDK base"));
                let mut all_interfaces = if let Some(base) = base_type {
                    if let Some(native) = ids.native_types.iter().position(|id| *id == base) {
                        assert!(native < index, "native base must precede its derived class");
                        options[native].all_interfaces.clone()
                    } else {
                        working.types[base].all_interfaces.clone()
                    }
                } else {
                    Vec::new()
                };
                for old in &direct[index] {
                    let id = ids.source_types[*old].unwrap();
                    all_interfaces.push(id);
                    all_interfaces.extend(&working.types[id].all_interfaces);
                }
                // Original compiler precedence keeps the last occurrence of an interface.
                let mut ordered = Vec::new();
                for id in all_interfaces.into_iter().rev() {
                    if !ordered.contains(&id) {
                        ordered.push(id);
                    }
                }
                ordered.reverse();
                let (module, is_abstract, _) = class_contract(&t.name);
                options.push(RustSemanticTypeOptions {
                    metadata_name: t.name.clone(),
                    namespace: format!("Dock.Model.ReactiveUI.{module}"),
                    assembly: Some(assembly),
                    accessibility: Accessibility::Public,
                    is_abstract,
                    containing_type: None,
                    base_type,
                    all_interfaces: ordered,
                    property_overrides: Vec::new(),
                });
            }
            let declarations: Vec<_> = metadata
                .iter()
                .zip(&options)
                .map(|(metadata, options)| RustSemanticDeclaration {
                    metadata,
                    source: source.metadata_definition(metadata).unwrap(),
                    options,
                })
                .collect();
            let allocated =
                bind_semantic_declarations(working, &declarations, |ty, _| resolve(ty))?;
            assert_eq!(allocated, ids.native_types);
            Ok(())
        })
        .unwrap();
    compilation.activated = reference.activated;
    compilation.registrations = reference
        .registrations
        .iter()
        .map(|r| Registration {
            type_id: r.type_id.map(|id| source_types[id].unwrap()),
            valid_type_constant: r.valid_type_constant,
            display_name: r.display_name.clone(),
            location: r.location.clone(),
        })
        .collect();
    compilation.source_types = reference
        .source_types
        .iter()
        .map(|id| source_types[*id].unwrap())
        .collect();
    compilation.occupied_type_names = reference.occupied_type_names.clone();
    NativeModelCompilation {
        compilation,
        source_types,
        native_types,
    }
}
