//! `ObservedChangeTests` (`ReactiveUI.Binding.Tests/ObservableForProperty/ObservedChangeTests.cs`).

use ferroui_reactiveui::*;

/// The property value the observed change reports.
const OBSERVED_VALUE: i32 = 42;

/// The object the observed change reports the value from.
const OBSERVED_SENDER: &str = "sender";

/// A change carries the sender, the value, and the expression it was observed through.
#[test]
fn constructor_reports_sender_value_and_expression() {
    // Upstream passes a null expression; the expression of the port is the text of a property
    // path, and the absent path is the empty text.
    let change = ObservedChange::new(OBSERVED_SENDER, "", OBSERVED_VALUE);

    assert_eq!(change.sender, OBSERVED_SENDER);
    assert_eq!(change.value, OBSERVED_VALUE);
    assert!(change.expression.is_empty());
    assert_eq!(*change.sender(), OBSERVED_SENDER);
    assert_eq!(*change.value(), OBSERVED_VALUE);
    assert_eq!(change.expression(), "");
}
