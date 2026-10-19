//! `TestSchedulerExtensions` (`ReactiveUI.Testing.Reactive/TestSchedulerExtensions.cs`).

use ferroui_reactive::{Notification, ObservableError};
use ferroui_reactive_testing::{duration_to_ticks, Recorded, TestScheduler};
use std::time::Duration;

/// The ticks of 100 nanoseconds in a millisecond (`TimeSpan.TicksPerMillisecond`).
const TICKS_PER_MILLISECOND: f64 = 10_000.0;

/// `TestScheduler.FromTimeSpan(TimeSpan.FromMilliseconds(milliseconds))`: the ticks of a time in
/// milliseconds, truncated to a whole tick as `TimeSpan.FromMilliseconds` does.
fn ticks_from_milliseconds(milliseconds: f64) -> i64 {
    (milliseconds * TICKS_PER_MILLISECOND) as i64
}

/// The virtual time of a [`TestScheduler`] in milliseconds, and recorded notifications at a time in
/// milliseconds (`TestSchedulerExtensions`, from ReactiveUI.Testing.Reactive).
///
/// The functions without a scheduler are called on the type, as upstream's static extension
/// members are: `TestScheduler::on_next_at(100.0, 42)`.
///
/// Not ported: the `ArgumentException` of a time that is not a number and the
/// `OverflowException` of one too large for a `TimeSpan` - argument validation (the conversion
/// saturates).
pub trait TestSchedulerExtensions {
    /// The ticks of the virtual time of a duration (`FromTimeSpan`).
    fn from_time_span(span: Duration) -> i64;

    /// A value at a time in milliseconds (`OnNextAt<T>(double, T)`).
    fn on_next_at<T>(milliseconds: f64, value: T) -> Recorded<Notification<T>>;

    /// An error at a time in milliseconds (`OnErrorAt<T>(double, Exception)`).
    fn on_error_at<T>(milliseconds: f64, error: ObservableError) -> Recorded<Notification<T>>;

    /// The end of a sequence at a time in milliseconds (`OnCompletedAt<T>(double)`).
    fn on_completed_at<T>(milliseconds: f64) -> Recorded<Notification<T>>;

    /// Runs the scheduler until its clock reaches the absolute time in milliseconds
    /// (`AdvanceToMs`).
    fn advance_to_ms(&self, milliseconds: f64);

    /// Runs the scheduler for a time in milliseconds (`AdvanceByMs`).
    fn advance_by_ms(&self, milliseconds: f64);
}

impl TestSchedulerExtensions for TestScheduler {
    fn from_time_span(span: Duration) -> i64 {
        duration_to_ticks(span)
    }

    fn on_next_at<T>(milliseconds: f64, value: T) -> Recorded<Notification<T>> {
        Recorded::new(ticks_from_milliseconds(milliseconds), Notification::OnNext(value))
    }

    fn on_error_at<T>(milliseconds: f64, error: ObservableError) -> Recorded<Notification<T>> {
        Recorded::new(ticks_from_milliseconds(milliseconds), Notification::OnError(error))
    }

    fn on_completed_at<T>(milliseconds: f64) -> Recorded<Notification<T>> {
        Recorded::new(ticks_from_milliseconds(milliseconds), Notification::OnCompleted)
    }

    fn advance_to_ms(&self, milliseconds: f64) {
        self.advance_to(ticks_from_milliseconds(milliseconds));
    }

    fn advance_by_ms(&self, milliseconds: f64) {
        self.advance_by(ticks_from_milliseconds(milliseconds));
    }
}
