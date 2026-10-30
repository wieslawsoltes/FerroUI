use ferrodock_model::{
    collections::ListHandle,
    core::{DockCapabilityPolicyImpl, IDock, IDockable},
};
// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockableBaseReference;
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
/// Native reference to the original DockBase class, retaining the enclosing object.
#[derive(Clone)]
pub struct DockBaseReference {
    model: Rc<dyn IDock>,
    base: DockableBaseReference,
}
impl DockBaseReference {
    /// Creates the source base-class view without copying or replacing its model.
    pub fn new<T: IDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: super::DockableBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the actual model behind this class reference.
    pub fn model(&self) -> Rc<dyn IDock> {
        self.model.clone()
    }
    /// Gets the source base-class reference to the same enclosing model.
    pub fn base(&self) -> DockableBaseReference {
        self.base.clone()
    }
}
impl PartialEq for DockBaseReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for DockBaseReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for DockBaseReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class DockBaseReference as "DockBase"{namespace:"FerroDock.Model.ReactiveUI.Core",this:DockBaseReference,handles:[DockBaseReference,Option<DockBaseReference>],base:DockableBaseReference,interfaces:[Rc<dyn IDock>],notify_property_changed:DockBaseReference,properties:[
        VisibleDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&DockBaseReference|this.model.visible_dockables(),try_set:|this:&DockBaseReference,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.model.set_visible_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        ActiveDockable:Option<Rc<dyn IDockable>>{get:|this:&DockBaseReference|this.model.active_dockable(),try_set:|this:&DockBaseReference,value:Option<Rc<dyn IDockable>>|{this.model.set_active_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        DefaultDockable:Option<Rc<dyn IDockable>>{get:|this:&DockBaseReference|this.model.default_dockable(),try_set:|this:&DockBaseReference,value:Option<Rc<dyn IDockable>>|{this.model.set_default_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        FocusedDockable:Option<Rc<dyn IDockable>>{get:|this:&DockBaseReference|this.model.focused_dockable(),try_set:|this:&DockBaseReference,value:Option<Rc<dyn IDockable>>|{this.model.set_focused_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        IsActive:bool{get:|this:&DockBaseReference|this.model.is_active(),try_set:|this:&DockBaseReference,value:bool|{this.model.set_is_active(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        OpenedDockablesCount:i32{get:|this:&DockBaseReference|this.model.opened_dockables_count(),try_set:|this:&DockBaseReference,value:i32|{this.model.set_opened_dockables_count(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CanCloseLastDockable:bool{get:|this:&DockBaseReference|this.model.can_close_last_dockable(),try_set:|this:&DockBaseReference,value:bool|{this.model.set_can_close_last_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        DockCapabilityPolicy:Option<Rc<dyn DockCapabilityPolicyImpl>>{get:|this:&DockBaseReference|this.model.dock_capability_policy(),try_set:|this:&DockBaseReference,value:Option<Rc<dyn DockCapabilityPolicyImpl>>|{this.model.set_dock_capability_policy(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CanGoBack:bool{try_get:|this:&DockBaseReference|Ok::<bool,ferroui_base::metadata::MarkupInvokeError>(this.model.can_go_back())} [IgnoreDataMember],
        CanGoForward:bool{try_get:|this:&DockBaseReference|Ok::<bool,ferroui_base::metadata::MarkupInvokeError>(this.model.can_go_forward())} [IgnoreDataMember],
        GoBack:Rc<dyn ICommand>{try_get:|this:&DockBaseReference|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.model.go_back())} [IgnoreDataMember],
        GoForward:Rc<dyn ICommand>{try_get:|this:&DockBaseReference|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.model.go_forward())} [IgnoreDataMember],
        Navigate:Rc<dyn ICommand>{try_get:|this:&DockBaseReference|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.model.navigate())} [IgnoreDataMember],
        Close:Rc<dyn ICommand>{try_get:|this:&DockBaseReference|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.model.close())} [IgnoreDataMember],
        EnableGlobalDocking:bool{get:|this:&DockBaseReference|this.model.enable_global_docking(),try_set:|this:&DockBaseReference,value:bool|{this.model.set_enable_global_docking(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],});
pub(crate) fn register() {
    ValueTypes::register_cast::<DockBaseReference, super::ReactiveBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<DockBaseReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<DockBaseReference, Rc<dyn IDockable>>(|value| value.model());

    MarkupType::register(<DockBaseReference as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockBaseReference>();
    ValueTypes::register_cast::<DockBaseReference, DockableBaseReference>(|value| value.base());
    ValueTypes::register_cast::<DockBaseReference, Rc<dyn IDock>>(|value| value.model());
}
