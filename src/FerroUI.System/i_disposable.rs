//! `System.IDisposable`.

/// A resource or subscription that is released explicitly.
///
/// Unlike `Drop`, disposal is an explicit, idempotent operation: dropping a
/// handle without calling [`dispose`](IDisposable::dispose) keeps the
/// subscription alive, which is what callers that ignore the returned handle
/// expect.
pub trait IDisposable {
    fn dispose(&self);
}

/// Handles compare by identity (reference equality), so that the handle a
/// subscription or binding returns can be held in untyped values.
impl PartialEq for dyn IDisposable {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
