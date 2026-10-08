//! `ColorSpectrumAutomationPeerTests.cs` of the upstream controls tests
//! (`ColorSpectrumAutomationPeerTests.AutomationPeerTests`). Upstream keeps
//! it with the tests of the controls; here it is with the crate of the
//! spectrum, which the controls crate does not depend on.

use super::ColorSpectrumAutomationPeer;
use crate::primitives::ColorSpectrum;
use ferroui_base::media::Colors;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::utilities::FormatError;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::IValueProvider;
use ferroui_controls::automation::{AutomationPropertyChangedEventArgs, ValuePatternIdentifiers};
use std::cell::RefCell;
use std::rc::Rc;

/// `ScopedTestBase`: the test runs in a scope of its own.
fn scope() -> UnitTestDispatcherScope {
    Dispatcher::unit_test_scope()
}

/// `(ColorSpectrumAutomationPeer)ControlAutomationPeer.CreatePeerForElement(spectrum)`.
fn create_peer(spectrum: &ColorSpectrum) -> Ref<ColorSpectrumAutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(spectrum).cast().expect("a ColorSpectrumAutomationPeer")
}

fn text(value: Option<&BoxedValue>) -> Option<String> {
    value.and_then(|value| value.downcast_ref::<String>().cloned())
}

#[test]
fn creates_color_spectrum_automation_peer() {
    let _scope = scope();
    let spectrum = ColorSpectrum::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&spectrum);

    assert!(std::ptr::eq(peer.get_type(), ColorSpectrumAutomationPeer::TYPE));
}

#[test]
fn control_type_is_custom() {
    let _scope = scope();
    let spectrum = ColorSpectrum::new();
    let peer = create_peer(&spectrum);

    assert_eq!(AutomationControlType::Custom, peer.get_automation_control_type());
}

#[test]
fn class_name_is_color_spectrum() {
    let _scope = scope();
    let spectrum = ColorSpectrum::new();
    let peer = create_peer(&spectrum);

    assert_eq!("ColorSpectrum", peer.get_class_name());
}

#[test]
fn implements_i_value_provider() {
    let _scope = scope();
    let spectrum = ColorSpectrum::new();
    let peer = create_peer(&spectrum);

    assert_eq!(Some(Colors::WHITE.to_string()), peer.value());

    let value_provider = peer.get_provider::<dyn IValueProvider>().expect("an IValueProvider");
    value_provider.set_value(Some("#00FF00")).unwrap();

    assert_eq!(Some(Colors::LIME.to_string()), peer.value());
    assert_eq!(Colors::LIME, spectrum.color());
}

#[test]
fn set_value_uses_color_parse_and_throws_format_exception_on_invalid_input() {
    let _scope = scope();
    let spectrum = ColorSpectrum::new();
    let peer = create_peer(&spectrum);
    let value_provider = peer.get_provider::<dyn IValueProvider>().expect("an IValueProvider");

    let error = value_provider.set_value(Some("not-a-color")).expect_err("a format error");
    assert!(error.downcast_ref::<FormatError>().is_some());
}

#[test]
fn value_property_raises_automation_property_changed_event_on_color_change() {
    let _scope = scope();
    let spectrum = ColorSpectrum::new();
    let peer = create_peer(&spectrum);
    let changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> = Rc::new(RefCell::new(None));

    let sink = changed.clone();
    let _subscription = peer.property_changed(move |e| {
        if std::ptr::eq(e.property(), ValuePatternIdentifiers::value_property()) {
            *sink.borrow_mut() = Some(e.clone());
        }
    });

    spectrum.set_color(Colors::BLACK);

    let changed = changed.borrow();
    let changed = changed.as_ref().expect("a property change");
    assert!(std::ptr::eq(ValuePatternIdentifiers::value_property(), changed.property()));
    assert_eq!(Some(Colors::WHITE.to_string()), text(changed.old_value()));
    assert_eq!(Some(Colors::BLACK.to_string()), text(changed.new_value()));
}
