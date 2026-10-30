// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Metadata delegates to original interface members and original factory commands.
use ferrodock_model::{
    collections::ListHandle,
    controls::{IDocumentDock, IToolDock},
    core::*,
};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, Tool, ToolDock},
    Factory,
};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{any::TypeId, rc::Rc};
fn meta<T: 'static>() -> &'static MarkupType {
    MarkupType::find_by_handle(TypeId::of::<T>()).unwrap()
}
fn property(mut m: &MarkupType, name: &str) -> &'static ferroui_base::metadata::MarkupProperty {
    loop {
        if let Some(p) = m.find_property(name) {
            return p;
        }
        m = m
            .base_type()
            .unwrap_or_else(|| panic!("missing property {name}"));
    }
}
fn get(m: &MarkupType, this: &BoxedValue, name: &str) -> Option<BoxedValue> {
    (property(m, name).get.unwrap())(&[Some(this.clone())]).unwrap()
}
fn set(m: &MarkupType, this: &BoxedValue, name: &str, value: Option<BoxedValue>) {
    (property(m, name).set.unwrap())(&[Some(this.clone()), value]).unwrap();
}
fn invoke(
    m: &MarkupType,
    this: &BoxedValue,
    name: &str,
    args: &[Option<BoxedValue>],
) -> Option<BoxedValue> {
    let mut a = vec![Some(this.clone())];
    a.extend_from_slice(args);
    (m.find_methods(name).next().unwrap().invoke)(&a).unwrap()
}
#[test]
fn complete_dock_interface_fields_and_commands_forward_to_same_original_model() {
    crate::register_types();
    let dock = DocumentDock::new();
    let handle: Rc<dyn IDock> = dock.clone();
    let this: BoxedValue = Rc::new(handle);
    let m = meta::<Rc<dyn IDock>>();
    assert_eq!(m.properties.len(), 15);
    let d: Rc<dyn IDockable> = Document::new();
    let list = ListHandle::from_items(vec![Some(d.clone())]);
    set(m, &this, "VisibleDockables", Some(Rc::new(list.clone())));
    assert!(dock.visible_dockables().unwrap().ptr_eq(&list));
    let actual = get(m, &this, "VisibleDockables").unwrap();
    assert!(actual
        .downcast_ref::<ListHandle<Option<Rc<dyn IDockable>>>>()
        .unwrap()
        .ptr_eq(&list));
    for name in ["ActiveDockable", "DefaultDockable", "FocusedDockable"] {
        set(m, &this, name, Some(Rc::new(d.clone())));
        let value = get(m, &this, name).unwrap();
        assert!(Rc::ptr_eq(
            value.downcast_ref::<Rc<dyn IDockable>>().unwrap(),
            &d
        ));
        set(m, &this, name, None);
        assert!(get(m, &this, name).is_none());
    }
    for name in ["IsActive", "CanCloseLastDockable", "EnableGlobalDocking"] {
        for value in [false, true] {
            set(m, &this, name, Some(Rc::new(value)));
            assert_eq!(
                get(m, &this, name).unwrap().downcast_ref::<bool>(),
                Some(&value)
            );
        }
    }
    set(m, &this, "OpenedDockablesCount", Some(Rc::new(23_i32)));
    assert_eq!(dock.opened_dockables_count(), 23);
    let policy: Rc<dyn DockCapabilityPolicyImpl> = DockCapabilityPolicy::new();
    set(
        m,
        &this,
        "DockCapabilityPolicy",
        Some(Rc::new(policy.clone())),
    );
    assert!(Rc::ptr_eq(&dock.dock_capability_policy().unwrap(), &policy));
    set(m, &this, "DockCapabilityPolicy", None);
    assert!(get(m, &this, "DockCapabilityPolicy").is_none());
    for (name, expected) in [
        ("GoBack", dock.go_back()),
        ("GoForward", dock.go_forward()),
        ("Navigate", dock.navigate()),
        ("Close", dock.close()),
    ] {
        let value = get(m, &this, name).unwrap();
        assert!(Rc::ptr_eq(
            value
                .downcast_ref::<Rc<dyn ferroui_reactiveui::ICommand>>()
                .unwrap(),
            &expected
        ));
    }
    assert_eq!(
        get(m, &this, "CanGoBack").unwrap().downcast_ref::<bool>(),
        Some(&dock.can_go_back())
    );
    assert_eq!(
        get(m, &this, "CanGoForward")
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&dock.can_go_forward())
    );
    set(m, &this, "VisibleDockables", None);
    assert!(dock.visible_dockables().is_none());
}
#[test]
fn document_and_tool_members_enum_fields_content_and_add_methods_use_original_factory() {
    crate::register_types();
    let factory = Factory::new();
    let dock = DocumentDock::new();
    dock.set_factory(Some(factory.clone()));
    let contract: Rc<dyn IDocumentDock> = dock.clone();
    let this: BoxedValue = Rc::new(contract);
    let m = meta::<Rc<dyn IDocumentDock>>();
    assert_eq!(m.properties.len(), 11);
    assert_eq!(
        get(m, &this, "Title").unwrap().downcast_ref::<String>(),
        Some(&dock.title())
    );
    get(m, &this, "VisibleDockables");
    for name in ["CanCreateDocument", "EnableWindowDrag"] {
        set(m, &this, name, Some(Rc::new(true)));
        assert_eq!(
            get(m, &this, name).unwrap().downcast_ref::<bool>(),
            Some(&true)
        );
    }
    set(
        m,
        &this,
        "LayoutMode",
        Some(Rc::new(DocumentLayoutMode::Mdi)),
    );
    assert_eq!(dock.layout_mode(), DocumentLayoutMode::Mdi);
    set(
        m,
        &this,
        "TabsLayout",
        Some(Rc::new(DocumentTabLayout::Right)),
    );
    assert_eq!(dock.tabs_layout(), DocumentTabLayout::Right);
    set(
        m,
        &this,
        "CloseButtonShowMode",
        Some(Rc::new(DocumentCloseButtonShowMode::Never)),
    );
    assert_eq!(
        dock.close_button_show_mode(),
        DocumentCloseButtonShowMode::Never
    );
    let content: BoxedValue = Rc::new("original empty".to_owned());
    set(m, &this, "EmptyContent", Some(content.clone()));
    assert!(Rc::ptr_eq(
        &get(m, &this, "EmptyContent").unwrap(),
        &content
    ));
    set(m, &this, "EmptyContent", None);
    assert!(dock.empty_content().is_none());
    for name in [
        "CreateDocument",
        "CascadeDocuments",
        "TileDocumentsHorizontal",
        "TileDocumentsVertical",
        "RestoreDocuments",
    ] {
        let command = dock.go_back();
        set(m, &this, name, Some(Rc::new(command.clone())));
        assert!(Rc::ptr_eq(
            get(m, &this, name)
                .unwrap()
                .downcast_ref::<Rc<dyn ferroui_reactiveui::ICommand>>()
                .unwrap(),
            &command
        ));
        set(m, &this, name, None);
        assert!(get(m, &this, name).is_none());
    }
    for name in ["AddDocument", "AddTool"] {
        let d: Rc<dyn IDockable> = Document::new();
        invoke(m, &this, name, &[Some(Rc::new(d.clone()))]);
        assert!(dock
            .visible_dockables()
            .unwrap()
            .contains(&Some((d).clone())));
        assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &d));
    }
    let tool = ToolDock::new();
    tool.set_factory(Some(factory.clone()));
    let handle: Rc<dyn IToolDock> = tool.clone();
    let this: BoxedValue = Rc::new(handle);
    let m = meta::<Rc<dyn IToolDock>>();
    assert_eq!(m.properties.len(), 4);
    set(m, &this, "Alignment", Some(Rc::new(Alignment::Unset)));
    assert_eq!(tool.alignment(), Alignment::Unset);
    set(m, &this, "GripMode", Some(Rc::new(GripMode::AutoHide)));
    assert_eq!(tool.grip_mode(), GripMode::AutoHide);
    for name in ["IsExpanded", "AutoHide"] {
        set(m, &this, name, Some(Rc::new(true)));
        assert_eq!(
            get(m, &this, name).unwrap().downcast_ref::<bool>(),
            Some(&true)
        );
    }
    // Keep the source factory strongly owned while the model's factory links remain weak.
    let d: Rc<dyn IDockable> = Tool::new();
    invoke(m, &this, "AddTool", &[Some(Rc::new(d.clone()))]);
    assert!(tool
        .visible_dockables()
        .unwrap()
        .contains(&Some((d).clone())));
}
#[test]
fn policy_descriptors_keep_original_optional_fields_and_virtual_get() {
    crate::register_types();
    let policy: Rc<dyn DockCapabilityPolicyImpl> = DockCapabilityPolicy::new();
    let interface: Rc<dyn IDockCapabilityPolicy> = policy.clone();
    let value: BoxedValue = Rc::new(interface);
    let m = meta::<Rc<dyn IDockCapabilityPolicy>>();
    for (i, name) in [
        "CanClose",
        "CanPin",
        "CanFloat",
        "CanDrag",
        "CanDrop",
        "CanDockAsDocument",
    ]
    .into_iter()
    .enumerate()
    {
        for flag in [Some(true), Some(false), None] {
            set(m, &value, name, flag.map(|v| Rc::new(v) as BoxedValue));
            assert_eq!(
                get(m, &value, name).and_then(|v| v.downcast_ref::<bool>().copied()),
                flag
            );
            assert_eq!(policy.get_raw(i as i32), flag);
        }
    }
    let overrides = DockCapabilityOverrides::new();
    let this: BoxedValue = Rc::new(overrides.clone());
    let m = meta::<Rc<DockCapabilityOverrides>>();
    assert_eq!(
        get(m, &this, "HasAnyOverride")
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&false)
    );
    set(m, &this, "CanClose", Some(Rc::new(false)));
    assert!(overrides.has_any_override());
    assert_eq!(
        get(m, &this, "CanClose").unwrap().downcast_ref::<bool>(),
        Some(&false)
    );
    let class = <dyn crate::DockCapabilityPolicyMetadata as MarkupTyped>::MARKUP;
    let this: BoxedValue = Rc::new(policy);
    assert!(invoke(class, &this, "Get", &[Some(Rc::new(DockCapability::Close))]).is_none());
}
#[test]
fn checked_dock_casts_and_notifiers_preserve_actual_membership_and_object_identity() {
    crate::register_types();
    let dock: Rc<dyn IDockable> = DocumentDock::new();
    let value: BoxedValue = Rc::new(dock.clone());
    let converted = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDocumentDock>>()).unwrap();
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(converted.downcast_ref::<Rc<dyn IDocumentDock>>().unwrap()),
        Rc::as_ptr(&dock)
    ));
    assert!(ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IToolDock>>()).is_none());
    let doc: Rc<dyn IDockable> = Document::new();
    let value: BoxedValue = Rc::new(doc);
    assert!(ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDock>>()).is_none());
    assert!(ValueTypes::try_cast(
        &value,
        ValueType::of::<Rc<dyn ferrodock_model::controls::IMdiDocument>>()
    )
    .is_some());
    let handle: Rc<dyn IDock> = DocumentDock::new();
    let value: BoxedValue = Rc::new(handle.clone());
    let notifier = (meta::<Rc<dyn IDock>>().notify_property_changed.unwrap())(&*value).unwrap();
    assert!(std::ptr::eq(
        notifier.property_changed(),
        handle
            .as_notify_property_changed()
            .unwrap()
            .property_changed()
    ));
}

struct VirtualPolicy(Rc<DockCapabilityPolicy>);
impl IDockCapabilityPolicy for VirtualPolicy {
    fn can_close(&self) -> Option<bool> {
        self.0.can_close()
    }
    fn set_can_close(&self, v: Option<bool>) {
        self.0.set_can_close(v);
    }
    fn can_pin(&self) -> Option<bool> {
        self.0.can_pin()
    }
    fn set_can_pin(&self, v: Option<bool>) {
        self.0.set_can_pin(v);
    }
    fn can_float(&self) -> Option<bool> {
        self.0.can_float()
    }
    fn set_can_float(&self, v: Option<bool>) {
        self.0.set_can_float(v);
    }
    fn can_drag(&self) -> Option<bool> {
        self.0.can_drag()
    }
    fn set_can_drag(&self, v: Option<bool>) {
        self.0.set_can_drag(v);
    }
    fn can_drop(&self) -> Option<bool> {
        self.0.can_drop()
    }
    fn set_can_drop(&self, v: Option<bool>) {
        self.0.set_can_drop(v);
    }
    fn can_dock_as_document(&self) -> Option<bool> {
        self.0.can_dock_as_document()
    }
    fn set_can_dock_as_document(&self, v: Option<bool>) {
        self.0.set_can_dock_as_document(v);
    }
}
impl DockCapabilityPolicyImpl for VirtualPolicy {
    fn get(&self, c: DockCapability) -> Option<bool> {
        if c == DockCapability::Close {
            Some(false)
        } else {
            self.0.get(c)
        }
    }
}
#[test]
fn native_policy_get_uses_original_custom_virtual_dispatch_and_constructors() {
    crate::register_types();
    let class = <dyn crate::DockCapabilityPolicyMetadata as MarkupTyped>::MARKUP;
    let created = (class.constructors[0].invoke)(&[]).unwrap();
    assert!(
        ferroui_base::metadata::from_markup_value::<Rc<dyn DockCapabilityPolicyImpl>>(&created)
            .is_some()
    );
    let custom: Rc<dyn DockCapabilityPolicyImpl> =
        Rc::new(VirtualPolicy(DockCapabilityPolicy::new()));
    custom.set_can_close(Some(true));
    let value: BoxedValue = Rc::new(custom);
    assert_eq!(
        get(class, &value, "CanClose")
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&true)
    );
    assert_eq!(
        invoke(
            class,
            &value,
            "Get",
            &[Some(Rc::new(DockCapability::Close))]
        )
        .unwrap()
        .downcast_ref::<bool>(),
        Some(&false)
    );
    let class = <dyn crate::DockCapabilityOverridesMetadata as MarkupTyped>::MARKUP;
    let created = (class.constructors[0].invoke)(&[]).unwrap();
    assert!(
        !ferroui_base::metadata::from_markup_value::<Rc<DockCapabilityOverrides>>(&created)
            .unwrap()
            .has_any_override()
    );
}

#[test]
fn list_metadata_uses_original_indexer_comparer_and_mutation_events() {
    crate::register_types();
    let a: Rc<dyn IDockable> = Document::new();
    let b: Rc<dyn IDockable> = Document::new();
    let list = ListHandle::from_items(vec![Some(a.clone())]);
    let this: BoxedValue = Rc::new(list.clone());
    let m = meta::<ListHandle<Option<Rc<dyn IDockable>>>>();
    assert!(std::ptr::eq(
        (m.notify_property_changed.unwrap())(&*this)
            .unwrap()
            .property_changed(),
        list.as_notify_property_changed()
            .unwrap()
            .property_changed()
    ));
    let index = &m.indexers[0];
    let read = (index.get.unwrap())(&[Some(this.clone()), Some(Rc::new(0_i32))])
        .unwrap()
        .unwrap();
    assert!(Rc::ptr_eq(
        read.downcast_ref::<Rc<dyn IDockable>>().unwrap(),
        &a
    ));
    (index.set.unwrap())(&[
        Some(this.clone()),
        Some(Rc::new(0_i32)),
        Some(Rc::new(b.clone())),
    ])
    .unwrap();
    assert!(Rc::ptr_eq(
        list.get(0).as_ref().expect("nonnull metadata fixture"),
        &b
    ));
    invoke(m, &this, "Add", &[Some(Rc::new(a.clone()))]);
    assert_eq!(
        get(m, &this, "Count").unwrap().downcast_ref::<i32>(),
        Some(&2)
    );
    assert_eq!(
        get(m, &this, "IsReadOnly").unwrap().downcast_ref::<bool>(),
        Some(&false)
    );
    assert_eq!(
        invoke(m, &this, "Contains", &[Some(Rc::new(a.clone()))])
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&true)
    );
    assert_eq!(
        invoke(m, &this, "IndexOf", &[Some(Rc::new(a.clone()))])
            .unwrap()
            .downcast_ref::<i32>(),
        Some(&1)
    );
    invoke(
        m,
        &this,
        "Insert",
        &[Some(Rc::new(1_i32)), Some(Rc::new(b.clone()))],
    );
    invoke(m, &this, "RemoveAt", &[Some(Rc::new(0_i32))]);
    assert_eq!(
        invoke(m, &this, "Remove", &[Some(Rc::new(b.clone()))])
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&true)
    );
    assert!(Rc::ptr_eq(
        list.get(0).as_ref().expect("nonnull metadata fixture"),
        &a
    ));
    invoke(m, &this, "Clear", &[]);
    assert_eq!(list.count(), 0);
    assert_eq!(
        invoke(m, &this, "IndexOf", &[Some(Rc::new(a))])
            .unwrap()
            .downcast_ref::<i32>(),
        Some(&-1)
    );
}

#[test]
fn complete_root_metadata_retains_all_original_lists_policies_windows_commands_and_notifier() {
    use ferrodock_model::controls::IRootDock;
    use ferrodock_model_reactiveui::{controls::RootDock, core::DockWindow};
    crate::register_types();
    let root = RootDock::new();
    let contract: Rc<dyn IRootDock> = root.clone();
    let this: BoxedValue = Rc::new(contract);
    let m = meta::<Rc<dyn IRootDock>>();
    assert_eq!(m.properties.len(), 15);
    assert!(std::ptr::eq(
        (m.notify_property_changed.unwrap())(&*this)
            .unwrap()
            .property_changed(),
        root.as_notify_property_changed()
            .unwrap()
            .property_changed()
    ));
    set(m, &this, "Title", Some(Rc::new("original root".to_owned())));
    assert_eq!(root.title(), "original root");
    for name in ["IsFocusableRoot", "EnableAdaptiveGlobalDockTargets"] {
        for v in [true, false] {
            set(m, &this, name, Some(Rc::new(v)));
            assert_eq!(
                get(m, &this, name).unwrap().downcast_ref::<bool>(),
                Some(&v)
            );
        }
    }
    let item: Rc<dyn IDockable> = Document::new();
    let list = ListHandle::from_plain(vec![Some(item)]);
    for name in [
        "HiddenDockables",
        "LeftPinnedDockables",
        "RightPinnedDockables",
        "TopPinnedDockables",
        "BottomPinnedDockables",
    ] {
        set(m, &this, name, Some(Rc::new(list.clone())));
        assert!(get(m, &this, name)
            .unwrap()
            .downcast_ref::<ListHandle<Option<Rc<dyn IDockable>>>>()
            .unwrap()
            .ptr_eq(&list));
        set(m, &this, name, None);
        assert!(get(m, &this, name).is_none());
    }
    let dock: Rc<dyn IToolDock> = ToolDock::new();
    set(m, &this, "PinnedDock", Some(Rc::new(dock.clone())));
    assert!(Rc::ptr_eq(
        get(m, &this, "PinnedDock")
            .unwrap()
            .downcast_ref::<Rc<dyn IToolDock>>()
            .unwrap(),
        &dock
    ));
    set(m, &this, "PinnedDock", None);
    assert!(root.pinned_dock().is_none());
    set(
        m,
        &this,
        "PinnedDockDisplayMode",
        Some(Rc::new(PinnedDockDisplayMode::Inline)),
    );
    assert_eq!(
        root.pinned_dock_display_mode(),
        PinnedDockDisplayMode::Inline
    );
    set(
        m,
        &this,
        "FloatingWindowHostMode",
        Some(Rc::new(DockFloatingWindowHostMode::Managed)),
    );
    assert_eq!(
        root.floating_window_host_mode(),
        DockFloatingWindowHostMode::Managed
    );
    let w: Rc<dyn IDockWindow> = DockWindow::new();
    set(m, &this, "Window", Some(Rc::new(w.clone())));
    assert!(Rc::ptr_eq(
        get(m, &this, "Window")
            .unwrap()
            .downcast_ref::<Rc<dyn IDockWindow>>()
            .unwrap(),
        &w
    ));
    set(m, &this, "Window", None);
    assert!(root.window().is_none());
    let windows = ListHandle::from_plain(vec![Some(w)]);
    set(m, &this, "Windows", Some(Rc::new(windows.clone())));
    assert!(get(m, &this, "Windows")
        .unwrap()
        .downcast_ref::<ListHandle<Option<Rc<dyn IDockWindow>>>>()
        .unwrap()
        .ptr_eq(&windows));
    set(m, &this, "Windows", None);
    assert!(root.windows().is_none());
    let p: Rc<dyn DockCapabilityPolicyImpl> = DockCapabilityPolicy::new();
    set(
        m,
        &this,
        "RootDockCapabilityPolicy",
        Some(Rc::new(p.clone())),
    );
    assert!(Rc::ptr_eq(
        get(m, &this, "RootDockCapabilityPolicy")
            .unwrap()
            .downcast_ref::<Rc<dyn DockCapabilityPolicyImpl>>()
            .unwrap(),
        &p
    ));
    set(m, &this, "RootDockCapabilityPolicy", None);
    assert!(root.root_dock_capability_policy().is_none());
    for (name, command) in [
        ("ShowWindows", root.show_windows()),
        ("ExitWindows", root.exit_windows()),
    ] {
        assert!(Rc::ptr_eq(
            get(m, &this, name)
                .unwrap()
                .downcast_ref::<Rc<dyn ferroui_reactiveui::ICommand>>()
                .unwrap(),
            &command
        ));
    }
    let dock: Rc<dyn IDock> = root.clone();
    let base: BoxedValue = Rc::new(dock);
    let target = ValueType::of::<Rc<dyn IRootDock>>();
    assert!(Rc::ptr_eq(
        ValueTypes::try_cast(&base, target)
            .unwrap()
            .downcast_ref::<Rc<dyn IRootDock>>()
            .unwrap(),
        &(root as Rc<dyn IRootDock>)
    ));
    let nonroot: BoxedValue = Rc::new(DocumentDock::new() as Rc<dyn IDock>);
    assert!(ValueTypes::try_cast(&nonroot, target).is_none());
}

#[test]
fn all_layout_container_descriptors_keep_original_models_properties_and_marker_casts() {
    use ferrodock_model::controls::*;
    use ferrodock_model_reactiveui::controls::{
        DockDock, GridDock, ProportionalDock, StackDock, UniformGridDock, WrapDock,
    };
    crate::register_types();
    let model = DockDock::new();
    let this = crate::model_projection::to_dockable_value(model.clone());
    let m = meta::<Rc<dyn IDockDock>>();
    set(m, &this, "LastChildFill", Some(Rc::new(false)));
    assert!(!model.last_child_fill());
    assert_eq!(
        get(m, &this, "LastChildFill")
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&false)
    );
    let model = ProportionalDock::new();
    let this = crate::model_projection::to_dockable_value(model.clone());
    let m = meta::<Rc<dyn IProportionalDock>>();
    set(
        m,
        &this,
        "Orientation",
        Some(Rc::new(Orientation::Vertical)),
    );
    assert_eq!(model.orientation(), Orientation::Vertical);
    let global = ValueTypes::try_cast(&this, ValueType::of::<Rc<dyn IGlobalTarget>>()).unwrap();
    let expected: Rc<dyn IGlobalTarget> = model;
    assert!(Rc::ptr_eq(
        global.downcast_ref::<Rc<dyn IGlobalTarget>>().unwrap(),
        &expected
    ));
    let model = GridDock::new();
    let this = crate::model_projection::to_dockable_value(model.clone());
    let m = meta::<Rc<dyn IGridDock>>();
    for name in ["ColumnDefinitions", "RowDefinitions"] {
        set(m, &this, name, Some(Rc::new("1*,Auto,50".to_owned())));
        assert_eq!(
            get(m, &this, name)
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("1*,Auto,50")
        );
        set(m, &this, name, None);
        assert!(get(m, &this, name).is_none());
    }
    let model = StackDock::new();
    let this = crate::model_projection::to_dockable_value(model.clone());
    let m = meta::<Rc<dyn IStackDock>>();
    set(
        m,
        &this,
        "Orientation",
        Some(Rc::new(Orientation::Vertical)),
    );
    set(m, &this, "Spacing", Some(Rc::new(13.0)));
    assert_eq!(model.spacing(), 13.0);
    assert_eq!(model.orientation(), Orientation::Vertical);
    let model = WrapDock::new();
    let this = crate::model_projection::to_dockable_value(model.clone());
    set(
        meta::<Rc<dyn IWrapDock>>(),
        &this,
        "Orientation",
        Some(Rc::new(Orientation::Vertical)),
    );
    assert_eq!(model.orientation(), Orientation::Vertical);
    let model = UniformGridDock::new();
    let this = crate::model_projection::to_dockable_value(model.clone());
    let m = meta::<Rc<dyn IUniformGridDock>>();
    set(m, &this, "Rows", Some(Rc::new(2_i32)));
    set(m, &this, "Columns", Some(Rc::new(3_i32)));
    assert_eq!((model.rows(), model.columns()), (2, 3));
    // A checked interface cast must reject another original container.
    assert!(ValueTypes::try_cast(&this, ValueType::of::<Rc<dyn IGridDock>>()).is_none());
}
#[test]
fn split_view_descriptor_routes_all_fields_methods_and_original_notifications() {
    use ferrodock_model::controls::ISplitViewDock;
    use ferrodock_model_reactiveui::controls::SplitViewDock;
    crate::register_types();
    let model = SplitViewDock::new();
    let this = crate::model_projection::to_dockable_value(model.clone());
    let m = meta::<Rc<dyn ISplitViewDock>>();
    for (name, value) in [("CompactPaneLength", 12.0), ("OpenPaneLength", 230.0)] {
        set(m, &this, name, Some(Rc::new(value)));
        assert_eq!(
            get(m, &this, name).unwrap().downcast_ref::<f64>(),
            Some(&value)
        );
    }
    set(
        m,
        &this,
        "DisplayMode",
        Some(Rc::new(SplitViewDisplayMode::CompactOverlay)),
    );
    assert_eq!(model.display_mode(), SplitViewDisplayMode::CompactOverlay);
    set(
        m,
        &this,
        "PanePlacement",
        Some(Rc::new(SplitViewPanePlacement::Bottom)),
    );
    assert_eq!(model.pane_placement(), SplitViewPanePlacement::Bottom);
    set(m, &this, "UseLightDismissOverlayMode", Some(Rc::new(true)));
    assert!(model.use_light_dismiss_overlay_mode());
    let document: Rc<dyn IDockable> = Document::new();
    for name in ["PaneDockable", "ContentDockable"] {
        set(m, &this, name, Some(Rc::new(document.clone())));
        assert!(Rc::ptr_eq(
            get(m, &this, name)
                .unwrap()
                .downcast_ref::<Rc<dyn IDockable>>()
                .unwrap(),
            &document
        ));
        set(m, &this, name, None);
        assert!(get(m, &this, name).is_none());
    }
    invoke(m, &this, "OpenPane", &[]);
    assert!(model.is_pane_open());
    invoke(m, &this, "TogglePane", &[]);
    assert!(!model.is_pane_open());
    invoke(m, &this, "OpenPane", &[]);
    invoke(m, &this, "ClosePane", &[]);
    assert!(!model.is_pane_open());
    let typed: Rc<dyn ISplitViewDock> = model;
    let boxed: BoxedValue = Rc::new(typed);
    assert!(m.notify_property_changed.unwrap()(&*boxed).is_some());
}
