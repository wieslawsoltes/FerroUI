// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDockCapabilityPolicy;
use std::rc::Rc;
/// Native descriptor carrier retaining the original IDockCapabilityPolicy value.
#[doc(hidden)]
pub trait IDockCapabilityPolicyMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockCapabilityPolicyMetadata as "IDockCapabilityPolicy"{
    namespace:"FerroDock.Model.Core",this:Rc<dyn IDockCapabilityPolicy>,handles:[Rc<dyn IDockCapabilityPolicy>,Option<Rc<dyn IDockCapabilityPolicy>>],
    properties:[
        CanClose:Option<bool>{get:|this:&Rc<dyn IDockCapabilityPolicy>|this.can_close(),set:|this:&Rc<dyn IDockCapabilityPolicy>,value|this.set_can_close(value)},
        CanPin:Option<bool>{get:|this:&Rc<dyn IDockCapabilityPolicy>|this.can_pin(),set:|this:&Rc<dyn IDockCapabilityPolicy>,value|this.set_can_pin(value)},
        CanFloat:Option<bool>{get:|this:&Rc<dyn IDockCapabilityPolicy>|this.can_float(),set:|this:&Rc<dyn IDockCapabilityPolicy>,value|this.set_can_float(value)},
        CanDrag:Option<bool>{get:|this:&Rc<dyn IDockCapabilityPolicy>|this.can_drag(),set:|this:&Rc<dyn IDockCapabilityPolicy>,value|this.set_can_drag(value)},
        CanDrop:Option<bool>{get:|this:&Rc<dyn IDockCapabilityPolicy>|this.can_drop(),set:|this:&Rc<dyn IDockCapabilityPolicy>,value|this.set_can_drop(value)},
        CanDockAsDocument:Option<bool>{get:|this:&Rc<dyn IDockCapabilityPolicy>|this.can_dock_as_document(),set:|this:&Rc<dyn IDockCapabilityPolicy>,value|this.set_can_dock_as_document(value)},
    ],

});
