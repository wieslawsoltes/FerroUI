//! Helpers for tests of styling, for this crate and the crates built on it
//! (the port of the reference test suite's `StyleHelpers`).

use crate::property_store::FrameType;
use crate::styling::{SelectorMatchResult, Style, StyleHostRef};
use crate::{ObjectType, Ref, StyledElement, Upcast};

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
