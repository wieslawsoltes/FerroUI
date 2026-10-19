//! `ReactiveObjectToPropertyIntegrationTests`
//! (`ReactiveUI.Tests/ReactiveObjects/ReactiveObjectToPropertyIntegrationTests.cs`): the helpers of
//! derived properties of reactive objects, end to end.

use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::*;
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging};
use ferroui_system::{prop, IObservable, IObserver, ObservableError};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

const UPDATED_VALUE: &str = "updated";

/// A view model with two derived properties, one named by its path and one by its name
/// (`ToPropertyViewModel`).
#[derive(Default)]
struct ToPropertyViewModel {
    reactive: ReactiveObject,
    upper: OnceCell<ObservableAsPropertyHelper<Option<String>>>,
    named: OnceCell<ObservableAsPropertyHelper<Option<String>>>,
}
reactive_object!(ToPropertyViewModel);

impl ToPropertyViewModel {
    fn new(upper_source: Rc<dyn IObservable<Option<String>>>, named_source: Rc<dyn IObservable<Option<String>>>) -> Rc<Self> {
        let view_model = Rc::new(Self::default());
        let upper = upper_source.to_property(&view_model, prop!(ToPropertyViewModel => upper));
        let _ = view_model.upper.set(upper);
        // Upstream names this property with `nameof(Named)`; the path names it here.
        let named = named_source.to_property(&view_model, prop!(ToPropertyViewModel => named));
        let _ = view_model.named.set(named);
        view_model
    }

    fn upper(&self) -> Option<String> {
        self.upper.get().and_then(|helper| helper.value())
    }

    fn named(&self) -> Option<String> {
        self.named.get().and_then(|helper| helper.value())
    }

    fn upper_helper(&self) -> &ObservableAsPropertyHelper<Option<String>> {
        self.upper.get().expect("the helper is set")
    }
}

reactive_properties! {
    /// A view model whose full name derives from two properties (`DerivedViewModel`).
    #[derive(Default)]
    struct DerivedViewModel {
        reactive: ReactiveObject,
        #[reactive]
        first_name: Option<String>,
        #[reactive]
        last_name: Option<String>,
        full_name: OnceCell<ObservableAsPropertyHelper<String>>,
    }
}
reactive_object!(DerivedViewModel);

impl DerivedViewModel {
    fn new() -> Rc<Self> {
        let view_model = Rc::new(Self::default());
        let full_name = view_model
            .when_any_value_select(
                (prop!(DerivedViewModel => first_name), prop!(DerivedViewModel => last_name)),
                |(first, last): (Option<String>, Option<String>)| format!("{} {}", first.unwrap_or_default(), last.unwrap_or_default()),
            )
            .to_property(&view_model, prop!(DerivedViewModel => full_name));
        let _ = view_model.full_name.set(full_name);
        view_model
    }

    fn full_name(&self) -> String {
        self.full_name.get().map(|helper| helper.value()).unwrap_or_default()
    }
}

/// A view model whose helper subscribes when the value is first read (`DeferredViewModel`).
#[derive(Default)]
struct DeferredViewModel {
    reactive: ReactiveObject,
    value: OnceCell<ObservableAsPropertyHelper<Option<String>>>,
}
reactive_object!(DeferredViewModel);

impl DeferredViewModel {
    fn new(source: Rc<dyn IObservable<Option<String>>>) -> Rc<Self> {
        let view_model = Rc::new(Self::default());
        // `ToProperty(this, x => x.Value, deferSubscription: true)`: the `_with` form takes the
        // initial value, the deferral of the subscription and the scheduler.
        let value = source.to_property_with(&view_model, prop!(DeferredViewModel => value), ObservableAsPropertyInitialValue::default(), true, None);
        let _ = view_model.value.set(value);
        view_model
    }

    fn value(&self) -> Option<String> {
        self.value.get().and_then(|helper| helper.value())
    }
}

#[test]
fn selector_to_property_raises_changing_then_changed() {
    let source = Subject::<Option<String>>::new();
    let fixture = ToPropertyViewModel::new(source.as_observable(), Observable::never());
    let events = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = events.clone();
    fixture.property_changing().add(Rc::new(move |name: &str| sink.borrow_mut().push(format!("changing:{name}"))));
    let sink = events.clone();
    fixture.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(format!("changed:{name}"))));

    source.on_next(Some(UPDATED_VALUE.to_owned()));

    assert_eq!(fixture.upper().as_deref(), Some(UPDATED_VALUE));
    assert_eq!(*events.borrow(), ["changing:Upper", "changed:Upper"]);
}

#[test]
fn name_of_to_property_raises_changed() {
    let source = Subject::<Option<String>>::new();
    let fixture = ToPropertyViewModel::new(Observable::never(), source.as_observable());
    let changed = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = changed.clone();
    fixture.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_owned())));

    source.on_next(Some(UPDATED_VALUE.to_owned()));

    assert_eq!(fixture.named().as_deref(), Some(UPDATED_VALUE));
    assert!(changed.borrow().iter().any(|name| name == "Named"));
}

#[test]
fn when_any_value_to_property_follows_source_property() {
    let fixture = DerivedViewModel::new();
    fixture.set_first_name(Some("Ada".to_owned()));
    fixture.set_last_name(Some("Lovelace".to_owned()));

    assert_eq!(fixture.full_name(), "Ada Lovelace");

    fixture.set_last_name(Some("Byron".to_owned()));

    assert_eq!(fixture.full_name(), "Ada Byron");
}

#[test]
fn deferred_to_property_subscribes_on_first_read() {
    let subscribed = Rc::new(Cell::new(false));
    let flag = subscribed.clone();
    let source = Observable::create(move |observer: Rc<dyn IObserver<Option<String>>>| {
        flag.set(true);
        observer.on_next(Some(UPDATED_VALUE.to_owned()));
        Disposable::empty()
    });

    let fixture = DeferredViewModel::new(source);

    assert!(!subscribed.get());
    assert_eq!(fixture.value().as_deref(), Some(UPDATED_VALUE));
    assert!(subscribed.get());
}

#[test]
fn source_error_reaches_thrown_exceptions() {
    let source = Subject::<Option<String>>::new();
    let fixture = ToPropertyViewModel::new(source.as_observable(), Observable::never());
    let errors = Rc::new(RefCell::new(Vec::<ObservableError>::new()));
    let sink = errors.clone();
    let subscription = fixture.upper_helper().thrown_exceptions().subscribe_next(move |error| sink.borrow_mut().push(error));

    source.on_error(Rc::new(PanicException { message: "boom".to_owned() }));

    assert_eq!(errors.borrow().len(), 1);
    subscription.dispose();
}
