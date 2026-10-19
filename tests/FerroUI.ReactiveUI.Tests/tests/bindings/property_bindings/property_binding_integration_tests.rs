//! `PropertyBindingIntegrationTests` (`ReactiveUI.Tests/Bindings/PropertyBindings/PropertyBindingIntegrationTests.cs`).
//!
//! The view and the view model are reactive objects ([`reactive_properties!`] and
//! [`reactive_object!`]); the view is typed for its view model ([`view_for!`]). The text
//! properties are `String`s where upstream's are nullable: an unset text is empty. The type
//! conversions use the built-in converters, which the tests register as ReactiveUI's builder does.

use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable, IObserver};
use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

const INITIAL_TEXT: &str = "Initial";

const CHANGED_TEXT: &str = "Changed";

const UPDATED_TEXT: &str = "Updated";

reactive_properties! {
    /// The view model of the tests (`TestViewModel`).
    #[derive(Default)]
    pub struct TestViewModel {
        reactive: ReactiveObject,
        #[reactive]
        count: i32,
        #[reactive]
        name: String,
    }
}
reactive_object!(TestViewModel);

reactive_properties! {
    /// The view of the tests (`TestView`).
    #[derive(Default)]
    pub struct TestView {
        reactive: ReactiveObject,
        #[reactive]
        name_text: String,
        view_model: RefCell<Option<Rc<TestViewModel>>>,
    }
}
reactive_object!(TestView);

impl TestView {
    pub fn view_model(&self) -> Option<Rc<TestViewModel>> {
        self.view_model.borrow().clone()
    }

    pub fn set_view_model(&self, view_model: Option<Rc<TestViewModel>>) {
        *self.view_model.borrow_mut() = view_model;
        IReactiveObject::raise_property_changed(self, "ViewModel");
    }
}

view_for!(TestView, TestViewModel);

/// A converter from `F` to `T` through a function, with an affinity that wins
/// (`FuncBindingTypeConverter<TFrom, TTo>`).
struct FuncBindingTypeConverter<F, T> {
    converter: Box<dyn Fn(&F) -> T>,
    _types: PhantomData<fn(F) -> T>,
}

impl<F: 'static, T: 'static> FuncBindingTypeConverter<F, T> {
    const HIGH_AFFINITY: i32 = 100;

    fn new(converter: impl Fn(&F) -> T + 'static) -> Self {
        Self { converter: Box::new(converter), _types: PhantomData }
    }
}

impl<F: 'static, T: 'static> BindingTypeConverter for FuncBindingTypeConverter<F, T> {
    type From = F;
    type To = T;

    fn affinity(&self) -> i32 {
        Self::HIGH_AFFINITY
    }

    fn convert(&self, from: Option<&F>, _conversion_hint: Option<&dyn Any>) -> Option<T> {
        from.map(|from| (self.converter)(from))
    }
}

fn view_model_named(name: &str) -> Rc<TestViewModel> {
    let view_model = Rc::new(TestViewModel::default());
    view_model.set_name(name.to_string());
    view_model
}

fn view_model_counting(count: i32) -> Rc<TestViewModel> {
    let view_model = Rc::new(TestViewModel::default());
    view_model.set_count(count);
    view_model
}

fn view_of(view_model: &Rc<TestViewModel>) -> Rc<TestView> {
    let view = Rc::new(TestView::default());
    view.set_view_model(Some(view_model.clone()));
    view
}

/// Registers the built-in converters in the current service, as ReactiveUI's builder does.
fn register_converters() {
    register_default_converters(&BindingConverters::current());
}

#[test]
fn bind_after_dispose_stops_updating() {
    let view_model = view_model_named(INITIAL_TEXT);
    let view = view_of(&view_model);

    let binding = view
        .bind(view_model.clone(), settable_prop!(TestViewModel => name), settable_prop!(TestView => name_text))
        .expect("no hook refuses the binding");
    assert_eq!(view.name_text(), INITIAL_TEXT);
    binding.dispose();

    view_model.set_name(CHANGED_TEXT.to_string());
    assert_eq!(view.name_text(), INITIAL_TEXT);
}

#[test]
fn bind_with_basic_properties_updates_both_directions() {
    let view_model = view_model_named(INITIAL_TEXT);
    let view = view_of(&view_model);

    let binding = view
        .bind(view_model.clone(), settable_prop!(TestViewModel => name), settable_prop!(TestView => name_text))
        .expect("no hook refuses the binding");

    // The view model to the view.
    assert_eq!(view.name_text(), INITIAL_TEXT);

    // The view to the view model.
    view.set_name_text(UPDATED_TEXT.to_string());
    assert_eq!(view_model.name(), UPDATED_TEXT);

    // The view model to the view again.
    view_model.set_name(CHANGED_TEXT.to_string());
    assert_eq!(view.name_text(), CHANGED_TEXT);
    binding.dispose();
}

#[test]
fn bind_with_converters_converts_values() {
    const UPDATED_COUNT: i32 = 100;
    let view_model = view_model_counting(42);
    let view = view_of(&view_model);

    let binding = view
        .bind_select(
            view_model.clone(),
            settable_prop!(TestViewModel => count),
            settable_prop!(TestView => name_text),
            |count: i32| format!("Count: {count}"),
            |text: String| text.replace("Count: ", "").parse().unwrap_or(0),
        )
        .expect("no hook refuses the binding");

    assert_eq!(view.name_text(), "Count: 42");

    view.set_name_text("Count: 100".to_string());
    assert_eq!(view_model.count(), UPDATED_COUNT);
    binding.dispose();
}

#[test]
fn bind_with_null_view_model_does_not_throw() {
    let view = Rc::new(TestView::default());

    let binding = view
        .bind(None::<Rc<TestViewModel>>, settable_prop!(TestViewModel => name), settable_prop!(TestView => name_text))
        .expect("no hook refuses the binding");

    // Nothing is bound, and nothing fails.
    assert!(view.name_text().is_empty());
    binding.dispose();
}

#[test]
fn bind_with_signal_view_update_updates_on_signal() {
    let view_model = view_model_named(INITIAL_TEXT);
    let view = view_of(&view_model);
    let signal: Rc<Subject<()>> = Subject::new();
    let updates: Rc<dyn IObservable<()>> = signal.as_observable();

    let binding = view
        .bind_with_trigger(view_model.clone(), settable_prop!(TestViewModel => name), settable_prop!(TestView => name_text), Some(updates), TriggerUpdate::ViewToViewModel)
        .expect("no hook refuses the binding");

    assert_eq!(view.name_text(), INITIAL_TEXT);

    // The view changes, without the signal.
    view.set_name_text(UPDATED_TEXT.to_string());
    assert_eq!(view_model.name(), INITIAL_TEXT);

    // The signal.
    signal.on_next(());
    assert_eq!(view_model.name(), UPDATED_TEXT);
    binding.dispose();
}

#[test]
fn bind_with_type_converter_converts_types() {
    const UPDATED_COUNT: i32 = 100;
    register_converters();
    let view_model = view_model_counting(42);
    let view = view_of(&view_model);

    let binding = view
        .bind(view_model.clone(), settable_prop!(TestViewModel => count), settable_prop!(TestView => name_text))
        .expect("no hook refuses the binding");

    // The number is converted to text.
    assert_eq!(view.name_text(), "42");

    view.set_name_text("100".to_string());
    assert_eq!(view_model.count(), UPDATED_COUNT);
    binding.dispose();
}

#[test]
fn bind_to_after_dispose_stops_updating() {
    let target = Rc::new(TestViewModel::default());
    let source: Rc<BehaviorSubject<String>> = BehaviorSubject::new(INITIAL_TEXT.to_string());

    let binding = source.as_observable().bind_to(target.clone(), settable_prop!(TestViewModel => name));
    assert_eq!(target.name(), INITIAL_TEXT);
    binding.dispose();

    source.on_next(UPDATED_TEXT.to_string());
    assert_eq!(target.name(), INITIAL_TEXT);
}

#[test]
fn bind_to_updates_target_property() {
    let target = Rc::new(TestViewModel::default());
    let source: Rc<BehaviorSubject<String>> = BehaviorSubject::new(INITIAL_TEXT.to_string());

    let binding = source.as_observable().bind_to(target.clone(), settable_prop!(TestViewModel => name));

    assert_eq!(target.name(), INITIAL_TEXT);

    source.on_next(UPDATED_TEXT.to_string());
    assert_eq!(target.name(), UPDATED_TEXT);

    source.on_next("Final".to_string());
    assert_eq!(target.name(), "Final");
    binding.dispose();
}

#[test]
fn bind_to_with_converter_converts_value() {
    const UPDATED_VALUE: i32 = 100;
    let target = Rc::new(TestViewModel::default());
    let source: Rc<BehaviorSubject<i32>> = BehaviorSubject::new(42);
    let converter: Rc<dyn IBindingTypeConverter> = Rc::new(FuncBindingTypeConverter::<i32, String>::new(|value| format!("Number: {value}")));

    let binding = source.as_observable().bind_to_with(target.clone(), settable_prop!(TestViewModel => name), None, Some(converter));

    assert_eq!(target.name(), "Number: 42");

    source.on_next(UPDATED_VALUE);
    assert_eq!(target.name(), "Number: 100");
    binding.dispose();
}

#[test]
fn one_way_bind_after_dispose_stops_updating() {
    let view_model = view_model_named(INITIAL_TEXT);
    let view = view_of(&view_model);

    let binding = view
        .one_way_bind(view_model.clone(), prop!(TestViewModel => name), settable_prop!(TestView => name_text))
        .expect("no hook refuses the binding");
    assert_eq!(view.name_text(), INITIAL_TEXT);
    binding.dispose();

    view_model.set_name(CHANGED_TEXT.to_string());
    assert_eq!(view.name_text(), INITIAL_TEXT);
}

#[test]
fn one_way_bind_updates_view_only() {
    let view_model = view_model_named(INITIAL_TEXT);
    let view = view_of(&view_model);

    let binding = view
        .one_way_bind(view_model.clone(), prop!(TestViewModel => name), settable_prop!(TestView => name_text))
        .expect("no hook refuses the binding");

    // The view model to the view.
    assert_eq!(view.name_text(), INITIAL_TEXT);

    view_model.set_name(UPDATED_TEXT.to_string());
    assert_eq!(view.name_text(), UPDATED_TEXT);

    // The view does not reach the view model.
    view.set_name_text("ViewChange".to_string());
    assert_eq!(view_model.name(), UPDATED_TEXT);
    binding.dispose();
}

#[test]
fn one_way_bind_with_null_view_model_does_not_throw() {
    let view = Rc::new(TestView::default());

    let binding = view
        .one_way_bind(None::<Rc<TestViewModel>>, prop!(TestViewModel => name), settable_prop!(TestView => name_text))
        .expect("no hook refuses the binding");

    assert!(view.name_text().is_empty());
    binding.dispose();
}

#[test]
fn one_way_bind_with_selector_transforms_value() {
    const UPDATED_COUNT: i32 = 100;
    let view_model = view_model_counting(42);
    let view = view_of(&view_model);

    let binding = view
        .one_way_bind_select(view_model.clone(), prop!(TestViewModel => count), settable_prop!(TestView => name_text), |count: i32| format!("The count is: {count}"))
        .expect("no hook refuses the binding");

    assert_eq!(view.name_text(), "The count is: 42");

    view_model.set_count(UPDATED_COUNT);
    assert_eq!(view.name_text(), "The count is: 100");
    binding.dispose();
}

#[test]
fn one_way_bind_with_type_converter_converts_types() {
    const UPDATED_COUNT: i32 = 100;
    register_converters();
    let view_model = view_model_counting(42);
    let view = view_of(&view_model);

    let binding = view
        .one_way_bind(view_model.clone(), prop!(TestViewModel => count), settable_prop!(TestView => name_text))
        .expect("no hook refuses the binding");

    assert_eq!(view.name_text(), "42");

    view_model.set_count(UPDATED_COUNT);
    assert_eq!(view.name_text(), "100");
    binding.dispose();
}
