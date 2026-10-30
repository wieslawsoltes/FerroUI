// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! A carrier whose name prefixes the actual constructor result's name.
use ferrodock_model::core::{DockCapabilityOverrides, IDockCapabilityPolicy};
use std::rc::Rc;

/// The application's plain data object, retaining the original engine policy.
pub struct CarrierValue {
    model: Rc<DockCapabilityOverrides>,
}
impl CarrierValue {
    /// Constructs the actual engine state, without a native control or copied policy.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            model: DockCapabilityOverrides::new(),
        })
    }
    /// Gets the original mutable engine object.
    pub fn model(&self) -> Rc<DockCapabilityOverrides> {
        self.model.clone()
    }
}
impl PartialEq for CarrierValue {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
/// A stateless declaration carrier; it never represents the constructed instance.
pub struct Carrier;
ferroui_base::ferro_markup_type!(class Carrier {
    namespace: "FerroDock.Markup.Tests.Carriers",
    this: Rc<CarrierValue>, handles: [Rc<CarrierValue>, Option<Rc<CarrierValue>>],
    constructors: [() => CarrierValue::new],
    properties: [CanClose: Option<bool> {
        get: |this: &Rc<CarrierValue>| this.model.can_close(),
        set: |this: &Rc<CarrierValue>, value| this.model.set_can_close(value)
    }],
});

/// Consumer-only fixture for assigning original engine objects through a foreign interface.
pub struct ContractConsumer {
    policy: std::cell::RefCell<Rc<dyn IDockCapabilityPolicy>>,
    optional: std::cell::RefCell<Option<Rc<dyn IDockCapabilityPolicy>>>,
    native: std::cell::RefCell<Rc<dyn NativeContract>>,
}
impl ContractConsumer {
    /// Creates a consumer retaining the actual source policies.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            policy: std::cell::RefCell::new(DockCapabilityOverrides::new()),
            optional: std::cell::RefCell::new(None),
            native: std::cell::RefCell::new(MultiHandleValue::new()),
        })
    }
    /// Gets the original required source policy.
    pub fn policy(&self) -> Rc<dyn IDockCapabilityPolicy> {
        self.policy.borrow().clone()
    }
    /// Gets the actual native trait value from the compiled assignment.
    pub fn native(&self) -> Rc<dyn NativeContract> {
        self.native.borrow().clone()
    }
    /// Gets the original nullable source policy.
    pub fn optional(&self) -> Option<Rc<dyn IDockCapabilityPolicy>> {
        self.optional.borrow().clone()
    }
}
impl PartialEq for ContractConsumer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class ContractConsumer {
 namespace:"FerroDock.Markup.Tests.Carriers",this:Rc<ContractConsumer>,handles:[Rc<ContractConsumer>,Option<Rc<ContractConsumer>>],
 constructors:[()=>ContractConsumer::new],
 properties:[
  Policy:Rc<dyn IDockCapabilityPolicy>{get:|this:&Rc<ContractConsumer>|this.policy(),set:|this:&Rc<ContractConsumer>,value|*this.policy.borrow_mut()=value},
  Native:Rc<dyn NativeContract>{get:|this:&Rc<ContractConsumer>|this.native(),set:|this:&Rc<ContractConsumer>,value|*this.native.borrow_mut()=value},
  Optional:Option<Rc<dyn IDockCapabilityPolicy>>{get:|this:&Rc<ContractConsumer>|this.optional(),set:|this:&Rc<ContractConsumer>,value|*this.optional.borrow_mut()=value},
 ],
});

/// Native declaring trait, distinct from an external interface descriptor carrier.
pub trait NativeContract: 'static {
    /// Gets the same original source engine policy.
    fn source_policy(&self) -> Rc<dyn IDockCapabilityPolicy>;
}
impl PartialEq for dyn NativeContract {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
ferroui_base::ferro_markup_type!(interface dyn NativeContract as "NativeContract" {
 namespace:"FerroDock.Markup.Tests.Carriers",this:Rc<dyn NativeContract>,handles:[Rc<dyn NativeContract>,Option<Rc<dyn NativeContract>>],
 properties:[Policy:Rc<dyn IDockCapabilityPolicy>{get:|this:&Rc<dyn NativeContract>|this.source_policy()}],
});
/// Consumer-only example of the guide's plain-value/shared-handle declaration.
#[derive(Clone)]
pub struct MultiHandleValue {
    policy: Rc<DockCapabilityOverrides>,
}
impl MultiHandleValue {
    /// Creates a fixture containing the genuine original policy object.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            policy: DockCapabilityOverrides::new(),
        })
    }
}
impl PartialEq for MultiHandleValue {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl NativeContract for MultiHandleValue {
    fn source_policy(&self) -> Rc<dyn IDockCapabilityPolicy> {
        self.policy.clone()
    }
}
ferroui_base::ferro_markup_type!(class MultiHandleValue {
 namespace:"FerroDock.Markup.Tests.Carriers",this:Rc<MultiHandleValue>,handles:[MultiHandleValue,Rc<MultiHandleValue>,Option<Rc<MultiHandleValue>>],interfaces:[Rc<dyn NativeContract>],
 constructors:[()=>MultiHandleValue::new],
 properties:[CanClose:Option<bool>{get:|this:&Rc<MultiHandleValue>|this.policy.can_close(),set:|this:&Rc<MultiHandleValue>,value|this.policy.set_can_close(value)}],
});
