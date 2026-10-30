//! Test compiler host: native SDK contract schemas with explicit residual type bindings.
use super::native_adapter::{
    bind_semantic_declarations, import_semantic_dependencies, read_source_declarations,
    RustSemanticDeclaration, RustSemanticTypeOptions,
};
use super::*;
use ferroui_build_scan::{
    model::RustType,
    scanner::{scan_crate, Scan, ScanOptions, Severity},
};

mod native_enums;
#[path = "published_command.rs"]
mod published_command;

pub(super) fn scan(root: &std::path::Path) -> Scan {
    let mut options = ScanOptions::new("ferrodock_model_ferro", root.join("lib.rs"));
    options.extern_crates = [
        "ferrodock_model",
        "ferrodock_runtime",
        "ferrodock_controls_recycling_model_ferro",
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
    scan
}

fn named(c: &Compilation, namespace: &str, name: &str) -> usize {
    if let Some(index) = METADATA_NAMES
        .iter()
        .position(|wire| wire.rsplit_once('.') == Some((namespace, name)))
    {
        return c.required_symbols[index].unwrap();
    }
    c.types
        .iter()
        .enumerate()
        .filter(|(id, _)| !c.required_symbols[..7].contains(&Some(*id)))
        .find(|(_, t)| t.namespace == namespace && t.metadata_name == name)
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("Missing residual dependency {namespace}.{name}"))
}
fn constructed(c: &Compilation, namespace: &str, name: &str, argument: usize) -> usize {
    c.types
        .iter()
        .enumerate()
        .filter(|(id, _)| !c.required_symbols[..7].contains(&Some(*id)))
        .find(|(_, t)| {
            t.namespace == namespace && t.metadata_name == name && t.type_arguments == [argument]
        })
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("Missing residual construction {namespace}.{name}<{argument}>"))
}

// These are explicit compiler mappings of the existing native nullable/value/handle
// contracts to their logical wire types. No property rows from the C# schema are read.
fn residual_bindings(c: &Compilation, scan: &Scan, enum_ids: &[usize; 5]) -> Vec<(String, usize)> {
    let mut bindings = Vec::new();
    let mut add = |text: &str, id| {
        let ty = scan.normalise(&scan.model.crate_name, text).unwrap();
        assert!(ty.is_resolved(), "{ty:?}");
        bindings.push((ty.text, id));
    };
    for (rust, name) in [
        ("String", "String"),
        ("Option<String>", "String"),
        ("bool", "Boolean"),
        ("i32", "Int32"),
        ("f64", "Double"),
        ("Option<::ferroui_base::BoxedValue>", "Object"),
    ] {
        add(rust, named(c, "System", name));
    }
    let boolean = named(c, "System", "Boolean");
    add(
        "Option<bool>",
        constructed(c, "System", "Nullable`1", boolean),
    );
    for (name, id) in native_enums::NAMES.into_iter().zip(enum_ids) {
        add(&format!("::ferrodock_model::core::{name}"), *id);
    }
    for name in ["DockingWindowState", "DockRect"] {
        add(
            &format!("::ferrodock_model::core::{name}"),
            named(c, "Dock.Model.Core", name),
        );
    }
    for (name, id) in [
        ("PinnedDockDisplayMode", enum_ids[1]),
        ("DockRect", named(c, "Dock.Model.Core", "DockRect")),
    ] {
        add(
            &format!("Option<::ferrodock_model::core::{name}>"),
            constructed(c, "System", "Nullable`1", id),
        );
    }
    for (rust, wire, trait_) in [
        ("DockCapabilityOverrides", "DockCapabilityOverrides", false),
        ("DockCapabilityPolicyImpl", "DockCapabilityPolicy", true),
    ] {
        let id = named(c, "Dock.Model.Core", wire);
        let dyn_ = if trait_ { "dyn " } else { "" };
        add(
            &format!("Option<::std::rc::Rc<{dyn_}::ferrodock_model::core::{rust}>>"),
            id,
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
                "IDocumentTemplate",
                "IToolTemplate",
                "IToolDock",
            ][..],
        ),
    ] {
        for name in names {
            let id = named(c, namespace, name);
            let handle = format!("::std::rc::Rc<dyn ::ferrodock_model::{module}::{name}>");
            add(&handle, id);
            add(&format!("Option<{handle}>"), id);
        }
    }
    add(
        "::std::rc::Rc<dyn ::ferroui_reactiveui::ICommand>",
        c.required_symbols[6].unwrap(),
    );
    let mut recycling = c
        .types
        .iter()
        .enumerate()
        .filter(|(id, _)| !c.required_symbols[..7].contains(&Some(*id)))
        .filter(|(_, t)| {
            t.metadata_name == "IControlRecyclingIdProvider" && t.kind == TypeKind::Interface
        });
    let recycling_id = recycling.next().unwrap().0;
    assert!(
        recycling.next().is_none(),
        "Ambiguous recycling contract binding"
    );
    add("::std::rc::Rc<dyn ::ferrodock_controls_recycling_model_ferro::IControlRecyclingIdProvider>",recycling_id);
    for (name, index) in [("IDockable", 0), ("IDockWindow", 3)] {
        let id = c.required_symbols[index].unwrap();
        let list = constructed(c, "System.Collections.Generic", "IList`1", id);
        add(&format!("Option<::ferrodock_model::collections::ListHandle<Option<::std::rc::Rc<dyn ::ferrodock_model::core::{name}>>>>"),list);
    }
    bindings
}

pub(super) fn enum_source_ids(reference: &Compilation) -> [usize; 5] {
    native_enums::NAMES.map(|name| named(reference, "Dock.Model.Core", name))
}

/// Imports foreign facts and constructs six Dock contracts, ICommand and five native enums.
/// Source identities still unify explicit external references; no old contract schema or
/// slot is retained. Foreign roots come from actual native property/handle type bindings.
pub(super) fn compilation(reference: &Compilation, scan: &Scan) -> Compilation {
    compilation_with_enum_ids(reference, scan, &enum_source_ids(reference))
}

/// The compiler supplies resolved source identities independently of discarded type headers.
pub(super) fn compilation_with_enum_ids(
    reference: &Compilation,
    scan: &Scan,
    enum_ids: &[usize; 5],
) -> Compilation {
    let mut source = read_source_declarations(scan).unwrap();
    let sdk_root = scan.files[0].path.canonicalize().unwrap();
    // The root of the workspace: the nearest directory of the source with its lock file.
    let workspace = sdk_root
        .ancestors()
        .find(|directory| directory.join("Cargo.lock").is_file())
        .expect("the workspace of the scanned source");
    let command = published_command::read(workspace);
    let enums = native_enums::read(workspace);
    source.types.extend(enums.iter().map(|e| e.source.clone()));
    source.types.push(command.source.clone());
    println!(
        "cargo::rerun-if-changed={}",
        workspace.join("Cargo.lock").display()
    );
    for file in &command.scan.files {
        println!("cargo::rerun-if-changed={}", file.path.display());
    }
    let bindings = residual_bindings(reference, scan, enum_ids);
    let enum_base = named(reference, "System", "Enum");
    let roots: Vec<_> = bindings
        .iter()
        .map(|(_, id)| *id)
        .chain([enum_base])
        .collect();
    let replaced: Vec<_> = reference.required_symbols[..7]
        .iter()
        .map(|id| id.unwrap())
        .chain(enum_ids.iter().copied())
        .collect();
    let mut metadata: Vec<_> = METADATA_NAMES[..6]
        .iter()
        .map(|wire| {
            let name = wire.rsplit_once('.').unwrap().1;
            scan.model
                .types
                .iter()
                .find(|m| m.name == name && m.namespace.starts_with("FerroDock.Model."))
                .unwrap()
                .clone()
        })
        .collect();
    metadata.push(command.metadata);
    import_semantic_dependencies(reference, &roots, &replaced, |working, ids| {
        let bindings: Vec<_> = bindings
            .iter()
            .map(|(path, id)| (path, ids.source_types[*id].unwrap()))
            .collect();
        let resolve = |ty: &RustType| {
            let ty = scan.normalise(&scan.model.crate_name, &ty.text)?;
            if !ty.is_resolved() {
                return None;
            }
            bindings
                .iter()
                .find(|(p, _)| p.as_str() == ty.text)
                .map(|(_, id)| *id)
        };
        let assembly = working.assemblies.len();
        working.assemblies.push(AssemblySymbol {
            name: scan.model.name.clone(),
            version: format!("{}.0", env!("CARGO_PKG_VERSION")),
            culture: None,
            public_key_token: Vec::new(),
            grants_internal_access: false,
        });
        let command_assembly = working.assemblies.len();
        working.assemblies.push(AssemblySymbol {
            name: command.scan.model.name.clone(),
            version: format!("{}.0", command.version),
            culture: None,
            public_key_token: Vec::new(),
            grants_internal_access: false,
        });
        let mut options: Vec<RustSemanticTypeOptions> = Vec::new();
        for (index, (metadata, wire)) in metadata.iter().zip(METADATA_NAMES).enumerate() {
            let (namespace, name) = wire.rsplit_once('.').unwrap();
            let mut interfaces = Vec::new();
            for direct in metadata.base.iter().chain(&metadata.interfaces) {
                let id = resolve(direct)
                    .unwrap_or_else(|| panic!("Missing native interface binding {direct:?}"));
                let inherited = if let Some(native) = ids.native_types.iter().position(|n| *n == id)
                {
                    assert!(
                        native < index,
                        "Native base interfaces must precede dependents"
                    );
                    &options[native].all_interfaces
                } else {
                    &working.types[id].all_interfaces
                };
                for id in std::iter::once(id).chain(inherited.iter().copied()) {
                    if !interfaces.contains(&id) {
                        interfaces.push(id);
                    }
                }
            }
            options.push(RustSemanticTypeOptions {
                metadata_name: name.into(),
                namespace: namespace.into(),
                assembly: Some(if index == 6 {
                    command_assembly
                } else {
                    assembly
                }),
                accessibility: Accessibility::Public,
                is_abstract: true,
                containing_type: None,
                base_type: None,
                all_interfaces: interfaces,
                property_overrides: Vec::new(),
            });
        }
        let enum_base = ids.source_types[enum_base].unwrap();
        for enum_ in enums {
            options.push(RustSemanticTypeOptions {
                metadata_name: enum_.metadata.name.clone(),
                namespace: "Dock.Model.Core".into(),
                assembly: Some(assembly),
                accessibility: Accessibility::Public,
                is_abstract: false,
                containing_type: None,
                base_type: Some(enum_base),
                all_interfaces: working.types[enum_base].all_interfaces.clone(),
                property_overrides: Vec::new(),
            });
            metadata.push(enum_.metadata);
        }
        let declarations: Vec<_> = metadata
            .iter()
            .zip(&options)
            .map(|(m, o)| RustSemanticDeclaration {
                metadata: m,
                source: source.metadata_definition(m).unwrap(),
                options: o,
            })
            .collect();
        let allocated = bind_semantic_declarations(working, &declarations, |ty, _| resolve(ty))?;
        assert_eq!(allocated, ids.native_types);
        Ok(())
    })
    .unwrap()
}
