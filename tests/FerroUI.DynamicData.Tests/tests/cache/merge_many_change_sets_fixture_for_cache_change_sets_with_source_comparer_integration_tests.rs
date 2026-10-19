//! `MergeManyChangeSetsFixture.ForCacheChangeSets.WithSourceComparer.IntegrationTests`
//! (`DynamicData.Tests/Cache/MergeManyChangeSetsFixture.ForCacheChangeSets.WithSourceComparer.IntegrationTests.cs`):
//! markets and their prices added and removed at random times while the merged prices are
//! subscribed and unsubscribed over and over; the merge must end with the prices of the best
//! market of each item.
//!
//! Upstream runs the stress on the task pool and yields to it between the subscriptions. Here the
//! same work runs on one thread on a `TestScheduler`: each `Task.Yield` becomes a step of the
//! virtual clock, so the additions and removals interleave with the subscriptions as they do
//! upstream, in an order of the seed. `Environment.ProcessorCount` (the number of parallel
//! streams) is a constant. The constants of the partial classes are repeated here, with the values
//! of a debug build.

use crate::domain::{fakers, Faker, FakerContext, Guid, IMarket, Market, MarketPrice, Randomizer};
use crate::utilities::{ObservableExtensions, StressAddRemoveExtensions};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

// The constants of `MergeManyChangeSetsFixture.ForCacheChangeSets` that the test uses.
const BASE_PRICE: f64 = 10.0;
const PRICE_OFFSET: f64 = 10.0;

/// The seed of the randomizer of the test.
const SEED: i32 = 0x10012022;

/// The number of parallel streams of additions (`Environment.ProcessorCount` upstream).
const PARALLEL: usize = 4;

/// The step of the virtual clock that stands for a `Task.Yield`.
const YIELD_STEP: Duration = Duration::from_millis(10);

/// The most rounds of subscriptions before the test gives up on the stress ending.
const MAX_ROUNDS: usize = 1_000_000;

type MarketCache = SourceCache<Rc<dyn IMarket>, Guid>;
type MarketComparer = Rc<dyn Fn(&Rc<dyn IMarket>, &Rc<dyn IMarket>) -> Ordering>;

/// `GetRandomPrice`.
fn get_random_price(randomizer: &Randomizer) -> f64 {
    MarketPrice::random_price(randomizer, BASE_PRICE, PRICE_OFFSET)
}

/// The state of the local functions of `MultiThreadedStressTest`.
struct Stress {
    market_cache: MarketCache,
    /// The markets the faker made, by identifier: the cache holds them as `IMarket`, and the test
    /// needs their prices caches (upstream's cast `(Market)market`).
    markets: Rc<RefCell<HashMap<Guid, Rc<Market>>>>,
    randomizer: Randomizer,
    market_faker: Faker<Rc<Market>>,
    scheduler: Rc<dyn IScheduler>,
    max_add_time: Duration,
    max_remove_time: Duration,
}

impl Stress {
    /// `MaxItemId`.
    const MAX_ITEM_ID: i32 = 50;

    /// `GetRemoveTime`.
    fn get_remove_time(&self) -> Option<Duration> {
        if self.randomizer.bool() {
            Some(self.randomizer.time_span(self.max_remove_time))
        } else {
            None
        }
    }

    /// The market of an item of the cache.
    fn market_of(&self, market: &Rc<dyn IMarket>) -> Rc<Market> {
        self.markets.borrow().get(&market.id()).cloned().expect("a market that the faker made")
    }

    /// `AddRemoveStress`.
    fn add_remove_stress(self: &Rc<Self>, market_count: usize, price_count: usize, parallel: usize) -> Rc<dyn IObservable<()>> {
        let this = self.clone();
        Observable::create(move |observer: Rc<dyn IObserver<()>>| {
            let markets_error = observer.clone();
            let markets_subscription =
                this.add_remove_markets(market_count, parallel).subscribe_next_error(|_| {}, move |error: ObservableError| markets_error.on_error(error));
            let prices_stress = this.clone();
            let (prices_error, prices_completed) = (observer.clone(), observer.clone());
            let prices_subscription = this
                .market_cache
                .connect()
                .merge_many(move |market: &Rc<dyn IMarket>| prices_stress.add_remove_prices(&prices_stress.market_of(market), price_count, parallel))
                .subscribe_all(|_| {}, move |error: ObservableError| prices_error.on_error(error), move || prices_completed.on_completed());
            Disposable::create(move || {
                markets_subscription.dispose();
                prices_subscription.dispose();
            })
        })
    }

    /// `AddRemoveMarkets`.
    fn add_remove_markets(self: &Rc<Self>, owner_count: usize, parallel: usize) -> Rc<dyn IObservable<Rc<dyn IMarket>>> {
        let markets = self.markets.clone();
        // `marketFaker.IntervalGenerate(MaxAddTime, scheduler)`: the overload of a fixed period.
        let generated: Rc<dyn IObservable<Rc<dyn IMarket>>> =
            self.market_faker.interval_generate_every(self.max_add_time, Some(self.scheduler.clone())).select(move |market: Rc<Market>| {
                markets.borrow_mut().insert(market.id(), market.clone());
                market as Rc<dyn IMarket>
            });
        let this = self.clone();
        let market_cache = self.market_cache.clone();
        generated
            .parallelize_with(owner_count, parallel, move |source: Rc<dyn IObservable<Rc<dyn IMarket>>>| {
                let remove_time = this.clone();
                let scheduler = this.scheduler.clone();
                source.stress_add_remove_cache(this.market_cache.clone(), move |_: &Rc<dyn IMarket>| remove_time.get_remove_time(), scheduler)
            })
            .finally(move || market_cache.dispose())
    }

    /// `AddRemovePrices`.
    fn add_remove_prices(self: &Rc<Self>, market: &Rc<Market>, price_count: usize, parallel: usize) -> Rc<dyn IObservable<Rc<MarketPrice>>> {
        let randomizer = self.randomizer.clone();
        let price_market = market.clone();
        let prices: Rc<dyn IObservable<Rc<MarketPrice>>> = self
            .randomizer
            .interval(self.max_add_time, Some(self.scheduler.clone()))
            .select(move |_: i64| price_market.create_price(randomizer.number(Self::MAX_ITEM_ID), get_random_price(&randomizer)));
        let this = self.clone();
        let prices_cache = market.prices_cache().clone();
        let disposed_cache = prices_cache.clone();
        prices
            .parallelize_with(price_count, parallel, move |source: Rc<dyn IObservable<Rc<MarketPrice>>>| {
                let remove_time = this.clone();
                let scheduler = this.scheduler.clone();
                source.stress_add_remove_cache(prices_cache.clone(), move |_: &Rc<MarketPrice>| remove_time.get_remove_time(), scheduler)
            })
            .finally(move || disposed_cache.dispose())
    }
}

/// `MultiThreadedStressTest(int marketCount, int priceCount)`.
fn multi_threaded_stress_test(market_count: usize, price_count: usize) {
    let max_add_time = Duration::from_millis(250);
    let max_remove_time = Duration::from_millis(100);
    let test_scheduler = TestScheduler::new();

    let randomizer = Randomizer::new(SEED);
    let market_faker = fakers::market()
        .rule_for(|market: &Rc<Market>, faker: &FakerContext| market.set_rating(faker.random().double_range(0.0, 5.0)))
        .with_seed(&randomizer);

    let market_cache: MarketCache = SourceCache::new(|market: &Rc<dyn IMarket>| market.id());
    let market_cache_results = market_cache.connect().as_aggregator();

    let merged = market_cache.connect().merge_many_change_sets_with_source_comparer(|market: &Rc<dyn IMarket>| market.latest_prices(), Market::rating_compare()).publish();
    let merged_source: Rc<dyn IObservable<ChangeSet<Rc<MarketPrice>, i32>>> = merged.clone();
    let adding = Rc::new(Cell::new(true));
    // `merged.LastOrDefaultAsync().ToTask()`: whether the merge has completed.
    let cache_completed = Rc::new(Cell::new(false));
    let completed = cache_completed.clone();
    let cache_completed_subscription = merged_source.subscribe_next_completed(|_| {}, move || completed.set(true));
    let price_results = merged_source.as_aggregator();
    let connect = merged.connect();

    let stress = Rc::new(Stress {
        market_cache: market_cache.clone(),
        markets: Rc::new(RefCell::new(HashMap::new())),
        randomizer: randomizer.clone(),
        market_faker,
        scheduler: test_scheduler.scheduler(),
        max_add_time,
        max_remove_time,
    });

    // Start modifying the parent list and the child lists
    let adding_flag = adding.clone();
    let adding_subscription = stress.add_remove_stress(market_count, price_count, PARALLEL).finally(move || adding_flag.set(false)).subscribe_ignore();

    // Subscribe / unsubscribe over and over while the collections are being modified
    let step = duration_to_ticks(YIELD_STEP);
    let mut rounds = 0;
    loop {
        // Ensure items are being added before subscribing to changes
        test_scheduler.advance_by(step);

        {
            // Subscribe
            let merged_subscription = merged_source.subscribe_ignore();

            // Let the other work run
            test_scheduler.advance_by(step);

            // Unsubscribe
            merged_subscription.dispose();
        }

        rounds += 1;
        if !adding.get() {
            break;
        }
        assert!(rounds < MAX_ROUNDS, "the stress did not end");
    }

    // Wait for the source cache to finish delivering all notifications.
    test_scheduler.run();
    assert!(cache_completed.get(), "the merged prices should complete");

    // Verify the results
    check_result_contents(&market_cache, &market_cache_results, &price_results, Market::rating_compare(), &stress);

    adding_subscription.dispose();
    connect.dispose();
    cache_completed_subscription.dispose();
}

/// `CheckResultContents`.
fn check_result_contents(
    market_cache: &MarketCache,
    market_results: &ChangeSetAggregator<Rc<dyn IMarket>, Guid>,
    price_results: &ChangeSetAggregator<Rc<MarketPrice>, i32>,
    comparer: MarketComparer,
    stress: &Stress,
) {
    let expected_markets: Vec<Rc<dyn IMarket>> = market_cache.items();

    // These should be subsets of each other
    let market_items = market_results.data().items();
    for market in &expected_markets {
        assert!(market_items.contains(market), "the results should have {market}");
    }
    assert_eq!(market_results.data().items().len(), expected_markets.len());

    // Pair up all the Markets/Prices, Group them by ItemId, and sort each Group by the Market comparer
    // Then pull out the first value from each group, which should be the price from the best market for each ItemId
    let mut groups: Vec<(i32, Vec<(Rc<dyn IMarket>, Rc<MarketPrice>)>)> = Vec::new();
    for market in &expected_markets {
        for price in stress.market_of(market).prices_cache().items() {
            let item_id = price.item_id();
            match groups.iter_mut().find(|(id, _)| *id == item_id) {
                Some((_, group)) => group.push((market.clone(), price)),
                None => groups.push((item_id, vec![(market.clone(), price)])),
            }
        }
    }
    let expected_prices: Vec<Rc<MarketPrice>> = groups
        .into_iter()
        .map(|(_, mut group)| {
            // `OrderBy` is a stable sort.
            group.sort_by(|(x, _), (y, _)| comparer(x, y));
            group.swap_remove(0).1
        })
        .collect();

    // These should be subsets of each other
    let price_items = price_results.data().items();
    for price in &expected_prices {
        assert!(price_items.contains(price), "the results should have {price}");
    }
    assert_eq!(price_results.data().items().len(), expected_prices.len());
}

#[test]
fn multi_threaded_stress_test_case1() {
    multi_threaded_stress_test(5, 7);
}

#[test]
fn multi_threaded_stress_test_case2() {
    multi_threaded_stress_test(10, 50);
}
