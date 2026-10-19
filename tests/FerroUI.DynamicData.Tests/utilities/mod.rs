//! The aids of the tests (`DynamicData.Tests/Utilities`), one module per upstream file.
//!
//! - Test sources: [`TestSourceCache`] and [`TestSourceList`], which a test can fail or complete.
//! - Recording observers: [`CacheItemRecordingObserver`], [`ListItemRecordingObserver`],
//!   [`ValueRecordingObserver`] over [`RecordingObserverBase`], subscribed by `record_cache_items`,
//!   `record_list_items` and `record_values` ([`ObservableExtensions`] and its siblings), which
//!   return the subscription and the observer (upstream's `out` parameter).
//! - Validators: `validate_cache_change_sets`, `validate_list_change_sets` and
//!   `validate_synchronization`.
//! - Schedulers and time: [`FakeScheduler`], [`ObservableEx::interval`], the random times and
//!   intervals of the [`Randomizer`](crate::domain::Randomizer) and `Faker::interval_generate`.
//! - Assertions: FluentAssertions becomes functions that panic with a message
//!   ([`assert_cache_should_have_refreshed`], [`assert_list_should_have_refreshed`],
//!   [`assert_should_not_support_sorting`], [`assert_equivalent`]).

mod cache_change_set_assertions;
mod cache_item_recording_observer;
mod cache_item_recording_observer_assertions;
mod comparer_extensions;
mod equivalence;
mod fake_scheduler;
mod faker_extensions;
mod functional_extensions;
mod list_change_set_assertions;
mod list_item_recording_observer;
mod observable_ex;
mod observable_extensions;
mod observable_spy;
mod randomizer_extensions;
mod raw_anonymous_observable;
mod raw_anonymous_observer;
mod recording_observer_base;
mod select_many_extensions;
mod stream_completion_strategy;
mod stress_add_remove_extensions;
mod test_source_cache;
mod test_source_list;
mod unsynchronized_notification_exception;
mod value_recording_observer;

pub use cache_change_set_assertions::assert_cache_should_have_refreshed;
pub use cache_item_recording_observer::CacheItemRecordingObserver;
pub use cache_item_recording_observer_assertions::assert_should_not_support_sorting;
pub use comparer_extensions::{inverted_comparer, no_op_comparer, no_op_equality_comparer, Comparer, ComparerExtensions};
pub use equivalence::{assert_equivalent, assert_equivalent_because, assert_equivalent_strict, is_equivalent};
pub use fake_scheduler::{FakeScheduler, ScheduledAction};
pub use functional_extensions::FunctionalExtensions;
pub use list_change_set_assertions::assert_list_should_have_refreshed;
pub use list_item_recording_observer::ListItemRecordingObserver;
pub use observable_ex::ObservableEx;
pub use observable_extensions::{CacheObservableExtensions, ChangeSetValidationError, ListObservableExtensions, ObservableExtensions};
pub use observable_spy::{CacheObservableSpy, ListObservableSpy, ObservableSpy, SpyOptions};
pub use raw_anonymous_observable::RawAnonymousObservable;
pub use raw_anonymous_observer::RawAnonymousObserver;
pub use recording_observer_base::RecordingObserverBase;
pub use select_many_extensions::{empty_if_null, SelectManyExtensions};
pub use stream_completion_strategy::StreamCompletionStrategy;
pub use stress_add_remove_extensions::StressAddRemoveExtensions;
pub use test_source_cache::TestSourceCache;
pub use test_source_list::TestSourceList;
pub use unsynchronized_notification_exception::UnsynchronizedNotificationException;
pub use value_recording_observer::ValueRecordingObserver;
