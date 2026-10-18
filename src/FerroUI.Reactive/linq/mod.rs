//! `System.Reactive.Linq`: the operators of observable sequences.
//!
//! The static members of upstream's `Observable` (the factories: `Create`, `Return`, `Range`, ...)
//! are associated functions of [`Observable`]; its extension methods on `IObservable<T>` (the
//! operators: `Select`, `Where`, `Merge`, ...) are methods of [`ObservableExt`] on
//! `Rc<dyn IObservable<T>>`, the handle of a sequence. The names are upstream's in Rust case
//! (`select`, `where_`, `distinct_until_changed`). Each family of operators is a module of its own:
//! its implementation, and the methods it adds to [`ObservableExt`] (a `*_methods!` macro the trait
//! invokes).

mod creation;
mod observable_ext;
pub(crate) mod multicast;
pub(crate) mod publish;
pub(crate) mod replay;
pub(crate) mod ref_count;
pub(crate) mod auto_connect;
pub(crate) mod select_many;
pub(crate) mod join;
pub(crate) mod group_join;
pub(crate) mod grouped_observable;
pub(crate) mod group_by;
pub(crate) mod group_by_until;
pub(crate) mod zip;
pub(crate) mod numeric;
pub(crate) mod sum;
pub(crate) mod average;
pub(crate) mod min;
pub(crate) mod max;
pub(crate) mod min_by;
pub(crate) mod max_by;
pub(crate) mod combine_latest;
pub(crate) mod merge;
pub(crate) mod concat;
pub(crate) mod amb;
pub(crate) mod switch;
pub(crate) mod with_latest_from;
pub(crate) mod buffer;
pub(crate) mod window;
pub(crate) mod delay;
pub(crate) mod delay_subscription;
pub(crate) mod time_interval_op;
pub(crate) mod timestamp;
pub(crate) mod throttle;
pub(crate) mod sample;
pub(crate) mod timeout;
pub(crate) mod observe_on;
pub(crate) mod subscribe_on;
pub(crate) mod synchronize;
pub(crate) mod defer;
pub(crate) mod range;
pub(crate) mod repeat;
pub(crate) mod generate;
pub(crate) mod using;
pub(crate) mod start;
pub(crate) mod to_observable;
pub(crate) mod timer;
pub(crate) mod interval;
pub(crate) mod distinct;
pub(crate) mod distinct_until_changed;
pub(crate) mod element_at;
pub(crate) mod catch;
pub(crate) mod on_error_resume_next;
pub(crate) mod retry;
pub(crate) mod repeat_source;
pub(crate) mod retry_when;
pub(crate) mod repeat_when;
pub(crate) mod aggregate;
pub(crate) mod all;
pub(crate) mod any;
pub(crate) mod append_prepend;
pub(crate) mod as_observable;
pub(crate) mod cast;
pub(crate) mod contains;
pub(crate) mod count;
pub(crate) mod default_if_empty;
pub(crate) mod dematerialize;
pub(crate) mod do_;
pub(crate) mod finally;
pub(crate) mod first_async;
pub(crate) mod ignore_elements;
pub(crate) mod is_empty;
pub(crate) mod last_async;
pub(crate) mod materialize;
pub(crate) mod of_type;
pub(crate) mod scan;
pub(crate) mod select;
pub(crate) mod sequence_equal;
pub(crate) mod single_async;
pub(crate) mod skip;
pub(crate) mod skip_last;
pub(crate) mod skip_until;
pub(crate) mod skip_while;
pub(crate) mod start_with;
pub(crate) mod subscribe;
pub(crate) mod take;
pub(crate) mod take_last;
pub(crate) mod take_last_buffer;
pub(crate) mod take_until;
pub(crate) mod take_while;
pub(crate) mod to_array;
pub(crate) mod to_dictionary;
pub(crate) mod to_list;
pub(crate) mod to_lookup;
pub(crate) mod where_;

pub use aggregate::SequenceError;
pub use cast::{AnyObject, InvalidCastException};
pub use creation::Observable;
pub use dematerialize::NotificationValue;
pub use element_at::ArgumentOutOfRangeError;
pub use grouped_observable::IGroupedObservable;
pub use merge::ObservableSource;
pub use multicast::ConnectableObservableExt;
pub use numeric::{Averageable, Extremum, Summable};
pub use timeout::TimeoutException;
pub use to_lookup::{Grouping, Lookup};
pub use observable_ext::ObservableExt;
