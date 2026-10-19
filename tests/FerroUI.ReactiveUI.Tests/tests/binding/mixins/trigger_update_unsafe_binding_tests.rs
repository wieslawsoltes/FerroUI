//! `TriggerUpdateUnsafeBindingTests` (`ReactiveUI.Binding.Tests/Mixins/TriggerUpdateUnsafeBindingTests.cs`).
//!
//! `BindUnsafe` with an update stream is
//! [`bind_with_trigger`](ferroui_reactiveui::PropertyBindingMixins::bind_with_trigger) and
//! [`bind_select_with_trigger`](ferroui_reactiveui::PropertyBindingMixins::bind_select_with_trigger);
//! the rows and matrices of upstream's data sources are loops.

use crate::tests::binding::test_models::{DispatchStubView, DispatchStubViewModel, StubViewThreadInvoker, TestAddress, TestViewModel};
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const MODEL_VALUE: &str = "model";

const VIEW_VALUE: &str = "view";

const EDITED_VALUE: &str = "edited";

const DIRECTIONS: [TriggerUpdate; 2] = [TriggerUpdate::ViewToViewModel, TriggerUpdate::ViewModelToView];

fn model(caption: &str) -> Rc<DispatchStubViewModel> {
    let model = DispatchStubViewModel::new();
    model.set_caption(caption.to_string());
    model
}

fn caption_of_model() -> SettableProperty<Rc<DispatchStubViewModel>, String> {
    settable_prop!(DispatchStubViewModel => caption)
}

fn caption_of_view() -> SettableProperty<Rc<DispatchStubView>, String> {
    settable_prop!(DispatchStubView => caption)
}

/// The changes of `binding`, recorded.
fn record_changes(binding: &Rc<dyn IReactiveBinding<DispatchStubView, BindingChange>>) -> (Rc<RefCell<Vec<BindingChange>>>, Rc<dyn IDisposable>) {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let sink = changes.clone();
    let subscription = binding.changed().subscribe_next(move |change| sink.borrow_mut().push(change));
    (changes, subscription)
}

/// Registers `invoker` and reads the invokers again.
fn register_invoker(invoker: &Rc<StubViewThreadInvoker>) {
    AppLocator::register_constant::<dyn IViewThreadInvoker>(invoker.clone());
    ViewThreadInvokers::refresh();
}

// Not ported: BindUnsafe_ExposesBothSignalOverloads - reflection

#[test]
fn bind_unsafe_all_wirings_initializes_from_view_model() {
    for direction in DIRECTIONS {
        for has_stream in [false, true] {
            for converters in [false, true] {
                let model = model(MODEL_VALUE);
                let view = DispatchStubView::with_caption(VIEW_VALUE);
                let signal: Rc<Subject<i32>> = Subject::new();
                let updates: Option<Rc<dyn IObservable<i32>>> = has_stream.then(|| signal.as_observable());
                let binding = if converters {
                    view.bind_select_with_trigger(model.clone(), caption_of_model(), caption_of_view(), |value: String| value, |value: String| value, updates, direction)
                } else {
                    view.bind_with_trigger(model.clone(), caption_of_model(), caption_of_view(), updates, direction)
                }
                .expect("no hook refuses the binding");

                assert_eq!(view.caption(), MODEL_VALUE, "{direction:?}, stream {has_stream}, converters {converters}");
                assert_eq!(model.caption(), MODEL_VALUE);
                assert_eq!(binding.direction(), BindingDirection::TwoWay);
                binding.dispose();
            }
        }
    }
}

#[test]
fn bind_unsafe_view_signal_replaces_view_notifications() {
    let model = model(MODEL_VALUE);
    let view = DispatchStubView::with_caption(VIEW_VALUE);
    let signal: Rc<Subject<i32>> = Subject::new();
    let binding = view
        .bind_with_trigger(model.clone(), caption_of_model(), caption_of_view(), Some(signal.as_observable()), TriggerUpdate::ViewToViewModel)
        .expect("no hook refuses the binding");
    let (changes, observer) = record_changes(&binding);

    view.set_caption(EDITED_VALUE.to_string());
    assert_eq!(model.caption(), MODEL_VALUE);
    signal.on_next(0);

    assert_eq!(model.caption(), EDITED_VALUE);
    assert_eq!(changes.borrow().len(), 1);
    assert!(!changes.borrow()[0].from_view_model);
    assert_eq!(changes.borrow()[0].value_as::<String>().map(String::as_str), Some(EDITED_VALUE));
    observer.dispose();
    binding.dispose();
}

#[test]
fn bind_unsafe_model_signal_requires_signal_after_initial_model_notification() {
    let model = model(MODEL_VALUE);
    let view = DispatchStubView::with_caption(VIEW_VALUE);
    let signal: Rc<Subject<i32>> = Subject::new();
    let binding = view
        .bind_with_trigger(model.clone(), caption_of_model(), caption_of_view(), Some(signal.as_observable()), TriggerUpdate::ViewModelToView)
        .expect("no hook refuses the binding");
    let (changes, observer) = record_changes(&binding);

    model.set_caption("first".to_string());
    model.set_caption("second".to_string());
    assert_eq!(view.caption(), MODEL_VALUE);
    signal.on_next(0);
    assert_eq!(view.caption(), "second");
    assert_eq!(changes.borrow().len(), 1);
    assert!(changes.borrow()[0].from_view_model);

    view.set_caption(EDITED_VALUE.to_string());
    assert_eq!(model.caption(), EDITED_VALUE);
    observer.dispose();
    binding.dispose();
}

#[test]
fn bind_unsafe_without_signal_observes_both_properties() {
    for direction in DIRECTIONS {
        let model = model(MODEL_VALUE);
        let view = DispatchStubView::with_caption(VIEW_VALUE);
        let binding = view
            .bind_with_trigger(model.clone(), caption_of_model(), caption_of_view(), None::<Rc<dyn IObservable<i32>>>, direction)
            .expect("no hook refuses the binding");

        model.set_caption("forward".to_string());
        assert_eq!(view.caption(), "forward", "{direction:?}");
        view.set_caption(EDITED_VALUE.to_string());
        assert_eq!(model.caption(), EDITED_VALUE, "{direction:?}");
        binding.dispose();
    }
}

#[test]
fn bind_unsafe_dispatched_signal_reads_at_delivery() {
    const DELIVERY_VALUE: &str = "delivery";
    for direction in DIRECTIONS {
        let model = model(MODEL_VALUE);
        let view = DispatchStubView::with_caption(VIEW_VALUE);
        let invoker = StubViewThreadInvoker::new(vec![view.clone() as Rc<dyn Any>]);
        invoker.set_has_access(true);
        register_invoker(&invoker);

        let signal: Rc<Subject<i32>> = Subject::new();
        let binding = view
            .bind_with_trigger(model.clone(), caption_of_model(), caption_of_view(), Some(signal.as_observable()), direction)
            .expect("no hook refuses the binding");
        invoker.set_has_access(false);
        if direction == TriggerUpdate::ViewModelToView {
            model.set_caption("queued".to_string());
            signal.on_next(0);
            model.set_caption(DELIVERY_VALUE.to_string());
            assert_eq!(view.caption(), MODEL_VALUE);
        } else {
            view.set_caption("queued".to_string());
            signal.on_next(0);
            view.set_caption(DELIVERY_VALUE.to_string());
            assert_eq!(model.caption(), MODEL_VALUE);
        }

        assert_eq!(invoker.post_count(), 1, "{direction:?}");
        invoker.run_posted();
        assert_eq!(view.caption(), DELIVERY_VALUE, "{direction:?}");
        assert_eq!(model.caption(), DELIVERY_VALUE, "{direction:?}");
        binding.dispose();
        AppLocator::unregister_all::<dyn IViewThreadInvoker>();
        ViewThreadInvokers::refresh();
    }
}

#[test]
fn bind_unsafe_view_edits_before_initial_dispatch_initializes_from_view_model() {
    for direction in DIRECTIONS {
        for has_stream in [false, true] {
            let model = model(MODEL_VALUE);
            let view = DispatchStubView::with_caption(VIEW_VALUE);
            let invoker = StubViewThreadInvoker::new(vec![view.clone() as Rc<dyn Any>]);
            register_invoker(&invoker);

            let signal: Rc<Subject<i32>> = Subject::new();
            let updates: Option<Rc<dyn IObservable<i32>>> = has_stream.then(|| signal.as_observable());
            let binding = view
                .bind_with_trigger(model.clone(), caption_of_model(), caption_of_view(), updates, direction)
                .expect("no hook refuses the binding");
            view.set_caption(EDITED_VALUE.to_string());
            signal.on_next(0);
            invoker.run_posted();

            assert_eq!(model.caption(), MODEL_VALUE, "{direction:?}, stream {has_stream}");
            assert_eq!(view.caption(), MODEL_VALUE, "{direction:?}, stream {has_stream}");
            binding.dispose();
            AppLocator::unregister_all::<dyn IViewThreadInvoker>();
            ViewThreadInvokers::refresh();
        }
    }
}

#[test]
fn bind_unsafe_converted_value_equals_destination_skips_write() {
    let model = model("same ");
    let view = DispatchStubView::with_caption("same");
    let writes = Rc::new(Cell::new(0));
    let counted = writes.clone();
    let _token = view.property_changed().add(Rc::new(move |_: &str| counted.set(counted.get() + 1)));
    let binding = view
        .bind_select_with_trigger(
            model.clone(),
            caption_of_model(),
            caption_of_view(),
            |value: String| value.trim().to_string(),
            |value: String| format!("{value} "),
            None::<Rc<dyn IObservable<i32>>>,
            TriggerUpdate::ViewToViewModel,
        )
        .expect("no hook refuses the binding");
    let (changes, observer) = record_changes(&binding);

    model.set_caption("same  ".to_string());
    assert_eq!(writes.get(), 0);
    assert!(changes.borrow().is_empty());
    view.set_caption(EDITED_VALUE.to_string());
    assert_eq!(model.caption(), format!("{EDITED_VALUE} "));
    assert_eq!(writes.get(), 1);
    assert_eq!(changes.borrow().len(), 1);
    observer.dispose();
    binding.dispose();
}

#[test]
fn dispose_all_wirings_stops_both_directions() {
    for direction in DIRECTIONS {
        for has_stream in [false, true] {
            let model = model(MODEL_VALUE);
            let view = DispatchStubView::with_caption(VIEW_VALUE);
            let signal: Rc<Subject<i32>> = Subject::new();
            let updates: Option<Rc<dyn IObservable<i32>>> = has_stream.then(|| signal.as_observable());
            let binding = view
                .bind_with_trigger(model.clone(), caption_of_model(), caption_of_view(), updates, direction)
                .expect("no hook refuses the binding");
            let completed = Rc::new(Cell::new(false));
            let done = completed.clone();
            let observer = binding.changed().subscribe_next_completed(|_| {}, move || done.set(true));

            binding.dispose();
            binding.dispose();
            model.set_caption("after disposal".to_string());
            assert_eq!(view.caption(), MODEL_VALUE, "{direction:?}, stream {has_stream}");
            view.set_caption(EDITED_VALUE.to_string());
            signal.on_next(0);
            assert_eq!(model.caption(), "after disposal");
            assert!(!signal.has_observers());
            assert!(completed.get());
            observer.dispose();
        }
    }
}

// Not ported: BindUnsafe_ModelParentMissing_ResumesWhenParentAppears - expression trees (a member chain whose parent is missing has no value; a property path of an absent parent reads an `Option`, which the binding converts as any value)

// Not ported: BindUnsafe_ViewParentMissing_InitializesWhenParentAppears - expression trees (a member chain whose parent is missing has no value; a property path of an absent parent reads an `Option`, which the binding converts as any value)

/// A view that is a [`TestViewModel`] with an `Age` (`ChainView`).
struct ChainView {
    age: Cell<i32>,
    property_changed: Event<str>,
}

impl ChainView {
    fn new(age: i32) -> Rc<Self> {
        Rc::new(Self { age: Cell::new(age), property_changed: Event::new() })
    }

    fn age(&self) -> i32 {
        self.age.get()
    }

    fn set_age(&self, value: i32) {
        self.age.set(value);
        self.property_changed.raise("Age");
    }
}

impl INotifyPropertyChanged for ChainView {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl IActivatableView for ChainView {}

impl IViewFor for ChainView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        None
    }

    fn set_view_model(&self, _view_model: Option<Rc<dyn Any>>) {}
}

#[test]
fn bind_unsafe_conversion_unavailable_skips_both_directions() {
    const INITIAL_AGE: i32 = 11;
    const EDITED_AGE: i32 = 22;
    let address = TestAddress::new();
    address.set_city(MODEL_VALUE.to_string());
    let model = TestViewModel::new();
    model.set_address(Some(address.clone()));
    let view = ChainView::new(INITIAL_AGE);
    let binding = view
        .bind_with_trigger(
            model.clone(),
            settable_prop!(TestViewModel => address),
            settable_prop!(ChainView => age),
            None::<Rc<dyn IObservable<i32>>>,
            TriggerUpdate::ViewToViewModel,
        )
        .expect("no hook refuses the binding");
    assert_eq!(view.age(), INITIAL_AGE);

    view.set_age(EDITED_AGE);
    assert!(model.address().is_some_and(|held| Rc::ptr_eq(&held, &address)));
    assert_eq!(view.age(), EDITED_AGE);
    binding.dispose();
}
