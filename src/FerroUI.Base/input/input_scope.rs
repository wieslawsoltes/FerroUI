//! Ordered, live WinUI input-scope objects using FerroUI's property and list APIs.
//! Copyright (c) Microsoft Corporation. See NOTICE.md for source attribution.

use super::{text_input::TextInputContentType, InputScopeNameValue};
use crate::collections::FerroList;
use crate::metadata::IAddChild;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::{FormatError, HandlerList};
use crate::{
    ferro_class, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StyledProperty,
};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

/// One live input-scope hint. The name remains mutable after insertion into
/// one or more input scopes; each scope publishes a fresh immutable snapshot.
#[repr(C)]
pub struct InputScopeName {
    base: FerroObject,
}
ferro_class!(InputScopeName: FerroObject);
crate::ferro_impl_classes!(InputScopeName: FerroObjectImpl);
crate::ferro_class_info!(InputScopeName {
    new: InputScopeName::new,
    markup: {
        content: NameValue,
        constructors: [(InputScopeNameValue) => InputScopeName::with_name_value],
    },
});
crate::ferro_properties! { impl InputScopeName {
    pub fn name_value_property() -> StyledProperty<InputScopeNameValue> {
        FerroProperty::register::<InputScopeName, _>("NameValue", InputScopeNameValue::Default)
    }
} }

impl InputScopeName {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn with_name_value(name_value: InputScopeNameValue) -> Ref<Self> {
        let result = Self::new();
        result.set_name_value(name_value);
        result
    }
    pub fn parse(value: &str) -> Result<Ref<Self>, FormatError> {
        InputScopeNameValue::parse(value).map(Self::with_name_value)
    }
    pub fn name_value(&self) -> InputScopeNameValue {
        self.get_value(Self::name_value_property())
    }
    pub fn set_name_value(&self, value: InputScopeNameValue) {
        self.set_value(Self::name_value_property(), value);
    }
}

/// An ordered, shared collection of input-method hints. A newly constructed
/// scope has no names, as in WinUI; that falls back to the host's normal policy.
#[repr(C)]
pub struct InputScope {
    base: FerroObject,
    names: FerroList<Ref<InputScopeName>>,
    collection_subscription: Cell<Option<u64>>,
    item_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    changed: Rc<HandlerList<dyn Fn()>>,
}
ferro_class!(InputScope: FerroObject);
crate::ferro_class_info!(InputScope {
    new: InputScope::new,
    interfaces: [Rc<dyn IAddChild<Ref<InputScopeName>>> => InputScope::as_add_child],
    markup: {
        content: Names,
        parse: InputScope::parse,
        properties: [Names: FerroList<Ref<InputScopeName>> { get: InputScope::names }],
    },
});

impl FerroObjectImpl for InputScope {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let token = this.names.add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.refresh_item_subscriptions();
                this.invalidate();
            }
        }));
        this.collection_subscription.set(Some(token));
    }
}

impl InputScope {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            names: FerroList::new(),
            collection_subscription: Cell::new(None),
            item_subscriptions: RefCell::new(Vec::new()),
            changed: Rc::new(HandlerList::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn with_name_value(value: InputScopeNameValue) -> Ref<Self> {
        let result = Self::new();
        result.names.add(InputScopeName::with_name_value(value));
        result
    }
    /// Converts one name, matching WinUI CInputScope::Create. The legacy
    /// FerroUI shortcut names remain accepted on this object converter.
    pub fn parse(value: &str) -> Result<Ref<Self>, FormatError> {
        let legacy = [
            ("Normal", InputScopeNameValue::Default),
            ("Alpha", InputScopeNameValue::AlphanumericHalfWidth),
            ("Email", InputScopeNameValue::EmailSmtpAddress),
            ("Name", InputScopeNameValue::PersonalFullName),
            ("Pin", InputScopeNameValue::NumericPin),
            ("Social", InputScopeNameValue::Chat),
        ]
        .into_iter()
        .find_map(|(name, scope)| name.eq_ignore_ascii_case(value.trim()).then_some(scope));
        legacy
            .map(Ok)
            .unwrap_or_else(|| InputScopeNameValue::parse(value))
            .map(Self::with_name_value)
    }
    pub fn names(&self) -> FerroList<Ref<InputScopeName>> {
        self.names.clone()
    }
    /// Captures values, never live objects, across the platform boundary.
    pub fn snapshot(&self) -> Rc<[InputScopeNameValue]> {
        self.names
            .snapshot()
            .iter()
            .map(|name| name.name_value())
            .collect()
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.changed);
        Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.remove(token);
            }
        })
    }
    pub fn as_add_child(this: Ref<Self>) -> Rc<dyn IAddChild<Ref<InputScopeName>>> {
        Rc::new(InputScopeAddChild(this))
    }
    fn refresh_item_subscriptions(&self) {
        let old = std::mem::take(&mut *self.item_subscriptions.borrow_mut());
        for subscription in old {
            subscription.dispose();
        }
        let mut subscriptions = Vec::new();
        let mut seen = HashSet::new();
        for name in self.names.snapshot().iter() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let weak = self.to_ref().downgrade();
            subscriptions.push(name.property_changed(move |change| {
                if change.property() == InputScopeName::name_value_property().as_property() {
                    if let Some(this) = weak.upgrade() {
                        this.invalidate();
                    }
                }
            }));
        }
        *self.item_subscriptions.borrow_mut() = subscriptions;
    }
    fn invalidate(&self) {
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }
}

impl Drop for InputScope {
    fn drop(&mut self) {
        if let Some(token) = self.collection_subscription.take() {
            self.names.remove_collection_changed(token);
        }
        for subscription in self.item_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

// Preserve the former Rust convenience setter while exposing the native object
// property and full collection. Callers can also pass a Ref<InputScope> or None.
impl From<TextInputContentType> for Option<Ref<InputScope>> {
    fn from(value: TextInputContentType) -> Self {
        Some(InputScope::with_name_value(value.into()))
    }
}
impl From<InputScopeNameValue> for Option<Ref<InputScope>> {
    fn from(value: InputScopeNameValue) -> Self {
        Some(InputScope::with_name_value(value))
    }
}

struct InputScopeAddChild(Ref<InputScope>);
impl IAddChild<Ref<InputScopeName>> for InputScopeAddChild {
    fn add_child(&self, value: Ref<InputScopeName>) {
        self.0.names.add(value);
    }
    fn reference_id(&self) -> usize {
        &*self.0 as *const InputScope as usize
    }
}
