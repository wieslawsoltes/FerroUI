//! Dynamic interface queries (`value is IFoo<TBackendEmitter, TEmitResult> foo`).
//!
//! AST nodes, property setters and wrapped methods are not generic over the emitter backend,
//! but a backend (or a host framework) may define nodes that know how to emit themselves for a
//! particular backend. Such an object overrides `query_interface` and answers requests for the
//! `Rc<dyn Interface>` handles it supports:
//!
//! ```text
//! fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
//!     xaml_query_interface!(self, slot, dyn IXamlAstEmitableNode<MyEmitter, MyResult>)
//! }
//! ```

use std::any::Any;
use std::rc::Rc;

use crate::ast::{
    IXamlAstClrPropertyExtension, IXamlAstNode, IXamlPropertySetter, IXamlWrappedMethod,
};

/// Answers a `query_interface` request for each listed interface type.
#[macro_export]
macro_rules! xaml_query_interface {
    ($this:expr, $slot:expr, $($iface:ty),+ $(,)?) => {{
        let this = $this;
        let slot: &mut dyn ::std::any::Any = $slot;
        $(
            if let Some(typed) = slot.downcast_mut::<Option<::std::rc::Rc<$iface>>>() {
                let handle: ::std::rc::Rc<$iface> = this;
                *typed = Some(handle);
                return true;
            }
        )+
        let _ = this;
        false
    }};
}

/// `node as T` for an interface `T` unknown to this crate.
pub fn query_node_interface<T: ?Sized + 'static>(node: &Rc<dyn IXamlAstNode>) -> Option<Rc<T>> {
    let mut slot: Option<Rc<T>> = None;
    let slot_any: &mut dyn Any = &mut slot;
    if node.clone().query_interface(slot_any) {
        slot
    } else {
        None
    }
}

/// `setter as T` for an interface `T` unknown to this crate.
pub fn query_property_setter_interface<T: ?Sized + 'static>(
    setter: &Rc<dyn IXamlPropertySetter>,
) -> Option<Rc<T>> {
    let mut slot: Option<Rc<T>> = None;
    let slot_any: &mut dyn Any = &mut slot;
    if setter.clone().query_interface(slot_any) {
        slot
    } else {
        None
    }
}

/// `method as T` for an interface `T` unknown to this crate.
pub fn query_wrapped_method_interface<T: ?Sized + 'static>(
    method: &Rc<dyn IXamlWrappedMethod>,
) -> Option<Rc<T>> {
    let mut slot: Option<Rc<T>> = None;
    let slot_any: &mut dyn Any = &mut slot;
    if method.clone().query_interface(slot_any) {
        slot
    } else {
        None
    }
}

/// `property as T` for an interface `T` implemented by a host class derived from
/// `XamlAstClrProperty` (see [`IXamlAstClrPropertyExtension`]).
pub fn query_clr_property_extension_interface<T: ?Sized + 'static>(
    extension: &Rc<dyn IXamlAstClrPropertyExtension>,
) -> Option<Rc<T>> {
    let mut slot: Option<Rc<T>> = None;
    let slot_any: &mut dyn Any = &mut slot;
    if extension.clone().query_interface(slot_any) {
        slot
    } else {
        None
    }
}
