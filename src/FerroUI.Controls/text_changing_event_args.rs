use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{FerroObject, Nullable};
use std::ops::Range;
use std::rc::Rc;

/// The exact replacement made by a native text-editing operation.
///
/// Positions count UTF-16 code units in `previous_text`. A rich document can
/// rebase its ranges at this location even when repeated characters would make
/// a comparison of the old and new strings identify a different location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextBoxTextChange {
    previous_text: Rc<str>,
    replaced_range: Range<i32>,
    inserted_length: i32,
}

impl TextBoxTextChange {
    pub fn previous_text(&self) -> &str { &self.previous_text }
    pub fn replaced_range(&self) -> Range<i32> { self.replaced_range.clone() }
    pub fn inserted_length(&self) -> i32 { self.inserted_length }

    pub(crate) fn new(previous_text: String, replaced_range: Range<i32>, inserted_length: i32) -> Self {
        Self { previous_text: Rc::from(previous_text), replaced_range, inserted_length }
    }

    // A subclass can coerce Text, and synchronous binding/handler callbacks
    // can replace it again. Only attach this edit to the matching notification.
    pub(crate) fn matches(&self, previous: &str, next: &str) -> bool {
        if self.previous_text() != previous || self.replaced_range.start < 0
            || self.replaced_range.end < self.replaced_range.start || self.inserted_length < 0 { return false; }
        let Some(new_end) = self.replaced_range.start.checked_add(self.inserted_length) else { return false; };
        let offset = |text: &str, target: i32| {
            let mut units = 0;
            for (byte, character) in text.char_indices() {
                if units == target { return Some(byte); }
                units += character.len_utf16() as i32;
                if units > target { return None; }
            }
            (units == target).then_some(text.len())
        };
        let (Some(old_start), Some(old_end), Some(next_start), Some(next_end)) = (
            offset(previous, self.replaced_range.start), offset(previous, self.replaced_range.end),
            offset(next, self.replaced_range.start), offset(next, new_end),
        ) else { return false; };
        previous[..old_start] == next[..next_start] && previous[old_end..] == next[next_end..]
    }
}

/// Provides data specific to a `TextBox::text_changing` event.
#[derive(Clone, Default)]
pub struct TextChangingEventArgs {
    base: RoutedEventArgs,
    text_change: Option<TextBoxTextChange>,
}

ferro_routed_event_args!(TextChangingEventArgs: RoutedEventArgs);

impl TextChangingEventArgs {
    /// Initializes a new instance of the `TextChangingEventArgs` class.
    ///
    /// `routed_event` is the routed event associated with these event args.
    pub fn with_event<T: ?Sized>(routed_event: &RoutedEvent<T>) -> Self {
        Self { base: RoutedEventArgs::with_event(routed_event), text_change: None }
    }

    /// Initializes a new instance of the `TextChangingEventArgs` class.
    ///
    /// `routed_event` is the routed event associated with these event args,
    /// `source` the source object that raised the routed event.
    pub fn with_event_and_source<T: ?Sized>(
        routed_event: &RoutedEvent<T>,
        source: impl Into<Nullable<FerroObject>>,
    ) -> Self {
        Self { base: RoutedEventArgs::with_event_and_source(routed_event, source), text_change: None }
    }

    /// Native replacement coordinates, when known. External Text assignments,
    /// undo/redo synchronization and coercions may have no single edit range.
    pub fn text_change(&self) -> Option<&TextBoxTextChange> { self.text_change.as_ref() }

    pub(crate) fn with_text_change(mut self, change: Option<TextBoxTextChange>) -> Self {
        self.text_change = change;
        self
    }
}
