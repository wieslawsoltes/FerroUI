//! Not from upstream (C5 of #423, step T3 of docs/idiomatic-rust-signatures.md): the timer
//! intervals and delays of the controls are `Duration` properties. A time span that is not
//! negative reaches them through a binding as the duration of the same value; a negative one
//! fails its conversion like any value a binding cannot convert, and the property keeps the
//! value it has without the binding, never clamped to zero (decision O2). The markup forms of
//! these tests are in `tests/FerroUI.Markup.Xaml.UnitTests` (`xaml::ferro_intrinsics_tests`
//! and `emitter::differential_tests`).

use crate::presenters::TextPresenter;
use crate::primitives::ScrollBar;
use crate::test_support::test_scope;
use crate::{AutoCompleteBox, RichEditBox, TextBox};
use ferroui_base::animation::SignedDuration;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::{ferro_model, BoxedValue, FerroObject, FerroProperty};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// A view model with a time span.
struct Timing {
    span: Cell<SignedDuration>,
    property_changed: Event<str>,
}

impl Timing {
    fn new(span: SignedDuration) -> Rc<Self> {
        Model::new_model(Self { span: Cell::new(span), property_changed: Event::new() })
    }

    fn set_span(&self, value: SignedDuration) {
        self.span.set(value);
        self.property_changed.raise("Span");
    }
}

impl INotifyPropertyChanged for Timing {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Timing, |b| b
    .notify_property_changed()
    .property::<Value<SignedDuration>>("Span", |o| o.span.get(), |o, v| o.set_span(v)));

/// Binds `property` of `target` to a time span that starts negative: the property keeps
/// `default` (what `get` reads without the binding); a span that is not negative is set as the
/// duration of the same value; a negative one again returns the property to `default`.
fn a_negative_span_keeps_the_default(
    target: &FerroObject,
    property: &'static FerroProperty,
    get: impl Fn() -> Duration,
    default: Duration,
) {
    let name = property.name();
    assert_eq!(get(), default, "{name}: the default");
    let source = Timing::new(SignedDuration::from_seconds(-1.0));
    let binding = ReflectionBinding::new("Span");
    binding.set_source(Some(source.clone() as BoxedValue));
    target.bind_binding(property, &*binding);
    assert_eq!(get(), default, "{name}: a negative span is not set");

    source.set_span(SignedDuration::from_milliseconds(1500.0));
    assert_eq!(get(), Duration::from_millis(1500), "{name}: a span that is not negative is set");

    source.set_span(SignedDuration::from_seconds(-2.0));
    assert_eq!(get(), default, "{name}: a negative span returns the property to its default");
}

#[test]
fn a_negative_span_from_a_binding_leaves_the_caret_blink_interval_at_its_default() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    a_negative_span_keeps_the_default(
        &text_box,
        TextBox::caret_blink_interval_property().as_property(),
        || text_box.caret_blink_interval(),
        Duration::from_millis(500),
    );

    let presenter = TextPresenter::new();
    a_negative_span_keeps_the_default(
        &presenter,
        TextPresenter::caret_blink_interval_property().as_property(),
        || presenter.caret_blink_interval(),
        Duration::from_millis(500),
    );

    let editor = RichEditBox::new();
    a_negative_span_keeps_the_default(
        &editor,
        RichEditBox::caret_blink_interval_property().as_property(),
        || editor.caret_blink_interval(),
        Duration::from_millis(500),
    );
}

#[test]
fn a_negative_span_from_a_binding_leaves_the_scroll_bar_delays_at_their_defaults() {
    let _scope = test_scope();
    let scroll_bar = ScrollBar::new();
    a_negative_span_keeps_the_default(
        &scroll_bar,
        ScrollBar::hide_delay_property().as_property(),
        || scroll_bar.hide_delay(),
        Duration::from_secs(2),
    );

    let scroll_bar = ScrollBar::new();
    a_negative_span_keeps_the_default(
        &scroll_bar,
        ScrollBar::show_delay_property().as_property(),
        || scroll_bar.show_delay(),
        Duration::from_millis(500),
    );
}

#[test]
fn a_negative_span_from_a_binding_leaves_the_minimum_populate_delay_at_its_default() {
    let _scope = test_scope();
    let auto_complete_box = AutoCompleteBox::new();
    a_negative_span_keeps_the_default(
        &auto_complete_box,
        AutoCompleteBox::minimum_populate_delay_property().as_property(),
        || auto_complete_box.minimum_populate_delay(),
        Duration::ZERO,
    );
}
