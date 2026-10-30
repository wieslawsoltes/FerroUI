use ferrodock_model::{
    collections::ListHandle,
    controls::{IRootDock, IToolDock},
    core::{
        DockCapabilityPolicyImpl, DockFloatingWindowHostMode, IDockWindow, IDockable,
        PinnedDockDisplayMode,
    },
};
// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_reactiveui::{ICommand, IReactiveObject};
use ferroui_system::{
    component_model::{INotifyPropertyChanged, INotifyPropertyChanging},
    events::Event,
};
use std::rc::Rc;
/// Native reference to the original RootDock class, retaining the enclosing object.
#[derive(Clone)]
pub struct RootDockReference {
    model: Rc<dyn IRootDock>,
    base: DockBaseReference,
}
impl RootDockReference {
    /// Constructs the source class and exposes the same original engine instance.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::RootDock::new())
    }

    /// Creates the source base-class view without copying or replacing its model.
    pub fn new<T: IRootDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: super::DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the actual model behind this class reference.
    pub fn model(&self) -> Rc<dyn IRootDock> {
        self.model.clone()
    }
    /// Gets the source base-class reference to the same enclosing model.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for RootDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for RootDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for RootDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class RootDockReference as "RootDock"{namespace:"FerroDock.Model.ReactiveUI.Controls",this:RootDockReference,handles:[RootDockReference,Option<RootDockReference>],base:DockBaseReference,interfaces:[Rc<dyn IRootDock>],constructors:[()=>RootDockReference::create],notify_property_changed:RootDockReference,properties:[
        IsFocusableRoot:bool{get:|this:&RootDockReference|this.model.is_focusable_root(),try_set:|this:&RootDockReference,value:bool|{this.model.set_is_focusable_root(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        HiddenDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&RootDockReference|this.model.hidden_dockables(),try_set:|this:&RootDockReference,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.model.set_hidden_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        LeftPinnedDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&RootDockReference|this.model.left_pinned_dockables(),try_set:|this:&RootDockReference,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.model.set_left_pinned_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        RightPinnedDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&RootDockReference|this.model.right_pinned_dockables(),try_set:|this:&RootDockReference,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.model.set_right_pinned_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        TopPinnedDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&RootDockReference|this.model.top_pinned_dockables(),try_set:|this:&RootDockReference,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.model.set_top_pinned_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        BottomPinnedDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&RootDockReference|this.model.bottom_pinned_dockables(),try_set:|this:&RootDockReference,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.model.set_bottom_pinned_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        PinnedDock:Option<Rc<dyn IToolDock>>{get:|this:&RootDockReference|this.model.pinned_dock(),try_set:|this:&RootDockReference,value:Option<Rc<dyn IToolDock>>|{this.model.set_pinned_dock(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        PinnedDockDisplayMode:PinnedDockDisplayMode{get:|this:&RootDockReference|this.model.pinned_dock_display_mode(),try_set:|this:&RootDockReference,value:PinnedDockDisplayMode|{this.model.set_pinned_dock_display_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        Window:Option<Rc<dyn IDockWindow>>{get:|this:&RootDockReference|this.model.window(),try_set:|this:&RootDockReference,value:Option<Rc<dyn IDockWindow>>|{this.model.set_window(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        Windows:Option<ListHandle<Option<Rc<dyn IDockWindow>>>>{get:|this:&RootDockReference|this.model.windows(),try_set:|this:&RootDockReference,value:Option<ListHandle<Option<Rc<dyn IDockWindow>>>>|{this.model.set_windows(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        FloatingWindowHostMode:DockFloatingWindowHostMode{get:|this:&RootDockReference|this.model.floating_window_host_mode(),try_set:|this:&RootDockReference,value:DockFloatingWindowHostMode|{this.model.set_floating_window_host_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        RootDockCapabilityPolicy:Option<Rc<dyn DockCapabilityPolicyImpl>>{get:|this:&RootDockReference|this.model.root_dock_capability_policy(),try_set:|this:&RootDockReference,value:Option<Rc<dyn DockCapabilityPolicyImpl>>|{this.model.set_root_dock_capability_policy(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        ShowWindows:Rc<dyn ICommand>{try_get:|this:&RootDockReference|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.model.show_windows())} [IgnoreDataMember],
        ExitWindows:Rc<dyn ICommand>{try_get:|this:&RootDockReference|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.model.exit_windows())} [IgnoreDataMember],
        EnableAdaptiveGlobalDockTargets:bool{get:|this:&RootDockReference|this.model.enable_adaptive_global_dock_targets(),try_set:|this:&RootDockReference,value:bool|{this.model.set_enable_adaptive_global_dock_targets(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],});
pub(crate) fn register() {
    ValueTypes::register_cast::<RootDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<RootDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<RootDockReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base().base()
    });
    ValueTypes::register_cast::<RootDockReference, Rc<dyn IDockable>>(|value| value.model());

    MarkupType::register(<RootDockReference as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<RootDockReference>();
    ValueTypes::register_cast::<RootDockReference, DockBaseReference>(|value| value.base());
    ValueTypes::register_cast::<RootDockReference, Rc<dyn IRootDock>>(|value| value.model());
}
