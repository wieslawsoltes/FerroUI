// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Shared original four-model serializer projection and explicit compiler fixture facts.
use ferrodock_serializer_system_text_json_generators::*;
use std::{fs, path::Path};
#[path = "json_compiler_input.rs"]
pub(crate) mod fixture_input;
fn prop(owner: &str, name: &str, value: &str, contract: &str, method: &str) -> RustPropertyBinding {
    let setter_method = if method == "pinned_bounds" {
        "pinned_bounds_rect"
    } else {
        method
    };
    RustPropertyBinding {
        getter_fallible: false,
        setter_fallible: false,
        name: name.into(),
        value_type: value.into(),
        getter: Some(format!("|v:&{owner}|{contract}::{method}(v.as_ref())")),
        setter: Some(format!(
            "|v:&{owner},value: {value}|{contract}::set_{setter_method}(v.as_ref(),value)"
        )),
        converter: None,
        object_payload: false,
    }
}
fn dockable(owner: &str) -> Vec<RustPropertyBinding> {
    let i = "::ferrodock_model::core::IDockable";
    let mut p = vec![
        prop(owner, "Id", "String", i, "id"),
        prop(owner, "Title", "String", i, "title"),
        prop(
            owner,
            "Owner",
            "Option<::std::rc::Rc<dyn ::ferrodock_model::core::IDockable>>",
            i,
            "owner",
        ),
    ];
    p.push(prop(owner, "Proportion", "f64", i, "proportion"));
    p.push(prop(
        owner,
        "Dock",
        "::ferrodock_model::core::DockMode",
        i,
        "dock",
    ));
    p.push(prop(
        owner,
        "DockingState",
        "::ferrodock_model::core::DockingWindowState",
        i,
        "docking_state",
    ));
    for (name, method) in [
        ("CollapsedProportion", "collapsed_proportion"),
        ("MinWidth", "min_width"),
        ("MaxWidth", "max_width"),
        ("MinHeight", "min_height"),
        ("MaxHeight", "max_height"),
    ] {
        p.push(prop(owner, name, "f64", i, method));
    }
    p.extend([
        prop(owner, "CanClose", "bool", i, "can_close"),
        prop(owner, "CanPin", "bool", i, "can_pin"),
        prop(
            owner,
            "PinnedDockDisplayModeOverride",
            "Option<::ferrodock_model::core::PinnedDockDisplayMode>",
            i,
            "pinned_dock_display_mode_override",
        ),
        prop(
            owner,
            "PinnedBounds",
            "Option<::ferrodock_model::core::DockRect>",
            i,
            "pinned_bounds",
        ),
        prop(owner, "CanFloat", "bool", i, "can_float"),
        prop(owner, "CanDrag", "bool", i, "can_drag"),
        prop(owner, "CanDrop", "bool", i, "can_drop"),
        prop(
            owner,
            "CanDockAsDocument",
            "bool",
            i,
            "can_dock_as_document",
        ),
        prop(
            owner,
            "DockCapabilityOverrides",
            "Option<::std::rc::Rc<::ferrodock_model::core::DockCapabilityOverrides>>",
            i,
            "dock_capability_overrides",
        ),
    ]);
    p
}
fn dock(owner: &str) -> Vec<RustPropertyBinding> {
    let i = "::ferrodock_model::core::IDock";
    let items="Option<::ferrodock_model::collections::ListHandle<Option<::std::rc::Rc<dyn ::ferrodock_model::core::IDockable>>>>";
    let value = "Option<::std::rc::Rc<dyn ::ferrodock_model::core::IDockable>>";
    let mut p = vec![
        prop(owner, "VisibleDockables", items, i, "visible_dockables"),
        prop(owner, "ActiveDockable", value, i, "active_dockable"),
        prop(owner, "DefaultDockable", value, i, "default_dockable"),
        prop(
            owner,
            "DockCapabilityPolicy",
            "Option<::std::rc::Rc<dyn ::ferrodock_model::core::DockCapabilityPolicyImpl>>",
            i,
            "dock_capability_policy",
        ),
    ];
    p.extend(dockable(owner));
    p
}
pub(crate) fn identity(owner: &str) -> String {
    format!("|v:&{owner}|::std::rc::Rc::as_ptr(v) as *const () as usize")
}
pub(crate) fn input(here: &Path) -> (Compilation, GenerationModel, RustBindings) {
    let data = here.join("../data/json-generator-semantics.json");
    println!("cargo::rerun-if-changed={}", data.display());
    println!("cargo::rerun-if-changed=../support/json_compiler_input.rs");
    let rows: serde_json::Value = serde_json::from_str(&fs::read_to_string(data).unwrap()).unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "actual-models")
        .unwrap();
    let compilation = fixture_input::compilation(&row["input"]);
    let model = build(&compilation).unwrap();
    assert!(model.diagnostics.is_empty());
    assert_eq!(model.serializable_types.len(), 4);
    let mut types = Vec::new();
    for t in &model.serializable_types {
        let short = compilation.types[t.type_id].metadata_name.as_str();
        let module = if short == "DockWindow" {
            "core"
        } else {
            "controls"
        };
        let concrete = format!("::ferrodock_model_reactiveui::{module}::{short}");
        let owner = format!("::std::rc::Rc<{concrete}>");
        let mut properties = match short {
            "RootDock" => {
                let i = "::ferrodock_model::controls::IRootDock";
                let mut p=vec![prop(&owner,"Window","Option<::std::rc::Rc<dyn ::ferrodock_model::core::IDockWindow>>",i,"window"),prop(&owner,"Windows","Option<::ferrodock_model::collections::ListHandle<Option<::std::rc::Rc<dyn ::ferrodock_model::core::IDockWindow>>>>",i,"windows")];
                p.insert(
                    0,
                    prop(
                        &owner,
                        "PinnedDockDisplayMode",
                        "::ferrodock_model::core::PinnedDockDisplayMode",
                        i,
                        "pinned_dock_display_mode",
                    ),
                );
                p.push(prop(
                    &owner,
                    "FloatingWindowHostMode",
                    "::ferrodock_model::core::DockFloatingWindowHostMode",
                    i,
                    "floating_window_host_mode",
                ));
                p.push(prop(
                    &owner,
                    "RootDockCapabilityPolicy",
                    "Option<::std::rc::Rc<dyn ::ferrodock_model::core::DockCapabilityPolicyImpl>>",
                    i,
                    "root_dock_capability_policy",
                ));
                p.extend(dock(&owner));
                p
            }
            "DocumentDock" => dock(&owner),
            "Document" => dockable(&owner),
            "DockWindow" => {
                let i = "::ferrodock_model::core::IDockWindow";
                let mut p = vec![prop(&owner, "Id", "String", i, "id")];
                for (name, method) in [
                    ("X", "x"),
                    ("Y", "y"),
                    ("Width", "width"),
                    ("Height", "height"),
                ] {
                    p.push(prop(&owner, name, "f64", i, method));
                }
                p.extend([
                    prop(
                        &owner,
                        "WindowState",
                        "::ferrodock_model::core::DockWindowState",
                        i,
                        "window_state",
                    ),
                    prop(&owner, "Topmost", "bool", i, "topmost"),
                    prop(&owner, "Title", "String", i, "title"),
                    prop(
                        &owner,
                        "OwnerMode",
                        "::ferrodock_model::core::DockWindowOwnerMode",
                        i,
                        "owner_mode",
                    ),
                    prop(
                        &owner,
                        "ParentWindow",
                        "Option<::std::rc::Rc<dyn ::ferrodock_model::core::IDockWindow>>",
                        i,
                        "parent_window",
                    ),
                    prop(&owner, "IsModal", "bool", i, "is_modal"),
                    prop(
                        &owner,
                        "ShowInTaskbar",
                        "Option<bool>",
                        i,
                        "show_in_taskbar",
                    ),
                    prop(
                        &owner,
                        "Layout",
                        "Option<::std::rc::Rc<dyn ::ferrodock_model::controls::IRootDock>>",
                        i,
                        "layout",
                    ),
                ]);
                p
            }
            _ => panic!("actual model binding"),
        };
        if short != "DockWindow" {
            let restrictions = "::ferrodock_model::core::IDockableDockingRestrictions";
            properties.extend([
                prop(
                    &owner,
                    "AllowedDockOperations",
                    "::ferrodock_model::core::DockOperationMask",
                    restrictions,
                    "allowed_dock_operations",
                ),
                prop(
                    &owner,
                    "AllowedDropOperations",
                    "::ferrodock_model::core::DockOperationMask",
                    restrictions,
                    "allowed_drop_operations",
                ),
            ]);
        }
        types.push(RustTypeBinding {
            symbol: t.type_id,
            value_type: owner.clone(),
            constructor: Some(format!("{concrete}::new")),
            identity: identity(&owner),
            properties,
            custom_codec: None,
            reference_casts: Vec::new(),
        });
    }
    let mut families = Vec::new();
    for p in &model.polymorphisms {
        if p.kind == DockBaseKind::Object {
            continue;
        }
        let trait_path = match p.kind {
            DockBaseKind::Dockable => "::ferrodock_model::core::IDockable",
            DockBaseKind::Dock => "::ferrodock_model::core::IDock",
            DockBaseKind::RootDock => "::ferrodock_model::controls::IRootDock",
            DockBaseKind::DockWindow => "::ferrodock_model::core::IDockWindow",
            DockBaseKind::DocumentTemplate => "::ferrodock_model::controls::IDocumentTemplate",
            DockBaseKind::ToolTemplate => "::ferrodock_model::controls::IToolTemplate",
            _ => unreachable!(),
        };
        let owner = format!("::std::rc::Rc<dyn {trait_path}>");
        let methods = if p.kind == DockBaseKind::DockWindow {
            "::ferrodock_model::core::IDockWindow"
        } else {
            "::ferrodock_model::core::IDockable"
        };
        let runtime = (!p.derived_types.is_empty()).then(|| {
            format!("|v:&{owner}|{methods}::boxed_reference(v.clone()).as_ref().type_id()")
        });
        let derived=p.derived_types.iter().map(|t|{let d=&types.iter().find(|b|b.symbol==t.type_id).unwrap().value_type;RustDerivedBinding{symbol:t.type_id,downcast:format!("|v:&{owner}|{methods}::boxed_reference(v.clone()).downcast_ref::<{d}>().cloned()"),upcast:format!("|v: {d}|v as {owner}")}}).collect();
        let properties = match p.kind {
            DockBaseKind::Dockable => dockable(&owner),
            DockBaseKind::Dock => dock(&owner),
            DockBaseKind::RootDock | DockBaseKind::DockWindow => {
                let name = if p.kind == DockBaseKind::RootDock {
                    "RootDock"
                } else {
                    "DockWindow"
                };
                let schema = types
                    .iter()
                    .find(|t| compilation.types[t.symbol].metadata_name == name)
                    .unwrap();
                schema
                    .properties
                    .iter()
                    // Optional restrictions belong to backend classes, not the root interface.
                    .filter(|property| {
                        !matches!(
                            property.name.as_str(),
                            "AllowedDockOperations" | "AllowedDropOperations"
                        )
                    })
                    .cloned()
                    .map(|mut prop| {
                        prop.getter = prop.getter.map(|v| v.replace(&schema.value_type, &owner));
                        prop.setter = prop.setter.map(|v| v.replace(&schema.value_type, &owner));
                        prop
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        families.push(RustFamilyBinding {
            kind: p.kind,
            value_type: owner.clone(),
            runtime_type: runtime,
            identity: (!p.derived_types.is_empty()).then(|| identity(&owner)),
            properties,
            derived,
        });
    }
    let j = "::ferrodock_serializer_system_text_json::JsonTypeInfo";
    let d = "Option<::std::rc::Rc<dyn ::ferrodock_model::core::IDockable>>";
    let w = "Option<::std::rc::Rc<dyn ::ferrodock_model::core::IDockWindow>>";
    let mut extra = vec![
        format!("{j}::list::<{d}>()"),
        format!("{j}::list::<{w}>()"),
        format!("{j}::optional::<::ferrodock_model::collections::ListHandle<{d}>>()"),
        format!("{j}::optional::<::ferrodock_model::collections::ListHandle<{w}>>()"),
        format!("{j}::optional::<String>()"),
    ];
    for i in [
        "::ferrodock_model::core::IDockable",
        "::ferrodock_model::core::IDock",
        "::ferrodock_model::controls::IRootDock",
        "::ferrodock_model::core::IDockWindow",
    ] {
        extra.push(format!("{j}::optional::<::std::rc::Rc<dyn {i}>>()"));
    }
    let bindings = RustBindings {
        runtime_crate: "::ferrodock_serializer_system_text_json".into(),
        model_crate: "::ferrodock_model".into(),
        types,
        families,
        additional_codecs: extra,
        additional_metadata_collections: vec![
            "::ferrodock_serializer_system_text_json::core_model_type_metadata()".into(),
        ],
    };
    (compilation, model, bindings)
}
