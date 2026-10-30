// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::{
    DockCapability, DockCapabilityPolicy, DockCapabilityPolicyImpl, IDockCapabilityPolicy,
};
use std::rc::Rc;
/// Native descriptor carrier retaining the original DockCapabilityPolicy value.
#[doc(hidden)]
pub trait DockCapabilityPolicyMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockCapabilityPolicyMetadata as "DockCapabilityPolicy"{
    namespace:"FerroDock.Model.Core",this:Rc<dyn DockCapabilityPolicyImpl>,handles:[Rc<DockCapabilityPolicy>,Option<Rc<DockCapabilityPolicy>>,Rc<dyn DockCapabilityPolicyImpl>,Option<Rc<dyn DockCapabilityPolicyImpl>>],
    interfaces:[Rc<dyn IDockCapabilityPolicy>],
    constructors:[()=>(|| DockCapabilityPolicy::new() as Rc<dyn DockCapabilityPolicyImpl>)],
    properties:[
        CanClose:Option<bool>{get:|this:&Rc<dyn DockCapabilityPolicyImpl>|this.can_close(),set:|this:&Rc<dyn DockCapabilityPolicyImpl>,value|this.set_can_close(value)},
        CanPin:Option<bool>{get:|this:&Rc<dyn DockCapabilityPolicyImpl>|this.can_pin(),set:|this:&Rc<dyn DockCapabilityPolicyImpl>,value|this.set_can_pin(value)},
        CanFloat:Option<bool>{get:|this:&Rc<dyn DockCapabilityPolicyImpl>|this.can_float(),set:|this:&Rc<dyn DockCapabilityPolicyImpl>,value|this.set_can_float(value)},
        CanDrag:Option<bool>{get:|this:&Rc<dyn DockCapabilityPolicyImpl>|this.can_drag(),set:|this:&Rc<dyn DockCapabilityPolicyImpl>,value|this.set_can_drag(value)},
        CanDrop:Option<bool>{get:|this:&Rc<dyn DockCapabilityPolicyImpl>|this.can_drop(),set:|this:&Rc<dyn DockCapabilityPolicyImpl>,value|this.set_can_drop(value)},
        CanDockAsDocument:Option<bool>{get:|this:&Rc<dyn DockCapabilityPolicyImpl>|this.can_dock_as_document(),set:|this:&Rc<dyn DockCapabilityPolicyImpl>,value|this.set_can_dock_as_document(value)},
    ],
    methods:[fn Get(DockCapability)->Option<bool> => (|this:&Rc<dyn DockCapabilityPolicyImpl>,value|this.get(value))],
});
