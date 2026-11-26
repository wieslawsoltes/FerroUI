//! The label, the event arguments and the scrolling kind of WinUI's `AnnotatedScrollBar`
//! (`controls/dev/AnnotatedScrollBar/AnnotatedScrollBar.idl`, `AnnotatedScrollBarLabel.cpp`,
//! `AnnotatedScrollBarScrollingEventArgs.cpp` and
//! `AnnotatedScrollBarDetailLabelRequestedEventArgs.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use ferroui_base::{ferro_markup_enum, ferro_markup_type, BoxedValue};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// What scrolls an [`AnnotatedScrollBar`](super::AnnotatedScrollBar) (WinUI's
/// `AnnotatedScrollBarScrollingEventKind`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AnnotatedScrollBarScrollingEventKind {
    /// A press on the rail: the view jumps to the offset under the pointer.
    #[default]
    Click = 0,
    /// A drag on the rail: the view follows the pointer.
    Drag = 1,
    /// The increment button (the top button): the view scrolls up by `SmallChange`.
    IncrementButton = 2,
    /// The decrement button (the bottom button): the view scrolls down by `SmallChange`.
    DecrementButton = 3,
}

ferro_markup_enum!(
    AnnotatedScrollBarScrollingEventKind { Click, Drag, IncrementButton, DecrementButton },
    { namespace: "FerroUI.Controls" }
);

/// A label of an [`AnnotatedScrollBar`](super::AnnotatedScrollBar) (WinUI's
/// `AnnotatedScrollBarLabel`): its content, shown by the `LabelTemplate` of the bar, and the
/// scroll offset it marks. Clones share the label; two labels are equal when they are the same
/// label.
#[derive(Clone)]
pub struct AnnotatedScrollBarLabel(Rc<LabelData>);

struct LabelData {
    content: Option<BoxedValue>,
    scroll_offset: f64,
}

impl AnnotatedScrollBarLabel {
    /// A label with `content` at `scroll_offset` (WinUI's constructor).
    pub fn new(content: Option<BoxedValue>, scroll_offset: f64) -> Self {
        Self(Rc::new(LabelData { content, scroll_offset }))
    }

    /// The content of the label (`Content`).
    pub fn content(&self) -> Option<BoxedValue> {
        self.0.content.clone()
    }

    /// The scroll offset the label marks (`ScrollOffset`).
    pub fn scroll_offset(&self) -> f64 {
        self.0.scroll_offset
    }
}

impl PartialEq for AnnotatedScrollBarLabel {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for AnnotatedScrollBarLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnnotatedScrollBarLabel")
            .field("has_content", &self.0.content.is_some())
            .field("scroll_offset", &self.0.scroll_offset)
            .finish()
    }
}

/// The arguments of [`AnnotatedScrollBar::scrolling`](super::AnnotatedScrollBar::scrolling)
/// (WinUI's `AnnotatedScrollBarScrollingEventArgs`): the offset the bar is about to request and
/// what scrolls; a handler cancels the request with [`set_cancel`](Self::set_cancel). The clones
/// given to the handlers of markup share the cancellation.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotatedScrollBarScrollingEventArgs {
    scroll_offset: f64,
    scrolling_event_kind: AnnotatedScrollBarScrollingEventKind,
    cancel: Rc<Cell<bool>>,
}

impl AnnotatedScrollBarScrollingEventArgs {
    pub(crate) fn new(scroll_offset: f64, scrolling_event_kind: AnnotatedScrollBarScrollingEventKind) -> Self {
        Self {
            scroll_offset,
            scrolling_event_kind,
            cancel: Rc::new(Cell::new(false)),
        }
    }

    /// The offset the bar requests (`ScrollOffset`).
    pub fn scroll_offset(&self) -> f64 {
        self.scroll_offset
    }

    /// What scrolls (`ScrollingEventKind`).
    pub fn scrolling_event_kind(&self) -> AnnotatedScrollBarScrollingEventKind {
        self.scrolling_event_kind
    }

    /// Whether a handler cancelled the request (`Cancel`).
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Cancels the request (`true`) or lets it happen (`false`).
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
}

/// The arguments of
/// [`AnnotatedScrollBar::detail_label_requested`](super::AnnotatedScrollBar::detail_label_requested)
/// (WinUI's `AnnotatedScrollBarDetailLabelRequestedEventArgs`): the scroll offset under the
/// pointer; a handler sets the content of the detail label shown there. The clones given to the
/// handlers of markup share the content.
#[derive(Clone)]
pub struct AnnotatedScrollBarDetailLabelRequestedEventArgs {
    scroll_offset: f64,
    content: Rc<RefCell<Option<BoxedValue>>>,
}

impl AnnotatedScrollBarDetailLabelRequestedEventArgs {
    pub(crate) fn new(scroll_offset: f64) -> Self {
        Self {
            scroll_offset,
            content: Rc::new(RefCell::new(None)),
        }
    }

    /// The scroll offset under the pointer (`ScrollOffset`).
    pub fn scroll_offset(&self) -> f64 {
        self.scroll_offset
    }

    /// The content of the detail label (`Content`).
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    /// Sets the content of the detail label.
    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }
}

impl PartialEq for AnnotatedScrollBarDetailLabelRequestedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.content, &other.content) && self.scroll_offset == other.scroll_offset
    }
}

impl fmt::Debug for AnnotatedScrollBarDetailLabelRequestedEventArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnnotatedScrollBarDetailLabelRequestedEventArgs")
            .field("scroll_offset", &self.scroll_offset)
            .field("has_content", &self.content.borrow().is_some())
            .finish()
    }
}

ferro_markup_type!(class AnnotatedScrollBarLabel {
    namespace: "FerroUI.Controls",
    handles: [AnnotatedScrollBarLabel],
    properties: [
        Content: Option<BoxedValue> { get: AnnotatedScrollBarLabel::content },
        ScrollOffset: f64 { get: AnnotatedScrollBarLabel::scroll_offset },
    ],
});

ferro_markup_type!(class AnnotatedScrollBarScrollingEventArgs {
    namespace: "FerroUI.Controls",
    handles: [AnnotatedScrollBarScrollingEventArgs],
    properties: [
        ScrollOffset: f64 { get: AnnotatedScrollBarScrollingEventArgs::scroll_offset },
        ScrollingEventKind: AnnotatedScrollBarScrollingEventKind {
            get: AnnotatedScrollBarScrollingEventArgs::scrolling_event_kind
        },
        Cancel: bool {
            get: AnnotatedScrollBarScrollingEventArgs::cancel,
            set: AnnotatedScrollBarScrollingEventArgs::set_cancel
        },
    ],
});

ferro_markup_type!(class AnnotatedScrollBarDetailLabelRequestedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [AnnotatedScrollBarDetailLabelRequestedEventArgs],
    properties: [
        ScrollOffset: f64 { get: AnnotatedScrollBarDetailLabelRequestedEventArgs::scroll_offset },
        Content: Option<BoxedValue> {
            get: AnnotatedScrollBarDetailLabelRequestedEventArgs::content,
            set: AnnotatedScrollBarDetailLabelRequestedEventArgs::set_content
        },
    ],
});
