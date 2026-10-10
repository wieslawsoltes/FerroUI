//! Port of `ViewModels/MainWindowViewModel.cs`.

use super::random::Random;
use super::{DataAnnotationsErrorViewModel, ExceptionErrorViewModel, IndeiErrorViewModel, NestedCommandViewModel};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::plugins::ObservableValue;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, Observable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::selection::{ISelectionModel, SelectionModel};
use mini_mvvm::{MiniCommand, ViewModelBase};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

ferroui_controls::ferro_markup_list!(pub TestItemOfStringList: Rc<TestItem<String>>);

pub struct MainWindowViewModel {
    base: ViewModelBase,
    boolean_string: RefCell<String>,
    double_value: Cell<f64>,
    string_value: RefCell<Option<String>>,
    boolean_flag: Cell<bool>,
    current_time: RefCell<Option<String>>,
    nested: RefCell<Option<Rc<NestedCommandViewModel>>>,
    items: Rc<BindableList<Rc<TestItem<String>>>>,
    selection: Rc<SelectionModel<Rc<TestItem<String>>>>,
    shuffle_items: Rc<MiniCommand>,
    current_time_observable: Rc<dyn IObservable<DateTimeOffset>>,
    string_value_command: Rc<MiniCommand>,
    data_annotations_validation: Rc<DataAnnotationsErrorViewModel>,
    exception_data_validation: Rc<ExceptionErrorViewModel>,
    indei_data_validation: Rc<IndeiErrorViewModel>,
    /// The background thread that sets `CurrentTime`; it ends with the view model.
    current_time_thread: OnceCell<CurrentTimeThread>,
}

impl PartialEq for MainWindowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for MainWindowViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl MainWindowViewModel {
    pub fn new() -> Rc<MainWindowViewModel> {
        let this = Rc::new_cyclic(|this: &Weak<MainWindowViewModel>| {
            let items = BindableList::new((0..20).map(|x| {
                let item = TestItem::<String>::new();
                item.set_value(Some(format!("Item {x}")));
                item.set_detail(Some(format!("Item {x} details")));
                item
            }));

            let selection = SelectionModel::<Rc<TestItem<String>>>::new();
            selection.set_single_select(false);

            let shuffle_items = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    let mut r = Random::new();
                    let items = this.items.items();
                    items.move_item(r.next_max(items.count() as i32) as usize, 1);
                })
            };

            let string_value_command = {
                let this = this.clone();
                MiniCommand::create_with::<BoxedValue>(move |param| {
                    let Some(this) = this.upgrade() else { return };
                    this.set_boolean_flag(!this.boolean_flag());
                    this.set_string_value(Some(ValueTypes::to_display_string(Some(&param))));
                    let nested = this.nested.borrow().clone();
                    this.set_nested_model(Some(nested.unwrap_or_else(NestedCommandViewModel::new)));
                })
            };

            let current_time_observable =
                timer(Duration::ZERO, Duration::from_secs(1)).select(|_: i64| DateTimeOffset::now());

            Self {
                base: ViewModelBase::new(),
                boolean_string: RefCell::new(String::from("True")),
                double_value: Cell::new(5.0),
                string_value: RefCell::new(Some(String::from("Simple Binding"))),
                boolean_flag: Cell::new(false),
                current_time: RefCell::new(None),
                nested: RefCell::new(None),
                items,
                selection,
                shuffle_items,
                current_time_observable,
                string_value_command,
                data_annotations_validation: DataAnnotationsErrorViewModel::new(),
                exception_data_validation: ExceptionErrorViewModel::new(),
                indei_data_validation: IndeiErrorViewModel::new(),
                current_time_thread: OnceCell::new(),
            }
        });

        // `Task.Run(() => { while (true) { CurrentTime = DateTimeOffset.Now.ToString(); Thread.Sleep(1000); } })`.
        if this.current_time_thread.set(CurrentTimeThread::start(&this)).is_err() {
            unreachable!("the background thread is started once");
        }

        this
    }

    pub fn items(&self) -> Rc<BindableList<Rc<TestItem<String>>>> {
        self.items.clone()
    }

    pub fn selection(&self) -> Rc<SelectionModel<Rc<TestItem<String>>>> {
        self.selection.clone()
    }

    pub fn shuffle_items(&self) -> Rc<MiniCommand> {
        self.shuffle_items.clone()
    }

    pub fn boolean_string(&self) -> String {
        self.boolean_string.borrow().clone()
    }

    pub fn set_boolean_string(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.boolean_string, value, "BooleanString");
    }

    pub fn double_value(&self) -> f64 {
        self.double_value.get()
    }

    pub fn set_double_value(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.double_value, value, "DoubleValue");
    }

    pub fn string_value(&self) -> Option<String> {
        self.string_value.borrow().clone()
    }

    pub fn set_string_value(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.string_value, value, "StringValue");
    }

    pub fn boolean_flag(&self) -> bool {
        self.boolean_flag.get()
    }

    pub fn set_boolean_flag(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.boolean_flag, value, "BooleanFlag");
    }

    pub fn current_time(&self) -> Option<String> {
        self.current_time.borrow().clone()
    }

    fn set_current_time(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.current_time, value, "CurrentTime");
    }

    pub fn current_time_observable(&self) -> Rc<dyn IObservable<DateTimeOffset>> {
        self.current_time_observable.clone()
    }

    pub fn string_value_command(&self) -> Rc<MiniCommand> {
        self.string_value_command.clone()
    }

    pub fn data_annotations_validation(&self) -> Rc<DataAnnotationsErrorViewModel> {
        self.data_annotations_validation.clone()
    }

    pub fn exception_data_validation(&self) -> Rc<ExceptionErrorViewModel> {
        self.exception_data_validation.clone()
    }

    pub fn indei_data_validation(&self) -> Rc<IndeiErrorViewModel> {
        self.indei_data_validation.clone()
    }

    pub fn nested_model(&self) -> Option<Rc<NestedCommandViewModel>> {
        self.nested.borrow().clone()
    }

    fn set_nested_model(&self, value: Option<Rc<NestedCommandViewModel>>) {
        self.base.raise_and_set_if_changed(&self.nested, value, "NestedModel");
    }

    pub fn do_(&self, _parameter: &Option<BoxedValue>) {}

    // `[DependsOn(nameof(BooleanFlag))]`: stated in the markup metadata of the method.
    fn can_do(&self, _parameter: &Option<BoxedValue>) -> bool {
        self.boolean_flag()
    }
}

ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [MainWindowViewModel, Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    constructors: [() => MainWindowViewModel::new],
    properties: [
        // The typed list of the items: an items source, and what `Items[1].Value` indexes.
        Items: FerroList<Rc<TestItem<String>>> { get: |this: &Rc<MainWindowViewModel>| this.items().items().clone() },
        Selection: Rc<dyn ISelectionModel> {
            get: |this: &Rc<MainWindowViewModel>| this.selection() as Rc<dyn ISelectionModel>
        },
        ShuffleItems: Rc<dyn ICommand> { get: |this: &Rc<MainWindowViewModel>| this.shuffle_items().as_command() },
        BooleanString: String {
            get: |this: &Rc<MainWindowViewModel>| this.boolean_string(),
            set: |this: &Rc<MainWindowViewModel>, value: String| this.set_boolean_string(value)
        },
        DoubleValue: f64 {
            get: |this: &Rc<MainWindowViewModel>| this.double_value(),
            set: |this: &Rc<MainWindowViewModel>, value: f64| this.set_double_value(value)
        },
        StringValue: Option<String> {
            get: |this: &Rc<MainWindowViewModel>| this.string_value(),
            set: |this: &Rc<MainWindowViewModel>, value: Option<String>| this.set_string_value(value)
        },
        BooleanFlag: bool {
            get: |this: &Rc<MainWindowViewModel>| this.boolean_flag(),
            set: |this: &Rc<MainWindowViewModel>, value: bool| this.set_boolean_flag(value)
        },
        CurrentTime: Option<String> { get: |this: &Rc<MainWindowViewModel>| this.current_time() },
        CurrentTimeObservable: ObservableValue {
            get: |this: &Rc<MainWindowViewModel>| ObservableValue::new(this.current_time_observable())
        },
        StringValueCommand: Rc<dyn ICommand> {
            get: |this: &Rc<MainWindowViewModel>| this.string_value_command().as_command()
        },
        DataAnnotationsValidation: Rc<DataAnnotationsErrorViewModel> {
            get: |this: &Rc<MainWindowViewModel>| this.data_annotations_validation()
        },
        ExceptionDataValidation: Rc<ExceptionErrorViewModel> {
            get: |this: &Rc<MainWindowViewModel>| this.exception_data_validation()
        },
        IndeiDataValidation: Rc<IndeiErrorViewModel> {
            get: |this: &Rc<MainWindowViewModel>| this.indei_data_validation()
        },
        NestedModel: Option<Rc<NestedCommandViewModel>> { get: |this: &Rc<MainWindowViewModel>| this.nested_model() },
    ],
    methods: [
        fn Do(Option<BoxedValue>) =>
            |this: &Rc<MainWindowViewModel>, parameter: Option<BoxedValue>| this.do_(&parameter),
        fn CanDo(Option<BoxedValue>) -> bool =>
            (|this: &Rc<MainWindowViewModel>, parameter: Option<BoxedValue>| this.can_do(&parameter))
            [DependsOn("BooleanFlag")],
    ],
    notify_property_changed: MainWindowViewModel,
});

// --- the background thread ------------------------------------------------------------------

thread_local! {
    /// The view models of this thread whose `CurrentTime` a background thread sets, by the
    /// number of each.
    static CURRENT_TIME_TARGETS: RefCell<HashMap<u64, Weak<MainWindowViewModel>>> = RefCell::new(HashMap::new());
}

/// The number of the next view model with a background thread.
static NEXT_CURRENT_TIME_TARGET: AtomicU64 = AtomicU64::new(0);

/// The background thread of a view model.
///
/// The managed original sets the property on the thread of the task, and a binding to the
/// property takes the change notification over to the thread of its target. A view model of
/// the port lives on the thread that created it (it is not `Send`), so the background thread
/// reads the clock and formats the time, and posts the text to the dispatcher of the view
/// model, where the property is set (`GAPS.md`, the background thread). The thread ends when the view model is
/// dropped; the one of the managed original runs until the process exits.
struct CurrentTimeThread {
    target: u64,
    running: Arc<AtomicBool>,
}

impl CurrentTimeThread {
    fn start(view_model: &Rc<MainWindowViewModel>) -> CurrentTimeThread {
        let target = NEXT_CURRENT_TIME_TARGET.fetch_add(1, Ordering::Relaxed);
        CURRENT_TIME_TARGETS.with(|targets| targets.borrow_mut().insert(target, Rc::downgrade(view_model)));
        let running = Arc::new(AtomicBool::new(true));
        let dispatcher = Dispatcher::current_dispatcher();
        {
            let running = running.clone();
            std::thread::spawn(move || {
                while running.load(Ordering::Acquire) {
                    let current_time = DateTimeOffset::now().to_string();
                    dispatcher.post(move || Self::set_current_time(target, current_time), DispatcherPriority::DEFAULT);
                    std::thread::sleep(Duration::from_millis(1000));
                }
            });
        }
        CurrentTimeThread { target, running }
    }

    /// Sets `CurrentTime` of the view model with the number `target`; runs on its thread.
    fn set_current_time(target: u64, current_time: String) {
        let view_model = CURRENT_TIME_TARGETS.with(|targets| targets.borrow().get(&target).and_then(Weak::upgrade));
        if let Some(view_model) = view_model {
            view_model.set_current_time(Some(current_time));
        }
    }
}

impl Drop for CurrentTimeThread {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        // The table is gone when the thread of the view model is ending.
        let _ = CURRENT_TIME_TARGETS.try_with(|targets| targets.borrow_mut().remove(&self.target));
    }
}

// --- the timer of the reactive library ------------------------------------------------------

/// `Observable.Timer(dueTime, period)` of the reactive library the managed original
/// references: a number after `due_time` and then one every `period`, counted from zero, for
/// each subscription.
///
/// The operator of the reactive library ticks on a thread of the thread pool, and a binding
/// takes the values over to the thread of its target; here the ticks are the ticks of
/// dispatcher timers (`GAPS.md`, the timer of the reactive library).
fn timer(due_time: Duration, period: Duration) -> Rc<dyn IObservable<i64>> {
    Observable::create(move |observer: Rc<dyn IObserver<i64>>| {
        let periodic: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let first = {
            let periodic = periodic.clone();
            DispatcherTimer::run_once(
                move || {
                    observer.on_next(0);
                    let count = Cell::new(0_i64);
                    let observer = observer.clone();
                    let ticks = DispatcherTimer::run(
                        move || {
                            count.set(count.get() + 1);
                            observer.on_next(count.get());
                            true
                        },
                        period,
                        DispatcherPriority::DEFAULT,
                    );
                    *periodic.borrow_mut() = Some(ticks);
                },
                due_time,
                DispatcherPriority::DEFAULT,
            )
        };
        Disposable::create(move || {
            first.dispose();
            let ticks = periodic.borrow_mut().take();
            if let Some(ticks) = ticks {
                ticks.dispose();
            }
        })
    })
}

// --- the nested class -----------------------------------------------------------------------

/// The nested class `MainWindowViewModel.TestItem<T>` ("just so we can test it in XAML"):
/// markup names it `vm:MainWindowViewModel+TestItem` with `x:TypeArguments`.
///
/// The managed original is an open generic class; markup knows the instantiations a crate
/// declares (docs/porting/DEVIATIONS.md, "Run-time type system of markup"). The one the
/// documents of the sample name is declared below ([`TestItemOfString`]).
pub struct TestItem<T> {
    base: ViewModelBase,
    value: RefCell<Option<T>>,
    detail: RefCell<Option<String>>,
}

impl<T> PartialEq for TestItem<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<T> INotifyPropertyChanged for TestItem<T> {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl<T: Clone + PartialEq + 'static> TestItem<T> {
    pub fn new() -> Rc<TestItem<T>> {
        Rc::new(Self { base: ViewModelBase::new(), value: RefCell::new(None), detail: RefCell::new(None) })
    }

    pub fn value(&self) -> Option<T> {
        self.value.borrow().clone()
    }

    pub fn set_value(&self, value: Option<T>) {
        self.base.raise_and_set_if_changed(&self.value, value, "Value");
    }

    pub fn detail(&self) -> Option<String> {
        self.detail.borrow().clone()
    }

    pub fn set_detail(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.detail, value, "Detail");
    }
}

/// Declares an instantiation of [`TestItem`] to markup: `$carrier` carries the markup
/// metadata of `TestItem<$value>`, under the name of the nested class.
macro_rules! test_item {
    ($carrier:ident: $value:ty) => {
        /// Carries the markup metadata of an instantiation of the nested generic class.
        pub struct $carrier;

        ferro_markup_type!(class $carrier as "MainWindowViewModel+TestItem`1" {
            this: Rc<TestItem<$value>>,
            handles: [TestItem<$value>, Rc<TestItem<$value>>, Option<Rc<TestItem<$value>>>],
            generic: "MainWindowViewModel+TestItem`1" [$value],
            constructors: [() => TestItem::<$value>::new],
            properties: [
                Value: Option<$value> {
                    get: |this: &Rc<TestItem<$value>>| this.value(),
                    set: |this: &Rc<TestItem<$value>>, value: Option<$value>| this.set_value(value)
                },
                Detail: Option<String> {
                    get: |this: &Rc<TestItem<$value>>| this.detail(),
                    set: |this: &Rc<TestItem<$value>>, value: Option<String>| this.set_detail(value)
                },
            ],
            notify_property_changed: TestItem<$value>,
        });
    };
}

// `{x:Type vm:MainWindowViewModel+TestItem, x:TypeArguments=x:String}` of the documents.
test_item!(TestItemOfString: String);

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_controls::ItemsSource;

    fn value_of(view_model: &MainWindowViewModel, index: usize) -> Option<String> {
        view_model.items().items().get(index).value()
    }

    #[test]
    fn the_view_model_starts_with_twenty_items_and_a_multiple_selection() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let view_model = MainWindowViewModel::new();
        assert_eq!(20, view_model.items().items().count());
        assert_eq!(Some(String::from("Item 0")), value_of(&view_model, 0));
        assert_eq!(Some(String::from("Item 19 details")), view_model.items().items().get(19).detail());
        assert!(!view_model.selection().single_select());
        assert_eq!("True", view_model.boolean_string());
        assert_eq!(5.0, view_model.double_value());
        assert_eq!(Some(String::from("Simple Binding")), view_model.string_value());
        assert!(!view_model.boolean_flag());
        assert!(view_model.nested_model().is_none());
    }

    #[test]
    fn shuffle_moves_an_item_to_the_second_place() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let view_model = MainWindowViewModel::new();
        view_model.selection().set_source(Some(ItemsSource::from(view_model.items())));
        let before: Vec<Option<String>> = (0..20).map(|index| value_of(&view_model, index)).collect();
        view_model.shuffle_items().execute(None);
        let after: Vec<Option<String>> = (0..20).map(|index| value_of(&view_model, index)).collect();

        // The same items, and the one that moved is the second.
        let mut sorted = (before.clone(), after.clone());
        sorted.0.sort();
        sorted.1.sort();
        assert_eq!(sorted.0, sorted.1);
        let moved = after[1].clone();
        let from = before.iter().position(|value| *value == moved).expect("the moved item");
        let mut expected = before.clone();
        let item = expected.remove(from);
        expected.insert(1, item);
        assert_eq!(expected, after);
    }

    #[test]
    fn the_string_value_command_toggles_the_flag_and_creates_the_nested_model_once() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let view_model = MainWindowViewModel::new();
        let parameter: BoxedValue = Rc::new(String::from("Button"));
        view_model.string_value_command().execute(Some(&parameter));
        assert!(view_model.boolean_flag());
        assert!(view_model.can_do(&None));
        assert_eq!(Some(String::from("Button")), view_model.string_value());
        let nested = view_model.nested_model().expect("the nested view model");

        let parameter: BoxedValue = Rc::new(String::from("CheckBox"));
        view_model.string_value_command().execute(Some(&parameter));
        assert!(!view_model.boolean_flag());
        assert_eq!(Some(String::from("CheckBox")), view_model.string_value());
        assert!(Rc::ptr_eq(&nested, &view_model.nested_model().expect("the nested view model")));
    }

    #[test]
    fn a_test_item_notifies_its_changes() {
        let item = TestItem::<String>::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        item.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));
        item.set_value(Some(String::from("shared")));
        item.set_value(Some(String::from("shared")));
        item.set_detail(Some(String::from("details")));
        assert_eq!(vec!["Value".to_string(), "Detail".to_string()], *seen.borrow());
    }
}
