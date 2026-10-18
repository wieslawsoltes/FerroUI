//! [`ObservableExt`]: the operators of a sequence as methods of its handle.

use ferroui_system::IObservable;
use std::rc::Rc;

/// The operators of an observable sequence, as methods of its handle (the extension methods of
/// upstream's `Observable` and `ObservableExtensions`).
///
/// The one required method gives the sequence as a handle; every operator is a provided method,
/// declared in the module of its family (`select`, `where_`, ...). The trait is implemented for
/// `Rc<dyn IObservable<T>>`, and for the subjects and test observables through it.
pub trait ObservableExt<T: 'static> {
    /// The sequence as the handle the operators take.
    fn observable(&self) -> Rc<dyn IObservable<T>>;

    super::subscribe::subscribe_methods!();
    super::select::select_methods!();
    super::where_::where_methods!();
    super::as_observable::as_observable_methods!();
    super::cast::cast_methods!();
    super::of_type::of_type_methods!();
    super::do_::do_methods!();
    super::finally::finally_methods!();
    super::ignore_elements::ignore_elements_methods!();
    super::materialize::materialize_methods!();
    super::dematerialize::dematerialize_methods!();
    super::scan::scan_methods!();
    super::start_with::start_with_methods!();
    super::append_prepend::append_prepend_methods!();
    super::default_if_empty::default_if_empty_methods!();
    super::skip::skip_methods!();
    super::skip_last::skip_last_methods!();
    super::skip_while::skip_while_methods!();
    super::skip_until::skip_until_methods!();
    super::take::take_methods!();
    super::take_last::take_last_methods!();
    super::take_last_buffer::take_last_buffer_methods!();
    super::take_while::take_while_methods!();
    super::take_until::take_until_methods!();
    super::aggregate::aggregate_methods!();
    super::all::all_methods!();
    super::any::any_methods!();
    super::contains::contains_methods!();
    super::count::count_methods!();
    super::first_async::first_async_methods!();
    super::is_empty::is_empty_methods!();
    super::last_async::last_async_methods!();
    super::sequence_equal::sequence_equal_methods!();
    super::single_async::single_async_methods!();
    super::to_array::to_array_methods!();
    super::to_dictionary::to_dictionary_methods!();
    super::to_list::to_list_methods!();
    super::to_lookup::to_lookup_methods!();
    super::distinct::distinct_methods!();
    super::distinct_until_changed::distinct_until_changed_methods!();
    super::element_at::element_at_methods!();
    super::catch::catch_methods!();
    super::on_error_resume_next::on_error_resume_next_methods!();
    super::retry::retry_methods!();
    super::repeat_source::repeat_source_methods!();
    super::retry_when::retry_when_methods!();
    super::repeat_when::repeat_when_methods!();
    super::delay::delay_methods!();
    super::delay_subscription::delay_subscription_methods!();
    super::time_interval_op::time_interval_methods!();
    super::timestamp::timestamp_methods!();
    super::throttle::throttle_methods!();
    super::sample::sample_methods!();
    super::timeout::timeout_methods!();
    super::observe_on::observe_on_methods!();
    super::subscribe_on::subscribe_on_methods!();
    super::synchronize::synchronize_methods!();
    super::merge::merge_methods!();
    super::concat::concat_methods!();
    super::amb::amb_methods!();
    super::switch::switch_methods!();
    super::with_latest_from::with_latest_from_methods!();
    super::buffer::buffer_methods!();
    super::window::window_methods!();
    super::combine_latest::combine_latest_methods!();
    super::zip::zip_methods!();
    super::group_by::group_by_methods!();
    super::group_by_until::group_by_until_methods!();
    super::multicast::multicast_methods!();
    super::publish::publish_methods!();
    super::replay::replay_methods!();
    super::select_many::select_many_methods!();
    super::join::join_methods!();
    super::group_join::group_join_methods!();
    super::sum::sum_methods!();
    super::average::average_methods!();
    super::min::min_methods!();
    super::max::max_methods!();
    super::min_by::min_by_methods!();
    super::max_by::max_by_methods!();
}

impl<T: 'static> ObservableExt<T> for Rc<dyn IObservable<T>> {
    fn observable(&self) -> Rc<dyn IObservable<T>> {
        self.clone()
    }
}
