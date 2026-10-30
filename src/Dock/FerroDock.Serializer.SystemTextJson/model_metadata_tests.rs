// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Explicit projection tests over actual original engine models, not the default model catalog.
use super::*;
use ferrodock_model::{collections::ListHandle, controls::*, core::*};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock},
    core::DockWindow,
    Factory,
};
use std::{cell::RefCell, rc::Rc};
type Items = ListHandle<Option<Rc<dyn IDockable>>>;
type Windows = ListHandle<Option<Rc<dyn IDockWindow>>>;
fn concrete<T: 'static>(value: &Rc<dyn IDockable>) -> Option<Rc<T>> {
    value
        .clone()
        .boxed_reference()
        .downcast_ref::<Rc<T>>()
        .cloned()
}
fn window(value: &Rc<dyn IDockWindow>) -> Option<Rc<DockWindow>> {
    value
        .clone()
        .boxed_reference()
        .downcast_ref::<Rc<DockWindow>>()
        .cloned()
}
fn dockable_cast<T: IDockable + 'static>(info: Rc<JsonTypeInfo>) -> Rc<JsonTypeInfo> {
    JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IDockable>>(info, |value| {
        value.clone() as Rc<dyn IDockable>
    })
}
fn dock_cast<T: IDock + 'static>(info: Rc<JsonTypeInfo>) -> Rc<JsonTypeInfo> {
    JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IDock>>(dockable_cast::<T>(info), |value| {
        value.clone() as Rc<dyn IDock>
    })
}
fn fields<T: IDockable + IDockableDockingRestrictions + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    vec![
        JsonPropertyInfo::new("Id", |v: &Rc<T>| v.id(), |v, x| v.set_id(x)),
        JsonPropertyInfo::new("Title", |v: &Rc<T>| v.title(), |v, x| v.set_title(x)),
        JsonPropertyInfo::new("Owner", |v: &Rc<T>| v.owner(), |v, x| v.set_owner(x)),
        JsonPropertyInfo::new(
            "Proportion",
            |v: &Rc<T>| v.proportion(),
            |v, x| v.set_proportion(x),
        ),
        JsonPropertyInfo::new("Dock", |v: &Rc<T>| v.dock(), |v, x| v.set_dock(x)),
        JsonPropertyInfo::new(
            "DockingState",
            |v: &Rc<T>| v.docking_state(),
            |v, x| v.set_docking_state(x),
        ),
        JsonPropertyInfo::new(
            "CollapsedProportion",
            |v: &Rc<T>| v.collapsed_proportion(),
            |v, x| v.set_collapsed_proportion(x),
        ),
        JsonPropertyInfo::new(
            "MinWidth",
            |v: &Rc<T>| v.min_width(),
            |v, x| v.set_min_width(x),
        ),
        JsonPropertyInfo::new(
            "MaxWidth",
            |v: &Rc<T>| v.max_width(),
            |v, x| v.set_max_width(x),
        ),
        JsonPropertyInfo::new(
            "MinHeight",
            |v: &Rc<T>| v.min_height(),
            |v, x| v.set_min_height(x),
        ),
        JsonPropertyInfo::new(
            "MaxHeight",
            |v: &Rc<T>| v.max_height(),
            |v, x| v.set_max_height(x),
        ),
        JsonPropertyInfo::new(
            "CanClose",
            |v: &Rc<T>| v.can_close(),
            |v, x| v.set_can_close(x),
        ),
        JsonPropertyInfo::new("CanPin", |v: &Rc<T>| v.can_pin(), |v, x| v.set_can_pin(x)),
        JsonPropertyInfo::new(
            "PinnedDockDisplayModeOverride",
            |v: &Rc<T>| v.pinned_dock_display_mode_override(),
            |v, x| v.set_pinned_dock_display_mode_override(x),
        ),
        JsonPropertyInfo::new(
            "PinnedBounds",
            |v: &Rc<T>| v.pinned_bounds(),
            |v, x| v.set_pinned_bounds_rect(x),
        ),
        JsonPropertyInfo::new(
            "CanFloat",
            |v: &Rc<T>| v.can_float(),
            |v, x| v.set_can_float(x),
        ),
        JsonPropertyInfo::new(
            "CanDrag",
            |v: &Rc<T>| v.can_drag(),
            |v, x| v.set_can_drag(x),
        ),
        JsonPropertyInfo::new(
            "CanDrop",
            |v: &Rc<T>| v.can_drop(),
            |v, x| v.set_can_drop(x),
        ),
        JsonPropertyInfo::new(
            "CanDockAsDocument",
            |v: &Rc<T>| v.can_dock_as_document(),
            |v, x| v.set_can_dock_as_document(x),
        ),
        JsonPropertyInfo::new(
            "DockCapabilityOverrides",
            |v: &Rc<T>| v.dock_capability_overrides(),
            |v, x| v.set_dock_capability_overrides(x),
        ),
        JsonPropertyInfo::new(
            "AllowedDockOperations",
            |v: &Rc<T>| v.allowed_dock_operations(),
            |v, x| v.set_allowed_dock_operations(x),
        ),
        JsonPropertyInfo::new(
            "AllowedDropOperations",
            |v: &Rc<T>| v.allowed_drop_operations(),
            |v, x| v.set_allowed_drop_operations(x),
        ),
    ]
}
fn dock_fields<T: IDock + IDockableDockingRestrictions + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>>
{
    let mut fields = vec![
        JsonPropertyInfo::new(
            "VisibleDockables",
            |v: &Rc<T>| v.visible_dockables(),
            |v, x| v.set_visible_dockables(x),
        ),
        JsonPropertyInfo::new(
            "ActiveDockable",
            |v: &Rc<T>| v.active_dockable(),
            |v, x| v.set_active_dockable(x),
        ),
        JsonPropertyInfo::new(
            "DefaultDockable",
            |v: &Rc<T>| v.default_dockable(),
            |v, x| v.set_default_dockable(x),
        ),
        JsonPropertyInfo::new(
            "DockCapabilityPolicy",
            |v: &Rc<T>| v.dock_capability_policy(),
            |v, x| v.set_dock_capability_policy(x),
        ),
    ];
    fields.extend(self::fields::<T>());
    fields
}
#[derive(Default)]
struct Holder {
    a: RefCell<Option<Items>>,
    b: RefCell<Option<Items>>,
    item: RefCell<Option<Rc<dyn IDockable>>>,
}
pub(super) fn resolver() -> Rc<GeneratedJsonTypeInfoResolver> {
    let mut root_fields = vec![
        JsonPropertyInfo::new(
            "PinnedDockDisplayMode",
            |v: &Rc<RootDock>| v.pinned_dock_display_mode(),
            |v, x| v.set_pinned_dock_display_mode(x),
        ),
        JsonPropertyInfo::new(
            "Window",
            |v: &Rc<RootDock>| v.window(),
            |v, x| v.set_window(x),
        ),
        JsonPropertyInfo::new(
            "Windows",
            |v: &Rc<RootDock>| v.windows(),
            |v, x| v.set_windows(x),
        ),
        JsonPropertyInfo::new(
            "FloatingWindowHostMode",
            |v: &Rc<RootDock>| v.floating_window_host_mode(),
            |v, x| v.set_floating_window_host_mode(x),
        ),
        JsonPropertyInfo::new(
            "RootDockCapabilityPolicy",
            |v: &Rc<RootDock>| v.root_dock_capability_policy(),
            |v, x| v.set_root_dock_capability_policy(x),
        ),
    ];
    root_fields.extend(dock_fields::<RootDock>());
    let root = JsonTypeInfo::object(
        RootDock::new,
        |v: &Rc<RootDock>| Rc::as_ptr(v) as usize,
        root_fields,
    );
    let root = JsonTypeInfo::with_reference_cast::<Rc<RootDock>, Rc<dyn IRootDock>>(
        dock_cast::<RootDock>(root),
        |v| v.clone() as Rc<dyn IRootDock>,
    );
    let docs = dock_cast::<DocumentDock>(JsonTypeInfo::object(
        DocumentDock::new,
        |v: &Rc<DocumentDock>| Rc::as_ptr(v) as usize,
        dock_fields::<DocumentDock>(),
    ));
    let document = dockable_cast::<Document>(JsonTypeInfo::object(
        Document::new,
        |v: &Rc<Document>| Rc::as_ptr(v) as usize,
        fields::<Document>(),
    ));
    let win = JsonTypeInfo::object(
        DockWindow::new,
        |v: &Rc<DockWindow>| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::new("Id", |v: &Rc<DockWindow>| v.id(), |v, x| v.set_id(x)),
            JsonPropertyInfo::new("X", |v: &Rc<DockWindow>| v.x(), |v, x| v.set_x(x)),
            JsonPropertyInfo::new("Y", |v: &Rc<DockWindow>| v.y(), |v, x| v.set_y(x)),
            JsonPropertyInfo::new(
                "Width",
                |v: &Rc<DockWindow>| v.width(),
                |v, x| v.set_width(x),
            ),
            JsonPropertyInfo::new(
                "Height",
                |v: &Rc<DockWindow>| v.height(),
                |v, x| v.set_height(x),
            ),
            JsonPropertyInfo::new(
                "WindowState",
                |v: &Rc<DockWindow>| v.window_state(),
                |v, x| v.set_window_state(x),
            ),
            JsonPropertyInfo::new(
                "Topmost",
                |v: &Rc<DockWindow>| v.topmost(),
                |v, x| v.set_topmost(x),
            ),
            JsonPropertyInfo::new(
                "Title",
                |v: &Rc<DockWindow>| v.title(),
                |v, x| v.set_title(x),
            ),
            JsonPropertyInfo::new(
                "OwnerMode",
                |v: &Rc<DockWindow>| v.owner_mode(),
                |v, x| v.set_owner_mode(x),
            ),
            JsonPropertyInfo::new(
                "ParentWindow",
                |v: &Rc<DockWindow>| v.parent_window(),
                |v, x| v.set_parent_window(x),
            ),
            JsonPropertyInfo::new(
                "IsModal",
                |v: &Rc<DockWindow>| v.is_modal(),
                |v, x| v.set_is_modal(x),
            ),
            JsonPropertyInfo::new(
                "ShowInTaskbar",
                |v: &Rc<DockWindow>| v.show_in_taskbar(),
                |v, x| v.set_show_in_taskbar(x),
            ),
            JsonPropertyInfo::new(
                "Layout",
                |v: &Rc<DockWindow>| v.layout(),
                |v, x| v.set_layout(x),
            ),
        ],
    );
    let win = JsonTypeInfo::with_reference_cast::<Rc<DockWindow>, Rc<dyn IDockWindow>>(win, |v| {
        v.clone() as Rc<dyn IDockWindow>
    });
    let base = JsonTypeInfo::polymorphic::<Rc<dyn IDockable>>(
        vec![
            JsonDerivedType::new::<Rc<RootDock>>(
                "Dock.Model.ReactiveUI.Controls.RootDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<DocumentDock>>(
                "Dock.Model.ReactiveUI.Controls.DocumentDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<Document>>(
                "Dock.Model.ReactiveUI.Controls.Document",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
        ],
        None,
        true,
    );
    let root_base = JsonTypeInfo::polymorphic::<Rc<dyn IRootDock>>(
        vec![JsonDerivedType::new::<Rc<RootDock>>(
            "Dock.Model.ReactiveUI.Controls.RootDock",
            |v: &Rc<dyn IRootDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
            |v| v as Rc<dyn IRootDock>,
        )],
        None,
        true,
    );
    let windows = JsonTypeInfo::polymorphic::<Rc<dyn IDockWindow>>(
        vec![JsonDerivedType::new::<Rc<DockWindow>>(
            "Dock.Model.ReactiveUI.Core.DockWindow",
            window,
            |v| v as Rc<dyn IDockWindow>,
        )],
        None,
        true,
    );
    let holder = JsonTypeInfo::object(
        || Rc::new(Holder::default()),
        |v: &Rc<Holder>| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::new(
                "A",
                |v: &Rc<Holder>| v.a.borrow().clone(),
                |v, x| {
                    v.a.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "B",
                |v: &Rc<Holder>| v.b.borrow().clone(),
                |v, x| {
                    v.b.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "Item",
                |v: &Rc<Holder>| v.item.borrow().clone(),
                |v, x| {
                    v.item.replace(x);
                },
            ),
        ],
    );
    let mut types = vec![
        root,
        docs,
        document,
        win,
        base,
        root_base,
        windows,
        holder,
        JsonTypeInfo::list::<Option<Rc<dyn IDockable>>>(),
        JsonTypeInfo::list::<Option<Rc<dyn IDockWindow>>>(),
        JsonTypeInfo::optional::<Items>(),
        JsonTypeInfo::optional::<Windows>(),
        JsonTypeInfo::optional::<Rc<dyn IDockable>>(),
        JsonTypeInfo::optional::<Rc<dyn IRootDock>>(),
        JsonTypeInfo::optional::<Rc<dyn IDockWindow>>(),
    ];
    types.extend(core_model_type_metadata());
    GeneratedJsonTypeInfoResolver::new(types)
}
fn source_check(name: &str, value: bool) {
    assert!(value, "{name}");
    assert!(
        include_str!("../../../tests/Dock/data/serializer-json-audit.model.tsv")
            .lines()
            .any(|row| row == format!("{name}\t1")),
        "missing source case {name}"
    );
}
#[test]
fn actual_model_projection_retains_interface_references_windows_lists_and_engine_behavior() {
    let serializer = DockSerializer::with_resolver(Some(resolver())).unwrap();
    let source = include_str!("../../../tests/Dock/data/serializer-json-audit.model.root.json");
    let root = serializer
        .deserialize::<Rc<RootDock>>(Some(source))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(root.clone()).unwrap(), source);
    let docs = concrete::<DocumentDock>(root.visible_dockables().unwrap().get(0).as_ref().unwrap())
        .unwrap();
    let doc =
        concrete::<Document>(docs.visible_dockables().unwrap().get(0).as_ref().unwrap()).unwrap();
    source_check(
        "model-root-undefined-dock-mode",
        root.dock().to_raw() == -19,
    );
    source_check(
        "model-root-undefined-pinned-and-host-mode",
        root.pinned_dock_display_mode().to_raw() == 123
            && root.floating_window_host_mode().to_raw() == -17,
    );
    source_check(
        "model-dock-undefined-signed-boundary",
        docs.dock().to_raw() == i32::MIN,
    );
    source_check(
        "model-document-undefined-dock-mode",
        doc.dock().to_raw() == 123,
    );
    source_check(
        "model-pinned-override-null-and-unknown",
        root.pinned_dock_display_mode_override().is_none()
            && docs.pinned_dock_display_mode_override() == Some(PinnedDockDisplayMode::Inline)
            && doc.pinned_dock_display_mode_override().map(|v| v.to_raw()) == Some(i32::MIN),
    );
    source_check(
        "model-policy-alias-reference",
        Rc::ptr_eq(
            &root.root_dock_capability_policy().unwrap(),
            &docs.dock_capability_policy().unwrap(),
        ),
    );
    source_check(
        "model-docking-state-numeric-flags",
        doc.docking_state() == DockingWindowState::DOCUMENT,
    );
    source_check(
        "model-undefined-operation-mask-retained",
        doc.allowed_drop_operations().bits() == i32::MIN | 1,
    );
    source_check(
        "model-masks-use-original-operation-rules",
        doc.allowed_dock_operations()
            .allows(DockOperation::OuterLeft)
            && !doc.allowed_dock_operations().allows(DockOperation::Right)
            && doc.allowed_drop_operations().allows(DockOperation::Fill)
            && !doc.allowed_drop_operations().allows(DockOperation::Window),
    );
    let pinned = doc.pinned_bounds().unwrap();
    source_check(
        "model-pinned-bounds-original-accessors",
        pinned.x.to_bits() == (-0.0f64).to_bits()
            && pinned.y == 0.125
            && pinned.width == 512.5
            && pinned.height == 256.25,
    );
    let overrides = doc.dock_capability_overrides().unwrap();
    source_check(
        "model-override-values-and-ignored-computed-state",
        overrides.can_close() == Some(true)
            && overrides.can_float() == Some(false)
            && overrides.has_any_override()
            && !source.contains("HasAnyOverride"),
    );
    source_check(
        "model-concrete-root-no-discriminator",
        JsonValue::parse(source)
            .unwrap()
            .property("$type")
            .is_none(),
    );
    source_check(
        "model-derived-document-type-tag",
        source.contains("Dock.Model.ReactiveUI.Controls.Document\""),
    );
    source_check(
        "model-root-concrete-to-owner-interface-reference",
        Rc::ptr_eq(&docs.owner().unwrap(), &(root.clone() as Rc<dyn IDockable>)),
    );
    source_check(
        "model-dock-concrete-to-owner-interface-reference",
        Rc::ptr_eq(&doc.owner().unwrap(), &(docs.clone() as Rc<dyn IDockable>)),
    );
    source_check(
        "model-root-default-same-visible",
        Rc::ptr_eq(
            &root.default_dockable().unwrap(),
            &(docs.clone() as Rc<dyn IDockable>),
        ),
    );
    source_check(
        "model-document-active-same-visible",
        Rc::ptr_eq(
            &docs.active_dockable().unwrap(),
            &(doc.clone() as Rc<dyn IDockable>),
        ),
    );
    source_check(
        "model-document-default-same-visible",
        Rc::ptr_eq(
            &docs.default_dockable().unwrap(),
            &(doc.clone() as Rc<dyn IDockable>),
        ),
    );
    source_check(
        "model-window-root-interface-reference",
        Rc::ptr_eq(
            &root.window().unwrap().layout().unwrap(),
            &(root.clone() as Rc<dyn IRootDock>),
        ),
    );
    let windows = root.windows().unwrap();
    let first = windows.get(0).unwrap();
    source_check(
        "model-window-parent-reference-alias",
        Rc::ptr_eq(&first.parent_window().unwrap(), &root.window().unwrap()),
    );
    source_check(
        "model-window-undefined-state-and-owner-retained",
        first.window_state().to_raw() == 123 && first.owner_mode().to_raw() == -19,
    );
    source_check(
        "model-window-presentation-flags-retained",
        first.topmost()
            && first.is_modal()
            && first.show_in_taskbar() == Some(false)
            && root.window().unwrap().show_in_taskbar().is_none(),
    );
    source_check(
        "model-window-shared-reference",
        Rc::ptr_eq(&windows.get(0).unwrap(), &windows.get(1).unwrap()),
    );
    source_check(
        "model-default-nan-proportions-and-constraints",
        root.proportion().is_nan() && root.min_width().is_nan() && doc.max_height().is_nan(),
    );
    source_check(
        "model-finite-proportion-and-collapsed-values",
        doc.proportion() == 0.125
            && docs.collapsed_proportion() == 0.25
            && doc.min_width() == 128.5,
    );
    source_check(
        "model-named-infinite-size",
        doc.max_width() == f64::INFINITY,
    );
    source_check(
        "model-negative-zero-size",
        doc.min_height().to_bits() == (-0.0_f64).to_bits(),
    );
    let float = windows.get(0).unwrap();
    source_check(
        "model-window-coordinate-and-size-wire",
        float.x().to_bits() == 0.0_f64.to_bits()
            && float.y() == 1e-5
            && float.width() == 1024.25
            && float.height() == 768.0,
    );
    source_check(
        "model-observable-dockable-list",
        root.visible_dockables()
            .unwrap()
            .as_observable_collection()
            .is_some(),
    );
    source_check(
        "model-observable-window-list",
        windows.as_observable_collection().is_some(),
    );
    let plain =
        DockSerializer::with_list_type_and_resolver(Some(JsonListType::list()), Some(resolver()))
            .unwrap()
            .deserialize::<Rc<RootDock>>(Some(source))
            .unwrap()
            .unwrap();
    source_check(
        "model-configured-plain-dockable-list",
        plain
            .visible_dockables()
            .unwrap()
            .as_observable_collection()
            .is_none(),
    );
    source_check(
        "model-configured-plain-window-list",
        plain
            .windows()
            .unwrap()
            .as_observable_collection()
            .is_none(),
    );
    let factory = Factory::new();
    factory.init_layout(root.clone());
    source_check(
        "model-original-engine-retains-dock-and-pinned-modes",
        doc.dock().to_raw() == 123
            && doc.pinned_dock_display_mode_override().map(|v| v.to_raw()) == Some(i32::MIN)
            && root.floating_window_host_mode().to_raw() == -17,
    );
    source_check(
        "model-original-engine-initializes-deserialized-layout",
        Rc::ptr_eq(&doc.factory().unwrap(), &(factory as Rc<dyn IFactory>))
            && Rc::ptr_eq(&doc.owner().unwrap(), &(docs.clone() as Rc<dyn IDockable>)),
    );
    let capability =
        ferrodock_model::DockCapabilityResolver::evaluate(Some(doc.clone()), DockCapability::Close);
    source_check(
        "model-restored-original-capability-precedence",
        capability.effective_value()
            && capability.root_policy_value() == Some(false)
            && capability.dock_policy_value() == Some(false)
            && capability.dockable_override_value() == Some(true)
            && capability.effective_source() == DockCapabilityValueSource::DockableOverride,
    );
    root.root_dock_capability_policy()
        .unwrap()
        .set_can_pin(Some(false));
    source_check(
        "model-policy-mutation-same-original-instance",
        docs.dock_capability_policy().unwrap().can_pin() == Some(false)
            && !ferrodock_model::DockCapabilityResolver::is_enabled(
                Some(doc),
                DockCapability::Pin,
                Some(docs),
            ),
    );
    root.root_dock_capability_policy()
        .unwrap()
        .set_can_pin(Some(true));
}
#[test]
fn actual_shared_list_elements_custom_creator_and_interface_root_cycles_match_source() {
    let serializer = DockSerializer::with_resolver(Some(resolver())).unwrap();
    let json = include_str!("../../../tests/Dock/data/serializer-json-audit.model.list.json");
    let holder = serializer
        .deserialize::<Rc<Holder>>(Some(json))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(holder.clone()).unwrap(), json);
    let a = holder.a.borrow().clone().unwrap();
    let b = holder.b.borrow().clone().unwrap();
    source_check("model-list-alias-reference", a.ptr_eq(&b));
    source_check(
        "model-list-item-alias-reference",
        Rc::ptr_eq(&a.get(0).unwrap(), holder.item.borrow().as_ref().unwrap()),
    );
    source_check(
        "model-list-null-entry-retained",
        a.count() == 2 && a.get(1).is_none(),
    );
    let created = Rc::new(RefCell::new(Vec::<usize>::new()));
    let calls = created.clone();
    let list_type = JsonListType::custom("CountedList`1", move |element| {
        if element == std::any::TypeId::of::<Option<Rc<dyn IDockable>>>() {
            let list = Items::from_plain(Vec::new());
            calls.borrow_mut().push(list.identity());
            Ok(Box::new(list))
        } else if element == std::any::TypeId::of::<Option<Rc<dyn IDockWindow>>>() {
            let list = Windows::from_plain(Vec::new());
            calls.borrow_mut().push(list.identity());
            Ok(Box::new(list))
        } else {
            Err(DockSerializationError::new(std::io::Error::other(
                "unregistered compiled element type",
            )))
        }
    });
    let custom = DockSerializer::with_list_type_and_resolver(Some(list_type), Some(resolver()))
        .unwrap()
        .deserialize::<Rc<Holder>>(Some(json))
        .unwrap()
        .unwrap();
    source_check(
        "model-custom-list-constructor",
        custom
            .a
            .borrow()
            .as_ref()
            .unwrap()
            .as_observable_collection()
            .is_none()
            && created
                .borrow()
                .iter()
                .filter(|id| **id == custom.a.borrow().as_ref().unwrap().identity())
                .count()
                == 1,
    );
    source_check(
        "model-custom-list-alias-same-constructor-instance",
        custom
            .a
            .borrow()
            .as_ref()
            .unwrap()
            .ptr_eq(custom.b.borrow().as_ref().unwrap()),
    );
    let doc = a.get(0).unwrap();
    let text = serializer.serialize(doc).unwrap();
    let restored = serializer
        .deserialize::<Rc<dyn IDockable>>(Some(&text))
        .unwrap()
        .unwrap();
    let concrete = concrete::<Document>(&restored);
    source_check(
        "model-interface-root-actual-concrete-class",
        concrete.is_some(),
    );
    let parent = restored.owner().unwrap().as_dock().unwrap();
    source_check(
        "model-interface-root-owner-cycle",
        Rc::ptr_eq(
            &parent.visible_dockables().unwrap().get(0).unwrap(),
            &restored,
        ),
    );
}
#[test]
fn unrecognized_and_invalid_polymorphic_metadata_never_fabricate_models() {
    let serializer = DockSerializer::with_resolver(Some(resolver())).unwrap();
    for (name, json) in [
        (
            "model-unknown-discriminator-interface-not-fabricated",
            r#"{"$type":"future","Id":"unknown"}"#,
        ),
        (
            "model-late-discriminator-rejected",
            r#"{"Id":"x","$type":"Dock.Model.ReactiveUI.Controls.Document"}"#,
        ),
        (
            "model-duplicate-discriminator-rejected",
            r#"{"$type":"x","$type":"y"}"#,
        ),
    ] {
        source_check(
            name,
            serializer
                .deserialize::<Rc<dyn IDockable>>(Some(json))
                .is_err(),
        );
    }
}
