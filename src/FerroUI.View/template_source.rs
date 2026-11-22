//! Source evidence from the element returned by a data template's actual item factory.
use crate::{builder::TypedProperty, element::PropValue, Element, RsxSourceInfo};
use ferroui_controls::templates::RustDataTemplateSource;
use std::{cell::Cell, panic::Location};

pub(crate) struct EditorSource {
    constructor: &'static Location<'static>,
    compiled_constructor: Option<RsxSourceInfo>,
    body: Cell<RenderedBody>,
}

#[derive(Clone, Copy)]
enum RenderedBody {
    Unseen,
    Invocation(RsxSourceInfo),
    Ambiguous,
}

impl EditorSource {
    pub(super) fn new(
        constructor: &'static Location<'static>,
        compiled_constructor: Option<RsxSourceInfo>,
    ) -> Self {
        Self {
            constructor,
            compiled_constructor,
            body: Cell::new(RenderedBody::Unseen),
        }
    }

    pub(super) fn observe(&self, element: &Element) {
        let next = match (self.body.get(), element_source(element)) {
            (RenderedBody::Unseen, Some(source)) => RenderedBody::Invocation(source),
            (RenderedBody::Invocation(old), Some(source)) if same_invocation(old, source) => {
                RenderedBody::Invocation(source)
            }
            _ => RenderedBody::Ambiguous,
        };
        self.body.set(next);
    }

    pub(super) fn provenance(&self) -> RustDataTemplateSource {
        RustDataTemplateSource {
            constructor: self.constructor,
            compiled_constructor: self.compiled_constructor,
            rendered_body: match self.body.get() {
                RenderedBody::Invocation(source) => Some(source),
                RenderedBody::Unseen | RenderedBody::Ambiguous => None,
            },
        }
    }
}

fn element_source(element: &Element) -> Option<RsxSourceInfo> {
    match element {
        Element::Control(control) => {
            let property = RsxSourceInfo::location_property().untyped();
            let entry = control
                .props
                .iter()
                .rfind(|entry| std::ptr::eq(entry.property, property))?;
            let PropValue::Value(value) = &entry.value else {
                return None;
            };
            *value.downcast_ref::<Option<RsxSourceInfo>>()?
        }
        Element::Fragment(parts) | Element::List(parts) => {
            let mut parts = parts.iter();
            let first = element_source(parts.next()?)?;
            parts
                .all(|element| {
                    element_source(element).is_some_and(|source| same_invocation(first, source))
                })
                .then_some(first)
        }
        _ => None,
    }
}

fn same_invocation(left: RsxSourceInfo, right: RsxSourceInfo) -> bool {
    left.file == right.file
        && left.manifest == right.manifest
        && left.compiled.invocation == right.compiled.invocation
        && left.compiled.revision == right.compiled.revision
}
