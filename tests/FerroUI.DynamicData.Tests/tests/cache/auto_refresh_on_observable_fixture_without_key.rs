//! `AutoRefreshOnObservableFixture.WithoutKey`
//! (`DynamicData.Tests/Cache/AutoRefreshOnObservableFixture.WithoutKey.cs`):
//! `AutoRefreshOnObservable` with a reevaluator of the item, with the tests of the base in the
//! module `base`.

use super::auto_refresh_on_observable_fixture::Item;
use ferroui_dynamic_data::cache::{ChangeSet, ObservableCacheEx};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::IObservable;
use std::rc::Rc;
use std::time::Duration;

// Not ported: ReevaluatorIsNull_ThrowsException - argument validation (skipped upstream as an existing defect)

/// `BuildUut`: `source.AutoRefreshOnObservable(reevaluator, changeSetBuffer, scheduler)`.
fn build_uut<A: 'static>(
    source: Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>,
    reevaluator: impl Fn(&Rc<Item>) -> Rc<dyn IObservable<A>> + 'static,
    change_set_buffer: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>> {
    source.auto_refresh_on_observable_with(reevaluator, change_set_buffer, scheduler)
}

super::auto_refresh_on_observable_fixture_base::auto_refresh_on_observable_fixture_base_tests!(build_uut);
