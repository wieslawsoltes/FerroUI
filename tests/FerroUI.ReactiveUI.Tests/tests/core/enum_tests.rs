//! `EnumTests` (`ReactiveUI.Tests/Core/EnumTests.cs`): the values of the enumerations of the
//! bindings.

use ferroui_reactiveui::*;

#[test]
fn binding_direction_enum_values_are_constant() {
    const ASYNC_ONE_WAY_VALUE: i32 = 2;
    // Assert
    assert_eq!(BindingDirection::OneWay as i32, 0);
    assert_eq!(BindingDirection::TwoWay as i32, 1);
    assert_eq!(BindingDirection::AsyncOneWay as i32, ASYNC_ONE_WAY_VALUE);
}

// Not ported: TriggerUpdate_EnumValues_AreConstant - needs TriggerUpdate (bindings, a later wave)
