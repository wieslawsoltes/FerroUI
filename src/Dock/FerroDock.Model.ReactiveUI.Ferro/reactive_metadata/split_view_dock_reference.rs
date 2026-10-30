// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model::core::{SplitViewDisplayMode, SplitViewPanePlacement};
use ferrodock_model::{
    controls::ISplitViewDock,
    core::{IDock, IDockable},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_reactiveui::IReactiveObject;
use ferroui_system::{
    component_model::{INotifyPropertyChanged, INotifyPropertyChanging},
    events::Event,
};
use std::rc::Rc;
/// Identity-preserving native class view of the original SplitViewDock model.
#[derive(Clone)]
pub struct SplitViewDockReference {
    model: Rc<dyn ISplitViewDock>,
    base: DockBaseReference,
}
impl SplitViewDockReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::SplitViewDock::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: ISplitViewDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn ISplitViewDock> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for SplitViewDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for SplitViewDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for SplitViewDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class SplitViewDockReference as "SplitViewDock" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: SplitViewDockReference,
 handles: [SplitViewDockReference, Option<SplitViewDockReference>], base: DockBaseReference,
 interfaces: [Rc<dyn ISplitViewDock>],
 constructors: [() => SplitViewDockReference::create], notify_property_changed: SplitViewDockReference,
 properties: [
  CompactPaneLength: f64 { get: |this: &SplitViewDockReference| this.model.compact_pane_length(), set: |this: &SplitViewDockReference, value| this.model.set_compact_pane_length(value) },
  DisplayMode: SplitViewDisplayMode { get: |this: &SplitViewDockReference| this.model.display_mode(), set: |this: &SplitViewDockReference, value| this.model.set_display_mode(value) },
  IsPaneOpen: bool { get: |this: &SplitViewDockReference| this.model.is_pane_open(), set: |this: &SplitViewDockReference, value| this.model.set_is_pane_open(value) },
  OpenPaneLength: f64 { get: |this: &SplitViewDockReference| this.model.open_pane_length(), set: |this: &SplitViewDockReference, value| this.model.set_open_pane_length(value) },
  PanePlacement: SplitViewPanePlacement { get: |this: &SplitViewDockReference| this.model.pane_placement(), set: |this: &SplitViewDockReference, value| this.model.set_pane_placement(value) },
  UseLightDismissOverlayMode: bool { get: |this: &SplitViewDockReference| this.model.use_light_dismiss_overlay_mode(), set: |this: &SplitViewDockReference, value| this.model.set_use_light_dismiss_overlay_mode(value) },
  PaneDockable: Option<Rc<dyn IDockable>> { get: |this: &SplitViewDockReference| this.model.pane_dockable(), set: |this: &SplitViewDockReference, value| this.model.set_pane_dockable(value) },
  ContentDockable: Option<Rc<dyn IDockable>> { get: |this: &SplitViewDockReference| this.model.content_dockable(), set: |this: &SplitViewDockReference, value| this.model.set_content_dockable(value) },
 ],
 methods: [fn OpenPane() => (|this: &SplitViewDockReference| this.model.open_pane()),fn ClosePane() => (|this: &SplitViewDockReference| this.model.close_pane()),fn TogglePane() => (|this: &SplitViewDockReference| this.model.toggle_pane())],
});
pub(crate) fn register() {
    MarkupType::register(SplitViewDockReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn ISplitViewDock>>();
    ValueTypes::register_cast::<Rc<dyn ISplitViewDock>, Rc<dyn IDockable>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn ISplitViewDock>, Rc<dyn IDock>>(|model| model.clone());
    ValueTypes::register_nullable::<SplitViewDockReference>();
    ValueTypes::register_cast::<SplitViewDockReference, super::DockBaseReference>(|value| {
        value.base()
    });
    ValueTypes::register_cast::<SplitViewDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<SplitViewDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<SplitViewDockReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base().base()
    });
    ValueTypes::register_cast::<SplitViewDockReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<SplitViewDockReference, Rc<dyn IDock>>(|value| value.model());
    ValueTypes::register_cast::<SplitViewDockReference, Rc<dyn ISplitViewDock>>(|value| {
        value.model()
    });
    ValueTypes::register_reference_identity::<SplitViewDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
