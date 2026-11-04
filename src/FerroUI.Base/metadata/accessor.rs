//! Shared typed adapters for generated property invokers.
//!
//! A metadata member still owns its public `MarkupInvoke` function pointer.
//! That small entry passes a typed callback to the adapter for its receiver
//! and value types. The callback is ordinary data, not another generic type,
//! so different members share argument handling and receiver cleanup. Getter
//! callbacks keep the complete result-conversion expression: a temporary
//! callable may have a destructor that must run after boxing its result.
//! This introduces an indirect typed call;
//! it does not erase a payload, allocate an invocation frame or change the
//! typed functions used by generated native markup.
//!
//! Keep owned and shared receivers separate. In particular, an arbitrary
//! receiver's `Clone` must still run and its temporary must retain its original
//! type and lifetime. The existing receiver traits remain unchanged.

use super::receiver::{MarkupReceiverProbe, OwnedMarkupReceiverFallback, SharedMarkupReceiver};
use super::{MarkupArguments, MarkupInvokeError, MarkupValue};

/// Property invokers whose exact receiver is a shared framework handle.
#[doc(hidden)]
pub trait SharedMarkupAccessor<This: Clone + 'static>: SharedMarkupReceiver<This> {
    #[inline(never)]
    fn invoke_getter(
        &self,
        arguments: &[MarkupValue],
        get: fn(&This) -> Result<MarkupValue, MarkupInvokeError>,
    ) -> Result<MarkupValue, MarkupInvokeError> {
        let mut arguments = MarkupArguments::new(arguments, 1)?;
        let this = SharedMarkupReceiver::markup_receiver(self, &mut arguments)?;
        // The receiver remains alive through return conversion, including a
        // conversion callback or a panic while boxing the result.
        get(&this)
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
    fn invoke_getter(
        &self,
        arguments: &[MarkupValue],
        get: fn(&This) -> Result<MarkupValue, MarkupInvokeError>,
    ) -> Result<MarkupValue, MarkupInvokeError> {
        let mut arguments = MarkupArguments::new(arguments, 1)?;
        let this = OwnedMarkupReceiverFallback::markup_receiver(self, &mut arguments)?;
        get(&this)
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
