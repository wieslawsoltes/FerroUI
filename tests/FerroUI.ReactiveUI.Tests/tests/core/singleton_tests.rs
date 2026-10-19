//! `SingletonTests` (`ReactiveUI.Tests/Core/SingletonTests.cs`): the shared arguments of the
//! notifications of reactive properties.
//!
//! The shared `PropertyChangedEventArgs` of upstream are the names of the properties
//! (`SingletonPropertyChangedEventArgs::VALUE`), which the `PropertyChanged` event carries.

use ferroui_reactiveui::*;

#[test]
fn singleton_data_errors_changed_event_args_value_has_correct_property_name() {
    assert_eq!(SingletonDataErrorsChangedEventArgs::VALUE.property_name(), Some("Value"));
}

#[test]
fn singleton_property_changed_event_args_error_message_has_correct_property_name() {
    assert_eq!(SingletonPropertyChangedEventArgs::ERROR_MESSAGE, "ErrorMessage");
}

#[test]
fn singleton_property_changed_event_args_has_errors_has_correct_property_name() {
    assert_eq!(SingletonPropertyChangedEventArgs::HAS_ERRORS, "HasErrors");
}

#[test]
fn singleton_property_changed_event_args_value_has_correct_property_name() {
    assert_eq!(SingletonPropertyChangedEventArgs::VALUE, "Value");
}
