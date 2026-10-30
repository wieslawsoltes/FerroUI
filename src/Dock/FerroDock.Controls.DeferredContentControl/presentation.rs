// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::DeferredContentPresentationTimeline;
use std::rc::Rc;
/// Allows content instances to opt out of deferred presentation.
pub trait IDeferredContentPresentation {
    /// Whether content presentation should be deferred.
    fn defer_content_presentation(&self) -> bool;
}
pub(crate) trait IDeferredContentPresentationTarget {
    fn enqueued_timeline(&self) -> Option<Rc<DeferredContentPresentationTimeline>>;
    fn set_enqueued_timeline(&self, timeline: Option<Rc<DeferredContentPresentationTimeline>>);
    fn retain_pending_presentation_on_failure(&self) -> bool;
    fn apply_deferred_presentation(&self) -> bool;
}
impl PartialEq for dyn IDeferredContentPresentation {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
pub(crate) fn should_defer(content: Option<&ferroui_base::BoxedValue>) -> bool {
    let Some(content) = content else {
        return true;
    };
    if let Some(content) = content.downcast_ref::<Rc<dyn IDeferredContentPresentation>>() {
        return content.defer_content_presentation();
    }
    use ferroui_base::data::core::{ValueType, ValueTypes};
    ValueTypes::try_cast(
        content,
        ValueType::of::<Rc<dyn IDeferredContentPresentation>>(),
    )
    .and_then(|value| {
        value
            .downcast_ref::<Rc<dyn IDeferredContentPresentation>>()
            .cloned()
    })
    .is_none_or(|content| content.defer_content_presentation())
}
ferroui_base::ferro_markup_type!(interface dyn IDeferredContentPresentation as "IDeferredContentPresentation" {
    this:Rc<dyn IDeferredContentPresentation>,
    handles:[Rc<dyn IDeferredContentPresentation>,Option<Rc<dyn IDeferredContentPresentation>>],
    properties:[DeferContentPresentation:bool{get:|content:&Rc<dyn IDeferredContentPresentation>|content.defer_content_presentation()}]
});
