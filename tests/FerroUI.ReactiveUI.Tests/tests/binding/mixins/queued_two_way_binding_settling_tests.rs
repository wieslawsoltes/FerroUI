//! `QueuedTwoWayBindingSettlingTests` (`ReactiveUI.Binding.Tests/Mixins/QueuedTwoWayBindingSettlingTests.cs`).
//!
//! The rows of upstream's data sources are loops. The amounts are `f64` where upstream has a
//! `decimal`, which lives in `ferroui-base` until phase 1b moves it; `F2` is `{:.2}`. The timeouts
//! of upstream's tests have no counterpart: a binding that never settles fails on the write limit
//! of the models, or on the pass limit of the scheduler.

use crate::tests::binding::test_models::{AmountView, AmountViewModel, DispatchStubView, DispatchStubViewModel, ManualSequencer};
use ferroui_reactiveui::*;
use ferroui_system::IDisposable;
use std::any::Any;
use std::rc::Rc;

const MODEL_VALUE: &str = "Rent March";

const VIEW_VALUE: &str = "";

const VIEW_EDIT: &str = "typed by the user";

const MODEL_EDIT: &str = "Rent April";

const AMOUNT: f64 = 125.50;

const AMOUNT_TEXT: &str = "125.50";

const MARKER: &str = "!";

const PASS_LIMIT: i32 = 20;

/// The forms of the bindings of text (`BindingKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingKind {
    ModelFirst,
    ModelFirstConverting,
    ViewFirstConverting,
}

const BINDINGS: [BindingKind; 3] = [BindingKind::ModelFirst, BindingKind::ModelFirstConverting, BindingKind::ViewFirstConverting];

/// The forms of the bindings of amounts (`NumericBindingKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericBindingKind {
    ModelFirstFunctions,
    ModelFirstConverters,
    ViewFirstFunctions,
    ViewFirstConverters,
}

const NUMERIC_BINDINGS: [NumericBindingKind; 4] = [
    NumericBindingKind::ModelFirstFunctions,
    NumericBindingKind::ModelFirstConverters,
    NumericBindingKind::ViewFirstFunctions,
    NumericBindingKind::ViewFirstConverters,
];

/// The affinity that wins over every built-in converter.
const WINNING_AFFINITY: i32 = 100;

/// Formats an amount (`AmountToTextConverter`).
struct AmountToTextConverter;

impl BindingTypeConverter for AmountToTextConverter {
    type From = f64;
    type To = String;

    fn affinity(&self) -> i32 {
        WINNING_AFFINITY
    }

    fn convert(&self, from: Option<&f64>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        from.map(|amount| format_amount(*amount))
    }
}

/// Parses an amount, failing on a text that is not one (`TextToAmountConverter`).
struct TextToAmountConverter;

impl BindingTypeConverter for TextToAmountConverter {
    type From = String;
    type To = f64;

    fn affinity(&self) -> i32 {
        WINNING_AFFINITY
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<f64> {
        from?.parse().ok()
    }
}

fn format_amount(amount: f64) -> String {
    format!("{amount:.2}")
}

/// Parses an amount; a text that is not one panics, as `decimal.Parse` throws.
fn parse_amount(text: String) -> f64 {
    match text.parse() {
        Ok(amount) => amount,
        Err(error) => panic!("{text} is not an amount: {error}"),
    }
}

fn show(kind: BindingKind, value: &str) -> String {
    if kind == BindingKind::ModelFirst {
        value.to_string()
    } else {
        format!("{value}{MARKER}")
    }
}

fn mark(value: String) -> String {
    format!("{value}{MARKER}")
}

fn strip(value: String) -> String {
    value.strip_suffix(MARKER).map(str::to_string).unwrap_or(value)
}

fn as_disposable<V: IViewFor + ?Sized + 'static, T: 'static>(binding: Option<Rc<dyn IReactiveBinding<V, T>>>) -> Rc<dyn IDisposable> {
    binding.expect("no hook refuses the binding")
}

fn connect_amounts(kind: NumericBindingKind) -> (Rc<AmountViewModel>, Rc<AmountView>, Rc<ManualSequencer>, Rc<dyn IDisposable>) {
    let view_model = AmountViewModel::new();
    let view = AmountView::new();
    IViewFor::set_view_model(&*view, Some(view_model.clone() as Rc<dyn Any>));
    let sequencer = ManualSequencer::new();
    let amount = || settable_prop!(AmountViewModel => amount);
    let text = || settable_prop!(AmountView => text);
    let to_text: Option<Rc<dyn IBindingTypeConverter>> = Some(Rc::new(AmountToTextConverter));
    let to_amount: Option<Rc<dyn IBindingTypeConverter>> = Some(Rc::new(TextToAmountConverter));

    let connection = match kind {
        NumericBindingKind::ModelFirstFunctions => view_model.bind_two_way_select_on(&view, amount(), text(), format_amount, parse_amount, Some(sequencer.scheduler())),
        NumericBindingKind::ModelFirstConverters => view_model.bind_two_way_with(&view, amount(), text(), to_text, to_amount, None, Some(sequencer.scheduler())),
        NumericBindingKind::ViewFirstFunctions => as_disposable(view.bind_select_on(view_model.clone(), amount(), text(), format_amount, parse_amount, Some(sequencer.scheduler()))),
        NumericBindingKind::ViewFirstConverters => as_disposable(view.bind_with(view_model.clone(), amount(), text(), to_text, to_amount, None, Some(sequencer.scheduler()))),
    };

    (view_model, view, sequencer, connection)
}

fn connect(kind: BindingKind) -> (Rc<DispatchStubViewModel>, Rc<DispatchStubView>, Rc<ManualSequencer>, Rc<dyn IDisposable>) {
    let model = DispatchStubViewModel::new();
    model.set_caption(MODEL_VALUE.to_string());
    let view = DispatchStubView::with_caption(VIEW_VALUE);
    IViewFor::set_view_model(&*view, Some(model.clone() as Rc<dyn Any>));
    let sequencer = ManualSequencer::new();
    let model_caption = || settable_prop!(DispatchStubViewModel => caption);
    let view_caption = || settable_prop!(DispatchStubView => caption);

    let connection = match kind {
        BindingKind::ModelFirst => model.bind_two_way_on(&view, model_caption(), view_caption(), Some(sequencer.scheduler())),
        BindingKind::ModelFirstConverting => model.bind_two_way_select_on(&view, model_caption(), view_caption(), mark, strip, Some(sequencer.scheduler())),
        BindingKind::ViewFirstConverting => as_disposable(view.bind_select_on(model.clone(), model_caption(), view_caption(), mark, strip, Some(sequencer.scheduler()))),
    };

    (model, view, sequencer, connection)
}

#[test]
fn bind_with_unequal_sides_goes_idle() {
    for kind in BINDINGS {
        let (_, _, sequencer, connection) = connect(kind);

        let passes = sequencer.run_until_idle(PASS_LIMIT);

        assert_ne!(passes, -1, "{kind:?}");
        connection.dispose();
    }
}

#[test]
fn bind_with_unequal_sides_carries_the_model_to_the_view() {
    for kind in BINDINGS {
        let (model, view, sequencer, connection) = connect(kind);

        let _ = sequencer.run_until_idle(PASS_LIMIT);

        assert_eq!(model.caption(), MODEL_VALUE, "{kind:?}");
        assert_eq!(view.caption(), show(kind, MODEL_VALUE), "{kind:?}");
        connection.dispose();
    }
}

#[test]
fn bind_after_settling_carries_a_view_edit_to_the_model() {
    for kind in BINDINGS {
        let (model, view, sequencer, connection) = connect(kind);
        let _ = sequencer.run_until_idle(PASS_LIMIT);

        view.set_caption(show(kind, VIEW_EDIT));
        let passes = sequencer.run_until_idle(PASS_LIMIT);

        assert_ne!(passes, -1, "{kind:?}");
        assert_eq!(model.caption(), VIEW_EDIT, "{kind:?}");
        assert_eq!(view.caption(), show(kind, VIEW_EDIT), "{kind:?}");
        connection.dispose();
    }
}

#[test]
fn bind_after_settling_carries_a_model_edit_to_the_view() {
    for kind in BINDINGS {
        let (model, view, sequencer, connection) = connect(kind);
        let _ = sequencer.run_until_idle(PASS_LIMIT);

        model.set_caption(MODEL_EDIT.to_string());
        let passes = sequencer.run_until_idle(PASS_LIMIT);

        assert_ne!(passes, -1, "{kind:?}");
        assert_eq!(model.caption(), MODEL_EDIT, "{kind:?}");
        assert_eq!(view.caption(), show(kind, MODEL_EDIT), "{kind:?}");
        connection.dispose();
    }
}

#[test]
fn bind_with_an_edit_before_the_first_delivery_goes_idle_on_the_edit() {
    for kind in NUMERIC_BINDINGS {
        let (view_model, view, sequencer, connection) = connect_amounts(kind);

        view_model.set_amount(AMOUNT);
        let passes = sequencer.run_until_idle(PASS_LIMIT);

        assert_ne!(passes, -1, "{kind:?}");
        assert_eq!(view_model.amount(), AMOUNT, "{kind:?}");
        assert_eq!(view.text(), AMOUNT_TEXT, "{kind:?}");
        connection.dispose();
    }
}

#[test]
fn bind_with_a_burst_of_edits_settles_on_the_latest() {
    for kind in NUMERIC_BINDINGS {
        let (view_model, view, sequencer, connection) = connect_amounts(kind);

        view_model.set_amount(1.0);
        view_model.set_amount(AMOUNT);
        let passes = sequencer.run_until_idle(PASS_LIMIT);

        assert_ne!(passes, -1, "{kind:?}");
        assert_eq!(view_model.amount(), AMOUNT, "{kind:?}");
        assert_eq!(view.text(), AMOUNT_TEXT, "{kind:?}");
        connection.dispose();
    }
}
