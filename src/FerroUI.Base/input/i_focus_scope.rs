/// Marks an input element as a focus scope: an element that remembers which
/// of its descendants was focused last.
///
/// Whether an element is a focus scope is answered at runtime by the
/// `is_focus_scope` virtual member of `InputElement`; a class that is a
/// focus scope overrides it to return true and implements this trait for
/// documentation.
pub trait IFocusScope {}
