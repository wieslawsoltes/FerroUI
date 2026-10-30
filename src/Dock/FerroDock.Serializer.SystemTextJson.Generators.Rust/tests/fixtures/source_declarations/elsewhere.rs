//! #[path] module with an inherited, unevaluated target condition.
#[cfg_attr(feature="inner",repr(C))]
pub struct Moved;
pub trait IExternal: crate::IRoot {}
