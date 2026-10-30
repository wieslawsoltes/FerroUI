// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original local/global marker casts needed by the host-state engine.
use crate::{
    controls::*,
    core::{DockableBase, DockableBaseImpl},
};
use ferrodock_model::core::{IDockable, IGlobalTarget, ILocalTarget};
use ferroui_reactiveui::reactive_object;
use std::rc::{Rc, Weak};
fn identity<T: ?Sized>(source: &Rc<dyn IDockable>, marker: &Rc<T>) {
    assert_eq!(
        Rc::as_ptr(source) as *const (),
        Rc::as_ptr(marker) as *const ()
    );
}
#[test]
fn original_concrete_local_and_global_contracts_retain_same_source_reference() {
    let locals: Vec<Rc<dyn IDockable>> = vec![
        Document::new(),
        Tool::new(),
        DocumentDock::new(),
        ToolDock::new(),
    ];
    for model in locals {
        identity(&model, &model.clone().as_local_target().unwrap());
        assert!(model.clone().as_global_target().is_none());
    }
    let global: Rc<dyn IDockable> = ProportionalDock::new();
    identity(&global, &global.clone().as_global_target().unwrap());
    assert!(global.as_local_target().is_none());
}
#[test]
fn ordinary_docks_splitters_and_root_are_not_promoted_to_source_target_markers() {
    let items: Vec<Rc<dyn IDockable>> = vec![
        RootDock::new(),
        DockDock::new(),
        StackDock::new(),
        WrapDock::new(),
        UniformGridDock::new(),
        GridDock::new(),
        SplitViewDock::new(),
        ProportionalDockSplitter::new(),
        GridDockSplitter::new(),
    ];
    for item in items {
        assert!(item.clone().as_local_target().is_none());
        assert!(item.as_global_target().is_none());
    }
}
struct CustomTargets {
    base: DockableBase,
}
reactive_object!(CustomTargets, base);
impl DockableBaseImpl for CustomTargets {}
impl ILocalTarget for CustomTargets {}
impl IGlobalTarget for CustomTargets {}
crate::impl_dockable!(
    CustomTargets,
    fn as_local_target(self: Rc<Self>) -> Option<Rc<dyn ILocalTarget>> {
        Some(self)
    },
    fn as_global_target(self: Rc<Self>) -> Option<Rc<dyn IGlobalTarget>> {
        Some(self)
    }
);
impl CustomTargets {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime: &Weak<Self>| Self {
            base: DockableBase::construct(runtime.clone()),
        })
    }
}
#[test]
fn custom_marker_only_model_exposes_both_original_contracts_without_fabricated_dock_role() {
    let model: Rc<dyn IDockable> = CustomTargets::new();
    let weak = Rc::downgrade(&model);
    let local = model.clone().as_local_target().unwrap();
    let global = model.clone().as_global_target().unwrap();
    identity(&model, &local);
    identity(&model, &global);
    assert!(model.clone().as_dock().is_none());
    assert_eq!(model.runtime_type_name(), "CustomTargets");
    drop(model);
    assert!(weak.upgrade().is_some());
    drop(local);
    assert!(weak.upgrade().is_some());
    drop(global);
    assert!(weak.upgrade().is_none());
}
#[test]
fn runtime_class_names_follow_the_original_concrete_model_behind_erased_views() {
    for (model, name) in [
        (Document::new() as Rc<dyn IDockable>, "Document"),
        (Tool::new(), "Tool"),
        (ProportionalDock::new(), "ProportionalDock"),
        (RootDock::new(), "RootDock"),
    ] {
        assert_eq!(model.runtime_type_name(), name);
    }
}

struct CustomProportional {
    base: ProportionalDock,
}
reactive_object!(CustomProportional, base);
impl DockableBaseImpl for CustomProportional {}
impl IGlobalTarget for CustomProportional {}
crate::impl_dock!(@base CustomProportional, ProportionalDock,
    fn as_proportional_dock(self: Rc<Self>) -> Option<Rc<dyn ferrodock_model::controls::IProportionalDock>> { Some(self) }
);
impl ferrodock_model::controls::IProportionalDock for CustomProportional {
    fn orientation(&self) -> ferrodock_model::core::Orientation {
        self.base.orientation()
    }
    fn set_orientation(&self, value: ferrodock_model::core::Orientation) {
        self.base.set_orientation(value);
    }
}
struct ToolOnly {
    base: DockableBase,
}
reactive_object!(ToolOnly, base);
impl DockableBaseImpl for ToolOnly {}
impl ILocalTarget for ToolOnly {}
impl ferrodock_model::controls::ITool for ToolOnly {}
crate::impl_dockable!(
    ToolOnly,
    fn as_tool(self: Rc<Self>) -> Option<Rc<dyn ferrodock_model::controls::ITool>> {
        Some(self)
    }
);
#[test]
fn inherited_global_and_tool_only_views_resolve_actual_supertraits_on_same_outer_model() {
    let proportional = Rc::new_cyclic(|runtime: &Weak<CustomProportional>| CustomProportional {
        base: ProportionalDock::construct(runtime),
    });
    proportional.base.initialize(&proportional);
    let model: Rc<dyn IDockable> = proportional;
    identity(&model, &model.clone().as_global_target().unwrap());
    assert!(model.as_local_target().is_none());
    let tool = Rc::new_cyclic(|runtime: &Weak<ToolOnly>| ToolOnly {
        base: DockableBase::construct(runtime.clone()),
    });
    let model: Rc<dyn IDockable> = tool;
    identity(&model, &model.clone().as_local_target().unwrap());
    assert!(model.as_global_target().is_none());
}
