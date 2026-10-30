//! The arguments of `AutoCompleteBox.QuerySubmitted`, a FerroUI addition after WinUI's
//! `AutoSuggestBoxQuerySubmittedEventArgs` (DEVIATIONS.md, "WinUI additions to upstream
//! controls").

use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::BoxedValue;

/// Provides data for the `QuerySubmitted` event of the [`AutoCompleteBox`](super::AutoCompleteBox):
/// the text of the box when Enter submitted it, and the suggestion that Enter chose from the open
/// drop-down, if any.
#[derive(Clone)]
pub struct AutoCompleteBoxQuerySubmittedEventArgs {
    base: RoutedEventArgs,
    query_text: String,
    chosen_suggestion: Option<BoxedValue>,
}

ferro_routed_event_args!(AutoCompleteBoxQuerySubmittedEventArgs: RoutedEventArgs);

impl AutoCompleteBoxQuerySubmittedEventArgs {
    /// Creates args for a routed event, the submitted text and the chosen suggestion.
    pub fn new<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        query_text: String,
        chosen_suggestion: Option<BoxedValue>,
    ) -> Self {
        let base = RoutedEventArgs::new();
        base.set_routed_event(routed_event);
        Self { base, query_text, chosen_suggestion }
    }

    /// The text of the box when the query was submitted.
    pub fn query_text(&self) -> &str {
        &self.query_text
    }

    /// The item chosen from the open drop-down by the Enter that submitted the query; `None` when
    /// the drop-down was closed or nothing was selected in it.
    pub fn chosen_suggestion(&self) -> Option<&BoxedValue> {
        self.chosen_suggestion.as_ref()
    }
}
