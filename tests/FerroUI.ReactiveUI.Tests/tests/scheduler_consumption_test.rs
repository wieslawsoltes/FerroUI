//! `SchedulerConsumptionTest` (`ReactiveUI.Tests/SchedulerConsumptionTest.cs`): code that uses
//! `RxSchedulers`.

use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_reactiveui::*;
use ferroui_system::{prop, IDisposable, IObservable, IObserver};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// A repository that processes its data on the background scheduler (`ExampleRepository`).
struct ExampleRepository {
    data_subject: Rc<Subject<String>>,
}

impl ExampleRepository {
    fn new() -> Self {
        Self { data_subject: Subject::new() }
    }

    fn get_data(&self) -> Rc<dyn IObservable<String>> {
        self.data_subject
            .as_observable()
            .observe_on(RxSchedulers::task_pool_scheduler())
            .select(|data: String| format!("Processed: {data}"))
    }

    fn publish_data(&self, data: &str) {
        self.data_subject.on_next(data.to_owned());
    }
}

impl IDisposable for ExampleRepository {
    fn dispose(&self) {
        self.data_subject.dispose();
    }
}

reactive_properties! {
    /// A view model whose greeting derives from its name on the main thread scheduler
    /// (`ExampleViewModel`).
    #[derive(Default)]
    struct ExampleViewModel {
        reactive: ReactiveObject,
        #[reactive]
        name: Option<String>,
        greeting: OnceCell<ObservableAsPropertyHelper<String>>,
    }
}
reactive_object!(ExampleViewModel);

impl ExampleViewModel {
    fn new() -> Rc<Self> {
        let view_model = Rc::new(Self::default());
        // Upstream names the property with `nameof(Greeting)`; the path names it here.
        let greeting = view_model
            .when_any_value(prop!(ExampleViewModel => name))
            .select(|name: Option<String>| format!("Hello, {}!", name.unwrap_or_else(|| "World".to_owned())))
            .observe_on(RxSchedulers::main_thread_scheduler())
            .to_property_with(&view_model, prop!(ExampleViewModel => greeting), ObservableAsPropertyInitialValue::default(), false, Some(RxSchedulers::main_thread_scheduler()));
        let _ = view_model.greeting.set(greeting);
        view_model
    }

    #[allow(dead_code)]
    fn greeting(&self) -> String {
        self.greeting.get().map(|helper| helper.value()).unwrap_or_default()
    }
}

// Not ported: ReactivePropertyFactoryMethodsWork - needs ReactiveProperty (reactive properties, a later wave)

#[test]
fn repository_can_use_schedulers_without_attributes() {
    let test_scheduler = TestScheduler::new();
    let original_scheduler = RxSchedulers::task_pool_scheduler();

    // Use test scheduler for predictable behavior
    RxSchedulers::set_task_pool_scheduler(test_scheduler.scheduler());

    let repository = ExampleRepository::new();
    let result = Rc::new(RefCell::new(None::<String>));

    let sink = result.clone();
    let subscription = repository.get_data().subscribe_next(move |data| *sink.borrow_mut() = Some(data));
    repository.publish_data("test");

    // Advance the test scheduler to process the observable
    test_scheduler.advance_by(duration_to_ticks(Duration::from_millis(1)));

    assert_eq!(result.borrow().as_deref(), Some("Processed: test"));
    subscription.dispose();
    repository.dispose();

    // Restore original scheduler
    RxSchedulers::set_task_pool_scheduler(original_scheduler);
}

#[test]
fn view_model_can_use_schedulers_without_attributes() {
    let test_scheduler = TestScheduler::new();
    let original_scheduler = RxSchedulers::main_thread_scheduler();

    // Use test scheduler for predictable behavior
    RxSchedulers::set_main_thread_scheduler(test_scheduler.scheduler());

    let view_model = ExampleViewModel::new();
    view_model.set_name(Some("ReactiveUI".to_owned()));

    // Advance the test scheduler to process the observable
    test_scheduler.advance_by(duration_to_ticks(Duration::from_millis(1)));

    // For this test, we're primarily verifying that the code compiles and runs.
    assert_eq!(view_model.name().as_deref(), Some("ReactiveUI"));

    // Restore original scheduler
    RxSchedulers::set_main_thread_scheduler(original_scheduler);
}
