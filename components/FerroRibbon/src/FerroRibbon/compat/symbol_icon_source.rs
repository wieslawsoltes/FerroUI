// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
//! Symbol input properties for the ribbon's snapshot factories; SDK live factory remains absent.
use super::{IconSource, Symbol};
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledProperty, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
#[repr(C)]
pub struct SymbolIconSource {
    base: IconSource,
}
ferro_class!(SymbolIconSource:IconSource);
ferro_impl_classes!(SymbolIconSource:FerroObjectImpl);
ferro_class_info!(SymbolIconSource {new:SymbolIconSource::new,markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_properties! {impl SymbolIconSource {pub fn symbol_property()->StyledProperty<Symbol> {FerroProperty::register::<Self,_>("Symbol",Symbol::Emoji)}}}
impl SymbolIconSource {
    pub fn construct() -> Self {
        Self {
            base: IconSource::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn symbol(&self) -> Symbol {
        self.get_value(Self::symbol_property())
    }
    pub fn set_symbol(&self, value: Symbol) {
        self.set_value(Self::symbol_property(), value)
    }
}
