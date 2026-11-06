//! The receiver of a generated metadata call.
//!
//! Instance members take `&This`. Shared handles can therefore borrow an
//! exact argument for the duration of the call: the argument slice already
//! owns the handle. A converted argument still has its own typed handle.
//! Other receiver types keep the original `Clone` operation, including a
//! user-defined clone that runs code or changes the copied value.

use super::{MarkupArguments, MarkupInvokeError};
use crate::collections::{FerroDictionary, FerroList};
use crate::media::MediaCollection;
use crate::{ObjectType, Ref};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::rc::Rc;

/// Selects receiver handling at a metadata declaration, where the concrete
/// instance type is known. Shared handles use the first autoderef match;
/// every other type uses the fallback.
#[doc(hidden)]
pub struct MarkupReceiverProbe<This>(PhantomData<fn() -> This>);

impl<This> MarkupReceiverProbe<This> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// A generated call borrows these framework-owned shared carriers. This is
/// deliberately not a blanket `Clone` implementation: arbitrary receiver
/// types can give cloning observable behavior.
#[doc(hidden)]
pub trait SharedMarkupReceiver<This: Clone + 'static> {
    #[inline]
    fn markup_receiver<'a>(&self, arguments: &mut MarkupArguments<'a>) -> Result<Cow<'a, This>, MarkupInvokeError> {
        arguments.shared_receiver()
    }
}

impl<T: ObjectType> SharedMarkupReceiver<Ref<T>> for &MarkupReceiverProbe<Ref<T>> {}
impl<T: ?Sized + 'static> SharedMarkupReceiver<Rc<T>> for &MarkupReceiverProbe<Rc<T>> {}
impl<T: 'static> SharedMarkupReceiver<FerroList<T>> for &MarkupReceiverProbe<FerroList<T>> {}
impl<K: 'static, V: 'static> SharedMarkupReceiver<FerroDictionary<K, V>> for &MarkupReceiverProbe<FerroDictionary<K, V>> {}
impl<T: Clone + 'static> SharedMarkupReceiver<MediaCollection<T>> for &MarkupReceiverProbe<MediaCollection<T>> {}

/// Uses the original owned receiver for value types and user-defined
/// carriers. Returning `This` directly keeps their original conversion
/// result representation and cleanup path; invokers borrow it explicitly.
#[doc(hidden)]
pub trait OwnedMarkupReceiverFallback<This: Clone + 'static> {
    #[inline(always)]
    fn markup_receiver(&self, arguments: &mut MarkupArguments<'_>) -> Result<This, MarkupInvokeError> {
        arguments.next()
    }
}

impl<This: Clone + 'static> OwnedMarkupReceiverFallback<This> for MarkupReceiverProbe<This> {}
