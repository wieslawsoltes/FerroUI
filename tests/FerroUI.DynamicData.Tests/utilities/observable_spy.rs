//! `ObservableSpy` (`Utilities/ObservableSpy.cs`): logs every event of a sequence, its
//! subscriptions and disposals, with the thread and an optional time stamp.
//!
//! Not ported: `TestSpy` (it writes to xUnit's `ITestOutputHelper`; `spy` with a logger does the
//! same) and the output to the debugger of `DebugSpy` (here it writes to the standard error in
//! debug builds, where the test harness captures it).

use ferroui_dynamic_data::cache::{self, ChangeReason, ObservableCacheEx};
use ferroui_dynamic_data::list::{self, ItemChange, ListChangeReason, ObservableListEx, RangeChange};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;
use std::time::SystemTime;

const CHANGE_SET_ENTRY_SPACING: &str = "\n\t";

/// The options of a spy (the optional parameters of `Spy`).
pub struct SpyOptions<T> {
    /// The text of every message; the type of the sequence when `None` (`infoText`, `opName`).
    pub info_text: Option<String>,
    /// Where the messages go; the standard output when `None` (`logger`, `Console.WriteLine`).
    pub logger: Option<Rc<dyn Fn(&str)>>,
    /// The text of a value; its `Debug` form when `None` (`formatter`, `ToString`).
    pub formatter: Option<Rc<dyn Fn(&T) -> String>>,
    /// Whether the messages of subscriptions are logged (`showSubs`).
    pub show_subs: bool,
    /// Whether the messages start with the time (`showTimestamps`).
    pub show_timestamps: bool,
}

impl<T> Default for SpyOptions<T> {
    fn default() -> Self {
        Self { info_text: None, logger: None, formatter: None, show_subs: true, show_timestamps: true }
    }
}

impl<T> SpyOptions<T> {
    /// The default options with the text `info_text`.
    pub fn named(info_text: impl Into<String>) -> Self {
        Self { info_text: Some(info_text.into()), ..Self::default() }
    }
}

/// `HH:mm:ss.fffffff` of the current time.
fn high_res_timestamp() -> String {
    let since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let seconds = since_epoch.as_secs() % 86_400;
    let ticks = since_epoch.subsec_nanos() / 100;
    format!("{:02}:{:02}:{:02}.{:07} ", seconds / 3600, seconds / 60 % 60, seconds % 60, ticks)
}

/// The logger of a spy (`CreateLogger`): the time, the thread and the text, then the message.
fn create_logger(base_logger: Rc<dyn Fn(&str)>, show_timestamps: bool, op_name: String) -> Rc<dyn Fn(&str)> {
    Rc::new(move |message: &str| {
        let time_stamp = if show_timestamps { high_res_timestamp() } else { String::new() };
        base_logger(&format!("{time_stamp}[{:?}] |{op_name}| {message}", std::thread::current().id()));
    })
}

fn default_formatter<T: fmt::Debug>() -> Rc<dyn Fn(&T) -> String> {
    Rc::new(|value: &T| format!("{value:?}"))
}

/// `Spy<T>`: `options.formatter` is not read; `formatter` formats the values.
fn spy_core<T: 'static>(source: Rc<dyn IObservable<T>>, options: SpyOptions<T>, formatter: Rc<dyn Fn(&T) -> String>) -> Rc<dyn IObservable<T>> {
    let active_subscription_counter = Rc::new(Cell::new(0usize));
    let subscription_counter = Rc::new(Cell::new(0usize));
    let base_logger = options.logger.unwrap_or_else(|| Rc::new(|message: &str| println!("{message}")));
    let op_name = options.info_text.unwrap_or_else(|| format!("IObservable<{}>", std::any::type_name::<T>()));
    let logger = create_logger(base_logger, options.show_timestamps, op_name);
    let sub_logger = options.show_subs.then(|| logger.clone());
    logger("Creating Observable");
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        subscription_counter.set(subscription_counter.get() + 1);
        let sub_id = subscription_counter.get();
        let value_counter = Rc::new(Cell::new(0usize));
        let completed_successfully: Rc<Cell<Option<bool>>> = Rc::new(Cell::new(None));
        if let Some(sub_logger) = &sub_logger {
            sub_logger(&format!("Creating Subscription #{sub_id}"));
        }
        let (next_logger, error_logger, completed_logger) = (logger.clone(), logger.clone(), logger.clone());
        let (next_counter, error_counter, completed_counter) = (value_counter.clone(), value_counter.clone(), value_counter);
        let (error_flag, completed_flag) = (completed_successfully.clone(), completed_successfully.clone());
        let formatter = formatter.clone();
        let (on_next_target, on_error_target, on_completed_target) = (observer.clone(), observer.clone(), observer);
        let subscription = source
            .do_all(
                move |value: &T| {
                    next_counter.set(next_counter.get() + 1);
                    next_logger(&format!("OnNext() [SubId:{sub_id}] (#{}): {}", next_counter.get(), formatter(value)));
                },
                move |error: ObservableError| {
                    error_logger(&format!("OnError() [SubId:{sub_id}] ({} Values) [Exception: {error}]", error_counter.get()));
                    error_flag.set(Some(false));
                },
                move || {
                    completed_logger(&format!("OnCompleted() [SubId:{sub_id}] ({} Values)", completed_counter.get()));
                    completed_flag.set(Some(true));
                },
            )
            .subscribe_all(
                // Upstream logs an exception downstream and rethrows it; a panic downstream unwinds
                // through the spy as it is.
                move |value: T| on_next_target.on_next(value),
                move |error| on_error_target.on_error(error),
                move || on_completed_target.on_completed(),
            );
        active_subscription_counter.set(active_subscription_counter.get() + 1);
        if let Some(sub_logger) = &sub_logger {
            sub_logger(&format!("Subscription Id #{sub_id} Created!  ({} Active Subscriptions)", active_subscription_counter.get()));
        }
        let (sub_logger, active) = (sub_logger.clone(), active_subscription_counter.clone());
        Disposable::create(move || {
            if let Some(sub_logger) = &sub_logger {
                match completed_successfully.get() {
                    Some(true) => sub_logger(&format!("Disposing SubId #{sub_id} due to OnComplete")),
                    Some(false) => sub_logger(&format!("Disposing SubId #{sub_id} due to OnError")),
                    None => sub_logger(&format!("Disposing SubId #{sub_id} due to Unsubscribe")),
                }
            }
            subscription.dispose();
            active.set(active.get().saturating_sub(1));
            if let Some(sub_logger) = &sub_logger {
                sub_logger(&format!("Dispose Completed! ({} Active Subscriptions)", active.get()));
            }
        })
    })
}

fn format_cache_change<T, K>(formatter: &dyn Fn(&T) -> String, change: &cache::Change<T, K>) -> String {
    let details = match (change.reason(), change.previous()) {
        (ChangeReason::Update, Some(previous)) => format!("{} [Previous: {}]", formatter(change.current()), formatter(previous)),
        _ => formatter(change.current()),
    };
    format!("[{:?}] {details}", change.reason())
}

fn format_range_change<T>(formatter: &dyn Fn(&T) -> String, range: &RangeChange<T>) -> String {
    let index = range.index().map(|index| format!("[Index: {index}] ")).unwrap_or_default();
    format!("({} Values) {index}{}", range.count(), range.iter().map(formatter).collect::<Vec<_>>().join(", "))
}

fn format_item_change<T>(formatter: &dyn Fn(&T) -> String, item: &ItemChange<T>) -> String {
    let indexes = match (item.current_index(), item.previous_index()) {
        (None, None) => String::new(),
        (Some(current), previous) if previous.is_none() || previous == Some(current) => format!(" [Index: {current}]"),
        (current, previous) => format!(" [Index: {}, Prev: {}]", current.map_or(-1, |index| index as i64), previous.map_or(-1, |index| index as i64)),
    };
    let previous = item.previous().map(|previous| format!(" [Previous: {}]", formatter(previous))).unwrap_or_default();
    format!("{}{indexes}{previous}", formatter(item.current()))
}

fn format_list_change<T>(formatter: &dyn Fn(&T) -> String, change: &list::Change<T>) -> String {
    let details = match change.reason() {
        ListChangeReason::AddRange | ListChangeReason::RemoveRange => format_range_change(formatter, change.range()),
        _ => match change.try_item() {
            Some(item) => format_item_change(formatter, item),
            None => format_range_change(formatter, change.range()),
        },
    };
    format!("[{:?}] {details}", change.reason())
}

fn cache_change_set_formatter<T: 'static, K: 'static>(formatter: Rc<dyn Fn(&T) -> String>) -> Rc<dyn Fn(&cache::ChangeSet<T, K>) -> String> {
    Rc::new(move |changes: &cache::ChangeSet<T, K>| {
        let entries: Vec<String> = changes.iter().enumerate().map(|(n, change)| format!("#{n} {}", format_cache_change(&*formatter, change))).collect();
        format!("[Cache Change Set]{CHANGE_SET_ENTRY_SPACING}{}", entries.join(CHANGE_SET_ENTRY_SPACING))
    })
}

fn list_change_set_formatter<T: 'static>(formatter: Rc<dyn Fn(&T) -> String>) -> Rc<dyn Fn(&list::ChangeSet<T>) -> String> {
    Rc::new(move |changes: &list::ChangeSet<T>| {
        let entries: Vec<String> = changes.iter().enumerate().map(|(n, change)| format!("#{n} {}", format_list_change(&*formatter, change))).collect();
        format!("[List Change Set]{CHANGE_SET_ENTRY_SPACING}{}", entries.join(CHANGE_SET_ENTRY_SPACING))
    })
}

/// The options of a change set from the options of its items, whose formatter the caller has taken.
fn change_set_options<T, C>(options: SpyOptions<T>) -> SpyOptions<C> {
    SpyOptions { info_text: options.info_text, logger: options.logger, formatter: None, show_subs: options.show_subs, show_timestamps: options.show_timestamps }
}

/// The spies of every sequence (`ObservableSpy.Spy<T>`, `DebugSpy<T>`).
pub trait ObservableSpy<T: 'static> {
    /// The sequence, logging its events (`Spy(infoText, logger, formatter, showSubs,
    /// showTimestamps)`).
    fn spy(&self, options: SpyOptions<T>) -> Rc<dyn IObservable<T>>;

    /// [`spy`](Self::spy) to the standard error in debug builds; the sequence itself otherwise
    /// (`DebugSpy`).
    fn debug_spy(&self, op_name: &str) -> Rc<dyn IObservable<T>>;
}

impl<T: fmt::Debug + 'static> ObservableSpy<T> for Rc<dyn IObservable<T>> {
    fn spy(&self, mut options: SpyOptions<T>) -> Rc<dyn IObservable<T>> {
        let formatter = options.formatter.take().unwrap_or_else(default_formatter);
        spy_core(self.clone(), options, formatter)
    }

    fn debug_spy(&self, op_name: &str) -> Rc<dyn IObservable<T>> {
        if cfg!(debug_assertions) {
            self.spy(SpyOptions { logger: Some(Rc::new(|message: &str| eprintln!("{message}"))), ..SpyOptions::named(op_name) })
        } else {
            self.clone()
        }
    }
}

/// The spies of the change sets of caches (`ObservableSpy.Spy<T, TKey>`, `DebugSpy<T, TKey>`): each
/// change on a line of its own.
pub trait CacheObservableSpy<T: 'static, K: 'static> {
    /// The change sets, logging their events (`Spy(IObservable<IChangeSet<T, TKey>>, ..)`).
    fn spy_cache(&self, options: SpyOptions<T>) -> Rc<dyn IObservable<cache::ChangeSet<T, K>>>;

    /// [`spy_cache`](Self::spy_cache) to the standard error in debug builds (`DebugSpy`).
    fn debug_spy_cache(&self, op_name: &str) -> Rc<dyn IObservable<cache::ChangeSet<T, K>>>;
}

impl<T: Clone + fmt::Debug + 'static, K: Eq + Hash + Clone + 'static, C: cache::IChangeSet<T, K>> CacheObservableSpy<T, K> for Rc<dyn IObservable<C>> {
    fn spy_cache(&self, mut options: SpyOptions<T>) -> Rc<dyn IObservable<cache::ChangeSet<T, K>>> {
        let formatter = cache_change_set_formatter::<T, K>(options.formatter.take().unwrap_or_else(default_formatter));
        let source = <Self as ObservableCacheEx<T, K>>::change_sets(self);
        let options = change_set_options(options);
        spy_core(source, options, formatter)
    }

    fn debug_spy_cache(&self, op_name: &str) -> Rc<dyn IObservable<cache::ChangeSet<T, K>>> {
        if cfg!(debug_assertions) {
            self.spy_cache(SpyOptions { logger: Some(Rc::new(|message: &str| eprintln!("{message}"))), ..SpyOptions::named(op_name) })
        } else {
            <Self as ObservableCacheEx<T, K>>::change_sets(self)
        }
    }
}

/// The spies of the change sets of lists (`ObservableSpy.Spy<T>` of `IChangeSet<T>`,
/// `DebugSpy<T>`): each change on a line of its own.
pub trait ListObservableSpy<T: 'static> {
    /// The change sets, logging their events (`Spy(IObservable<IChangeSet<T>>, ..)`).
    fn spy_list(&self, options: SpyOptions<T>) -> Rc<dyn IObservable<list::ChangeSet<T>>>;

    /// [`spy_list`](Self::spy_list) to the standard error in debug builds (`DebugSpy`).
    fn debug_spy_list(&self, op_name: &str) -> Rc<dyn IObservable<list::ChangeSet<T>>>;
}

impl<T: Clone + PartialEq + fmt::Debug + 'static, C: list::IChangeSet<T>> ListObservableSpy<T> for Rc<dyn IObservable<C>> {
    fn spy_list(&self, mut options: SpyOptions<T>) -> Rc<dyn IObservable<list::ChangeSet<T>>> {
        let formatter = list_change_set_formatter::<T>(options.formatter.take().unwrap_or_else(default_formatter));
        let source = <Self as ObservableListEx<T>>::change_sets(self);
        let options = change_set_options(options);
        spy_core(source, options, formatter)
    }

    fn debug_spy_list(&self, op_name: &str) -> Rc<dyn IObservable<list::ChangeSet<T>>> {
        if cfg!(debug_assertions) {
            self.spy_list(SpyOptions { logger: Some(Rc::new(|message: &str| eprintln!("{message}"))), ..SpyOptions::named(op_name) })
        } else {
            <Self as ObservableListEx<T>>::change_sets(self)
        }
    }
}
