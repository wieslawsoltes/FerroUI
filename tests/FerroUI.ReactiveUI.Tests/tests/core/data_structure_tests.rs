//! `DataStructureTests` (`ReactiveUI.Tests/Core/DataStructureTests.cs`): the arguments of changes.

use ferroui_reactiveui::*;

#[test]
fn observed_change_constructor_stores_all_values() {
    // Arrange
    const SENDER: &str = "test sender";
    // Upstream passes the expression `Expression.Constant("test")`; the expression is the text of
    // the path here.
    let expression = "test";
    const VALUE: i32 = 42;

    // Act
    let observed_change = ObservedChange::new(SENDER, expression, VALUE);

    // Assert
    assert_eq!(observed_change.sender, SENDER);
    assert_eq!(observed_change.expression, expression);
    assert_eq!(observed_change.value, VALUE);
}

#[test]
fn reactive_property_changed_event_args_constructor_stores_values() {
    // Arrange
    const SENDER: &str = "test sender";
    const PROPERTY_NAME: &str = "TestProperty";

    // Act
    let event_args = ReactivePropertyChangedEventArgs::new(SENDER, PROPERTY_NAME);

    // Assert
    assert_eq!(event_args.sender, SENDER);
    assert_eq!(event_args.property_name, PROPERTY_NAME);
    assert_eq!(IReactivePropertyChangedEventArgs::property_name(&event_args), PROPERTY_NAME);
    assert_eq!(*IReactivePropertyChangedEventArgs::sender(&event_args), SENDER);
}
