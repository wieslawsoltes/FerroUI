//! Shared typed adapters for generated property invokers.
//!
//! A metadata member still owns its public `MarkupInvoke` function pointer.
//! Getters share receiver preparation, then keep that receiver and the entire
//! original result expression in the member's own thunk. A temporary callable
//! in a tail expression can outlive local variables; placing it in a separate
//! callback would change its cleanup order relative to an owned receiver.
//!
//! Setters pass a typed callback containing the original complete statement.
//! That callback is ordinary data, not another generic type, so members share
//! argument handling and receiver cleanup. Static getters likewise share their
//! arity check and keep the complete result expression in a callback. These
//! callback paths introduce an indirect typed call; getter receiver preparation
//! does not. None erases a payload, allocates an invocation frame or changes the
//! typed functions used by generated native markup.
//!
//! Keep owned and shared receivers separate. In particular, an arbitrary
//! receiver's `Clone` must still run and its temporary must retain its original
//! type and lifetime. The existing receiver traits remain unchanged.

use super::receiver::{MarkupReceiverProbe, OwnedMarkupReceiverFallback, SharedMarkupReceiver};
use super::{MarkupArguments, MarkupInvokeError, MarkupValue};
use std::borrow::Cow;

/// Property invokers whose exact receiver is a shared framework handle.
#[doc(hidden)]
pub trait SharedMarkupAccessor<This: Clone + 'static>: SharedMarkupReceiver<This> {
    #[inline(never)]
    fn prepare_getter<'a>(
        &self,
        arguments: &'a [MarkupValue],
    ) -> Result<Cow<'a, This>, MarkupInvokeError> {
        let mut arguments = MarkupArguments::new(arguments, 1)?;
        SharedMarkupReceiver::markup_receiver(self, &mut arguments)
    }

    #[inline(never)]
    fn invoke_setter<Value: Clone + 'static>(
        &self,
        arguments: &[MarkupValue],
        set: fn(&This, Value),
    ) -> Result<MarkupValue, MarkupInvokeError> {
        let mut arguments = MarkupArguments::new(arguments, 2)?;
        let this = SharedMarkupReceiver::markup_receiver(self, &mut arguments)?;
        let value = arguments.next::<Value>()?;
        set(&this, value);
        Ok(None)
    }

    #[inline(never)]
    fn invoke_try_setter<Value: Clone + 'static>(
        &self,
        arguments: &[MarkupValue],
        set: fn(&This, Value) -> Result<(), MarkupInvokeError>,
    ) -> Result<MarkupValue, MarkupInvokeError> {
        let mut arguments = MarkupArguments::new(arguments, 2)?;
        let this = SharedMarkupReceiver::markup_receiver(self, &mut arguments)?;
        let value = arguments.next::<Value>()?;
        set(&this, value)?;
        Ok(None)
    }
}

impl<This, Probe> SharedMarkupAccessor<This> for Probe
where
    This: Clone + 'static,
    Probe: SharedMarkupReceiver<This>,
{
}

/// Property invokers that retain the original owned receiver conversion.
#[doc(hidden)]
pub trait OwnedMarkupAccessorFallback<This: Clone + 'static>: OwnedMarkupReceiverFallback<This> {
    #[inline(never)]
    fn prepare_getter(
        &self,
        arguments: &[MarkupValue],
    ) -> Result<This, MarkupInvokeError> {
        let mut arguments = MarkupArguments::new(arguments, 1)?;
        OwnedMarkupReceiverFallback::markup_receiver(self, &mut arguments)
    }

    #[inline(never)]
    fn invoke_setter<Value: Clone + 'static>(
        &self,
        arguments: &[MarkupValue],
        set: fn(&This, Value),
    ) -> Result<MarkupValue, MarkupInvokeError> {
        let mut arguments = MarkupArguments::new(arguments, 2)?;
        let this = OwnedMarkupReceiverFallback::markup_receiver(self, &mut arguments)?;
        let value = arguments.next::<Value>()?;
        set(&this, value);
        Ok(None)
    }

    #[inline(never)]
    fn invoke_try_setter<Value: Clone + 'static>(
        &self,
        arguments: &[MarkupValue],
        set: fn(&This, Value) -> Result<(), MarkupInvokeError>,
    ) -> Result<MarkupValue, MarkupInvokeError> {
        let mut arguments = MarkupArguments::new(arguments, 2)?;
        let this = OwnedMarkupReceiverFallback::markup_receiver(self, &mut arguments)?;
        let value = arguments.next::<Value>()?;
        set(&this, value)?;
        Ok(None)
    }
}

impl<This: Clone + 'static> OwnedMarkupAccessorFallback<This> for MarkupReceiverProbe<This> {}

/// A static getter retains its complete result expression in the callback.
#[doc(hidden)]
#[inline(never)]
pub fn invoke_static_getter(
    arguments: &[MarkupValue],
    get: fn() -> Result<MarkupValue, MarkupInvokeError>,
) -> Result<MarkupValue, MarkupInvokeError> {
    MarkupArguments::new(arguments, 0)?;
    get()
}
