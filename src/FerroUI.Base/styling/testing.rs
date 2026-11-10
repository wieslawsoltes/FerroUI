//! Helpers for tests of styling, for this crate and the crates built on it
//! (the port of the reference test suite's `StyleHelpers`), and the members
//! of styling the benchmarks of the framework reach (upstream's benchmark
//! project reaches them through its access to the internals).

use crate::property_store::FrameType;
use crate::styling::{Selector, SelectorMatchResult, Style, StyleHostRef};
use crate::{FerroObject, ObjectType, Ref, StyledElement, Upcast};

/// Attaches a style to an element outside of a styling pass, with the
/// element itself as the style host unless one is given
/// (`StyleHelpers.TryAttach`).
pub fn try_attach<T: ObjectType + Upcast<StyledElement>>(
    style: &Style,
    element: &Ref<T>,
    host: Option<StyleHostRef>,
) -> SelectorMatchResult {
    let element: Ref<StyledElement> = element.clone().upcast();
    let host = host.unwrap_or_else(|| StyleHostRef::Element(element.clone()));
    style.try_attach(&element, Some(&host), FrameType::Style)
}

/// The behaviour of a kind of selector (the abstract members of `Selector`),
/// for a selector class declared outside this crate.
pub use super::selector::SelectorNode;

/// A selector of a kind declared outside this crate.
pub fn selector(node: impl SelectorNode) -> Selector {
    Selector::new(node)
}

/// `target.GetValueStore().BeginStyling()`: starts a styling pass over the
/// values of an object; pending reevaluations are flushed when the outermost
/// pass ends.
pub fn begin_styling(target: &FerroObject) {
    target.values().begin_styling();
}

/// `target.GetValueStore().EndStyling()`.
pub fn end_styling(target: &FerroObject) {
    target.values().end_styling(target);
}
