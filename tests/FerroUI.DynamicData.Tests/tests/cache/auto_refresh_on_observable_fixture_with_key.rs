//! `AutoRefreshOnObservableFixture.WithKey`
//! (`DynamicData.Tests/Cache/AutoRefreshOnObservableFixture.WithKey.cs`): `AutoRefreshOnObservable`
//! with a reevaluator of the item and its key, with the tests of the base in the module `base`.

use super::auto_refresh_on_observable_fixture::Item;
use ferroui_dynamic_data::cache::{ChangeSet, ObservableCacheEx};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::IObservable;
use std::rc::Rc;
use std::time::Duration;

// Not ported: ReevaluatorIsNull_ThrowsException - argument validation

/// `BuildUut`: `source.AutoRefreshOnObservable((item, _) => reevaluator.Invoke(item), changeSetBuffer, scheduler)`.
fn build_uut<A: 'static>(
    source: Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>,
    reevaluator: impl Fn(&Rc<Item>) -> Rc<dyn IObservable<A>> + 'static,
    change_set_buffer: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>> {
    source.auto_refresh_on_observable_with_key_with(move |item: &Rc<Item>, _: &i32| reevaluator(item), change_set_buffer, scheduler)
}

super::auto_refresh_on_observable_fixture_base::auto_refresh_on_observable_fixture_base_tests!(build_uut);
