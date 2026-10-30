// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::{DockCapabilityOverrides, DockCapabilityPolicyImpl};
use std::rc::Rc;
/// Native descriptor carrier retaining the original DockCapabilityOverrides value.
#[doc(hidden)]
pub trait DockCapabilityOverridesMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockCapabilityOverridesMetadata as "DockCapabilityOverrides"{
    namespace:"FerroDock.Model.Core",this:Rc<DockCapabilityOverrides>,handles:[Rc<DockCapabilityOverrides>,Option<Rc<DockCapabilityOverrides>>],
    base:Rc<dyn DockCapabilityPolicyImpl>,
    constructors:[()=>DockCapabilityOverrides::new],
    properties:[HasAnyOverride:bool{get:|this:&Rc<DockCapabilityOverrides>|this.has_any_override()}],
});
