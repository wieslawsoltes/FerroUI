//! `MergeManyChangeSetsCacheFixture` (`DynamicData.Tests/List/MergeManyChangeSetsCacheFixture.cs`):
//! the change sets of the caches of prices of the markets of a list, merged into those of one
//! cache (`merge_many_cache_change_sets*`), with the equality and the comparers of prices that
//! decide which price of an item the result shows.
//!
//! The counts are those of upstream's debug build.
//!
//! The list holds markets as `Rc<dyn IMarket>`, which cannot be cast back to `Market` as upstream
//! casts them (`(Market)market`, `m as IDisposable`): the fixture keeps the markets it adds in a
//! registry, and finds a market of the list there by its address.

use crate::domain::{fakers, FixedMarket, Guid, IMarket, Market, MarketPrice, Randomizer};
use crate::utilities::{CacheObservableSpy, ListObservableSpy, ObservableExtensions, StressAddRemoveExtensions};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator as CacheChangeSetAggregator;
use ferroui_dynamic_data::cache::{IObservableCache, ISourceUpdater, ObservableCacheEx};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{self, IExtendedList, IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{duration_to_ticks, new_error, TestScheduler};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// `MarketCount` (debug build).
const MARKET_COUNT: usize = 3;
/// `PricesPerMarket` (debug build).
const PRICES_PER_MARKET: usize = 5;
/// `RemoveCount` (debug build).
const REMOVE_COUNT: usize = 2;
/// `ItemIdStride`.
const ITEM_ID_STRIDE: i32 = 1000;
/// `BasePrice`.
const BASE_PRICE: f64 = 10.0;
/// `PriceOffset`.
const PRICE_OFFSET: f64 = 10.0;
/// `HighestPrice`.
const HIGHEST_PRICE: f64 = BASE_PRICE + PRICE_OFFSET + 1.0;
/// `LowestPrice`.
const LOWEST_PRICE: f64 = BASE_PRICE - 1.0;

/// [`PRICES_PER_MARKET`] as an identifier of an item.
const PRICES: i32 = PRICES_PER_MARKET as i32;

/// The seed of the fixture.
const SEED: i32 = 0x0325_1976;

/// The number of streams that add and remove at once in the stress test
/// (`Environment.ProcessorCount` upstream).
const PARALLEL: usize = 4;

/// The most rounds of subscribing and unsubscribing before the stress test gives up on the
/// stress ending.
const MAX_ROUNDS: usize = 1_000_000;

type MarketList = SourceList<Rc<dyn IMarket>>;
type MarketRegistry = Rc<RefCell<Vec<Rc<Market>>>>;
type PriceResults = CacheChangeSetAggregator<Rc<MarketPrice>, i32>;

/// `GetRandomPrice`.
fn get_random_price(randomizer: &Randomizer) -> f64 {
    MarketPrice::random_price(randomizer, BASE_PRICE, PRICE_OFFSET)
}

/// The market as the list holds it.
fn as_imarket(market: &Rc<Market>) -> Rc<dyn IMarket> {
    let market: Rc<dyn IMarket> = market.clone();
    market
}

/// The `Market` of the registry that `market` is (`(Market)market`), if it is one.
fn find_market(registry: &RefCell<Vec<Rc<Market>>>, market: &Rc<dyn IMarket>) -> Option<Rc<Market>> {
    registry.borrow().iter().find(|candidate| std::ptr::addr_eq(Rc::as_ptr(*candidate), Rc::as_ptr(market))).cloned()
}

/// `Enumerable.Range(0, count).Select(n => new Market(n)).ToArray()`.
fn new_markets(count: usize) -> Vec<Rc<Market>> {
    (0..count).map(|n| Market::from_number(n as i32)).collect()
}

/// `MergeManyChangeSetsCacheFixture`: a list of markets, the results of its change sets, a faker of
/// markets and the seeded generator of prices.
struct Fixture {
    market_list: MarketList,
    market_list_results: ChangeSetAggregator<Rc<dyn IMarket>>,
    market_faker: fakers::Faker<Rc<Market>>,
    randomizer: Randomizer,
    markets: MarketRegistry,
}

impl Fixture {
    fn new() -> Self {
        let randomizer = Randomizer::new(SEED);
        let market_faker = fakers::market().with_seed(&randomizer);
        let market_list: MarketList = SourceList::new();
        let market_list_results = market_list.connect().as_aggregator();
        Self { market_list, market_list_results, market_faker, randomizer, markets: Rc::new(RefCell::new(Vec::new())) }
    }

    /// `GetRandomPrice`.
    fn random_price(&self) -> f64 {
        get_random_price(&self.randomizer)
    }

    fn register(&self, markets: &[Rc<Market>]) {
        self.markets.borrow_mut().extend(markets.iter().cloned());
    }

    /// `_marketList.Add(market)`.
    fn add(&self, market: &Rc<Market>) {
        self.register(std::slice::from_ref(market));
        let item = as_imarket(market);
        self.market_list.edit(|list: &mut dyn IExtendedList<Rc<dyn IMarket>>| list.add(item));
    }

    /// `_marketList.AddRange(markets)`.
    fn add_range(&self, markets: &[Rc<Market>]) {
        self.register(markets);
        let items: Vec<Rc<dyn IMarket>> = markets.iter().map(as_imarket).collect();
        self.market_list.edit(|list: &mut dyn IExtendedList<Rc<dyn IMarket>>| list.add_range(items));
    }

    /// `_marketList.Remove(market)`.
    fn remove(&self, market: &Rc<Market>) {
        let item = as_imarket(market);
        self.market_list.edit(|list: &mut dyn IExtendedList<Rc<dyn IMarket>>| {
            list.remove(&item);
        });
    }

    /// `_marketList.Replace(original, replaceWith)` (`ListEx.Replace`).
    fn replace(&self, original: &Rc<Market>, replace_with: &Rc<Market>) {
        self.register(std::slice::from_ref(replace_with));
        let (original, replace_with) = (as_imarket(original), as_imarket(replace_with));
        self.market_list.edit(|list: &mut dyn IExtendedList<Rc<dyn IMarket>>| {
            let index = list.index_of(&original).expect("the market to replace is in the list");
            list.set(index, replace_with);
        });
    }

    /// `CheckResultContents`.
    fn check_result_contents(&self, market_results: &ChangeSetAggregator<Rc<dyn IMarket>>, price_results: &PriceResults) {
        let expected_markets = self.market_list.items();
        let expected_prices: Vec<Rc<MarketPrice>> = expected_markets
            .iter()
            .flat_map(|market| find_market(&self.markets, market).expect("every market of the list is a Market").prices_cache().items())
            .collect();

        // These should be subsets of each other
        let result_markets = market_results.data().items();
        for market in &expected_markets {
            assert!(result_markets.contains(market), "the market {market} should be in the results");
        }
        assert_eq!(result_markets.len(), expected_markets.len());

        // These should be subsets of each other
        let result_prices = price_results.data().items();
        for price in &expected_prices {
            assert!(result_prices.contains(price), "the price {price} should be in the results");
        }
        assert_eq!(result_prices.len(), expected_prices.len());
    }

    /// `DisposeMarkets`: disposes the markets of the list that can be disposed, then the list, and
    /// clears it.
    fn dispose_markets(&self) {
        for market in self.market_list.items() {
            if let Some(market) = find_market(&self.markets, &market) {
                market.dispose();
            }
        }
        self.market_list.dispose();
        self.market_list.edit(|list: &mut dyn IExtendedList<Rc<dyn IMarket>>| list.clear());
    }
}

impl Drop for Fixture {
    /// `Dispose`.
    fn drop(&mut self) {
        self.market_list_results.dispose();
        self.dispose_markets();
    }
}

/// `m => m.LatestPrices`.
fn latest_prices(market: &Rc<dyn IMarket>) -> Rc<dyn IObservable<ferroui_dynamic_data::cache::ChangeSet<Rc<MarketPrice>, i32>>> {
    market.latest_prices()
}

/// The identifiers of the markets of the prices of `results`.
fn market_ids(results: &PriceResults) -> Vec<Guid> {
    results.data().items().iter().map(|price| price.market_id()).collect()
}

fn assert_all_from(results: &PriceResults, market: &Rc<Market>) {
    for guid in market_ids(results) {
        assert_eq!(guid, market.id());
    }
}

fn multi_threaded_stress_test(market_count: usize, price_count: usize) {
    // Deviation (DEVIATIONS.md, DynamicData): upstream runs the stress on the task pool
    // (`TaskPoolScheduler.Default`, `Environment.ProcessorCount` streams in parallel) and
    // subscribes and unsubscribes between `Task.Yield()`s while it runs. Here it runs on one
    // thread, on a `TestScheduler`: the clock advances by a millisecond before each subscription
    // and before each unsubscription, and the number of streams is fixed so that the seed gives
    // the same run on every machine.
    let fixture = Fixture::new();
    let max_add_time = Duration::from_millis(250);
    let max_remove_time = Duration::from_millis(100);

    let test_scheduler = TestScheduler::new();
    let scheduler: Rc<dyn IScheduler> = test_scheduler.scheduler();

    // `GetRemoveTime`
    let get_remove_time: Rc<dyn Fn() -> Option<Duration>> = {
        let randomizer = fixture.randomizer.clone();
        Rc::new(move || if randomizer.bool() { Some(randomizer.time_span(max_remove_time)) } else { None })
    };

    // `AddRemoveMarkets`
    let add_remove_markets: Rc<dyn IObservable<Rc<dyn IMarket>>> = {
        let registry = fixture.markets.clone();
        let market_list = fixture.market_list.clone();
        let market_list_to_dispose = fixture.market_list.clone();
        let get_remove_time = get_remove_time.clone();
        let stress_scheduler = scheduler.clone();
        fixture
            .market_faker
            .interval_generate_every(max_add_time, Some(scheduler.clone()))
            .select(move |market: Rc<Market>| {
                registry.borrow_mut().push(market.clone());
                as_imarket(&market)
            })
            .parallelize_with(market_count, PARALLEL, move |items: Rc<dyn IObservable<Rc<dyn IMarket>>>| {
                let get_remove_time = get_remove_time.clone();
                items.stress_add_remove_list(market_list.clone(), move |_: &Rc<dyn IMarket>| get_remove_time(), stress_scheduler.clone())
            })
            .finally(move || market_list_to_dispose.dispose())
    };

    // `AddRemovePrices`
    let add_remove_prices: Rc<dyn Fn(&Rc<Market>) -> Rc<dyn IObservable<Rc<MarketPrice>>>> = {
        let randomizer = fixture.randomizer.clone();
        let get_remove_time = get_remove_time.clone();
        let scheduler = scheduler.clone();
        Rc::new(move |market: &Rc<Market>| {
            let (pricing_market, price_randomizer) = (market.clone(), randomizer.clone());
            let prices_cache = market.prices_cache().clone();
            let prices_cache_to_dispose = market.prices_cache().clone();
            let get_remove_time = get_remove_time.clone();
            let stress_scheduler = scheduler.clone();
            randomizer
                .interval(max_add_time, Some(scheduler.clone()))
                .select(move |_: i64| pricing_market.create_unique_price(|_| get_random_price(&price_randomizer)))
                .parallelize_with(price_count, PARALLEL, move |items: Rc<dyn IObservable<Rc<MarketPrice>>>| {
                    let get_remove_time = get_remove_time.clone();
                    items.stress_add_remove_cache(prices_cache.clone(), move |_: &Rc<MarketPrice>| get_remove_time(), stress_scheduler.clone())
                })
                .finally(move || prices_cache_to_dispose.dispose())
        })
    };

    // `AddRemoveStress`
    let add_remove_stress: Rc<dyn IObservable<()>> = {
        let add_remove_markets = add_remove_markets.clone();
        let add_remove_prices = add_remove_prices.clone();
        let market_list = fixture.market_list.clone();
        let registry = fixture.markets.clone();
        Observable::create(move |observer: Rc<dyn IObserver<()>>| {
            let markets_error = observer.clone();
            let markets_subscription = add_remove_markets.subscribe_next_error(|_| {}, move |error: ObservableError| markets_error.on_error(error));

            let (prices_error, prices_completed) = (observer.clone(), observer.clone());
            let (registry, add_remove_prices) = (registry.clone(), add_remove_prices.clone());
            let prices_subscription = market_list
                .connect()
                .merge_many(move |market: &Rc<dyn IMarket>| {
                    let market = find_market(&registry, market).expect("every market of the stress is a Market");
                    add_remove_prices(&market)
                })
                .subscribe_all(|_| {}, move |error: ObservableError| prices_error.on_error(error), move || prices_completed.on_completed());

            Disposable::create(move || {
                markets_subscription.dispose();
                prices_subscription.dispose();
            })
        })
    };

    let merged = fixture.market_list.connect().merge_many_cache_change_sets(latest_prices);
    let price_results: PriceResults = merged.as_aggregator();

    let adding = Rc::new(Cell::new(true));

    // Start modifying the parent list and the child caches
    let adding_subscription = {
        let adding = adding.clone();
        add_remove_stress.finally(move || adding.set(false)).subscribe_ignore()
    };

    // Subscribe / unsubscribe over and over while the collections are being modified
    let step = duration_to_ticks(Duration::from_millis(1));
    let mut rounds = 0;
    while adding.get() {
        // Ensure items are being added before subscribing to changes
        test_scheduler.advance_by(step);

        // Subscribe
        let merged_subscription = merged.subscribe_ignore();

        // Let the other streams run
        test_scheduler.advance_by(step);

        // Unsubscribe
        merged_subscription.dispose();

        rounds += 1;
        assert!(rounds < MAX_ROUNDS, "the stress should end");
    }

    // Verify the results
    fixture.check_result_contents(&fixture.market_list_results, &price_results);

    adding_subscription.dispose();
}

#[test]
fn multi_threaded_stress_test_case1() {
    multi_threaded_stress_test(5, 7);
}

#[test]
fn multi_threaded_stress_test_case2() {
    multi_threaded_stress_test(10, 50);
}

// Not ported: NullChecks - argument validation

#[test]
fn able_to_invoke_factory() {
    let fixture = Fixture::new();

    // having
    let invoked = Rc::new(Cell::new(false));
    let factory = {
        let invoked = invoked.clone();
        move |market: &Rc<dyn IMarket>| {
            invoked.set(true);
            market.latest_prices()
        }
    };
    let sub = fixture.market_list.connect().merge_many_cache_change_sets(factory).subscribe_ignore();

    // when
    fixture.add(&Market::from_number(0));

    // then
    assert_eq!(fixture.market_list_results.data().count(), 1);
    assert!(invoked.get());

    sub.dispose();
}

#[test]
fn all_existing_sub_items_present_in_result() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(MARKET_COUNT);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    for (index, market) in markets.iter().enumerate() {
        market.add_unique_prices(index as i32, PRICES, ITEM_ID_STRIDE, || fixture.random_price());
    }

    // when
    fixture.add_range(&markets);

    // then
    assert_eq!(fixture.market_list_results.data().count(), MARKET_COUNT);
    assert_eq!(markets.iter().map(|market| market.prices_cache().count()).sum::<usize>(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), 1);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn all_new_sub_items_present_in_result() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(MARKET_COUNT);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_range(&markets);

    // when
    for (index, market) in markets.iter().enumerate() {
        market.add_unique_prices(index as i32, PRICES, ITEM_ID_STRIDE, || fixture.random_price());
    }

    // then
    assert_eq!(fixture.market_list_results.data().count(), MARKET_COUNT);
    assert_eq!(markets.iter().map(|market| market.prices_cache().count()).sum::<usize>(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), MARKET_COUNT);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn all_refreshed_sub_items_are_refreshed() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(MARKET_COUNT);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_range(&markets);
    for (index, market) in markets.iter().enumerate() {
        market.add_unique_prices(index as i32, PRICES, ITEM_ID_STRIDE, || fixture.random_price());
    }

    // when
    for market in &markets {
        market.refresh_all_prices(|| fixture.random_price());
    }

    // then
    assert_eq!(fixture.market_list_results.data().count(), MARKET_COUNT);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), MARKET_COUNT * 2);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
    assert_eq!(results.summary().overall.refreshes, MARKET_COUNT * PRICES_PER_MARKET);
}

#[test]
fn any_duplicate_key_values_should_be_hidden() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(2);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_range(&markets);

    // when
    markets[0].set_prices(0, PRICES, || fixture.random_price());
    markets[1].set_prices(0, PRICES, || fixture.random_price());

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    for (first, second) in results.data().items().iter().zip(markets[0].prices_cache().items().iter()) {
        assert_eq!(first, second);
    }
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn any_duplicate_values_should_be_no_op_when_removed() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(2);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_range(&markets);
    markets[0].set_prices(0, PRICES, || fixture.random_price());
    markets[1].set_prices(0, PRICES, || fixture.random_price());

    // when
    markets[1].remove_all_prices();

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    for (first, second) in results.data().items().iter().zip(markets[0].prices_cache().items().iter()) {
        assert_eq!(first, second);
    }
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn any_duplicate_values_should_be_unhidden_when_other_is_removed() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(2);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_range(&markets);
    markets[0].set_prices(0, PRICES, || fixture.random_price());
    markets[1].set_prices(0, PRICES, || fixture.random_price());

    // when
    fixture.remove(&markets[0]);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    for (first, second) in results.data().items().iter().zip(markets[1].prices_cache().items().iter()) {
        assert_eq!(first, second);
    }
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.messages()[1].updates(), PRICES_PER_MARKET);
}

#[test]
fn any_duplicate_values_should_not_refresh_when_hidden() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(2);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_range(&markets);
    markets[0].set_prices(0, PRICES, || fixture.random_price());
    markets[1].set_prices(0, PRICES, || fixture.random_price());

    // when
    markets[1].refresh_all_prices(|| fixture.random_price());

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.refreshes, 0);
    for (first, second) in results.data().items().iter().zip(markets[0].prices_cache().items().iter()) {
        assert_eq!(first, second);
    }
}

#[test]
fn any_removed_sub_item_is_removed() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(MARKET_COUNT);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_range(&markets);
    for (index, market) in markets.iter().enumerate() {
        market.add_unique_prices(index as i32, PRICES, ITEM_ID_STRIDE, || fixture.random_price());
    }

    // when
    for market in &markets {
        market.prices_cache().edit(|updater: &mut dyn ISourceUpdater<Rc<MarketPrice>, i32>| {
            let keys: Vec<i32> = updater.keys().into_iter().take(REMOVE_COUNT).collect();
            updater.remove_keys(&mut keys.into_iter());
        });
    }

    // then
    assert_eq!(fixture.market_list_results.data().count(), MARKET_COUNT);
    assert_eq!(results.data().count(), MARKET_COUNT * (PRICES_PER_MARKET - REMOVE_COUNT));
    assert_eq!(results.messages().len(), MARKET_COUNT * 2);
    assert_eq!(results.messages()[0].adds(), PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, MARKET_COUNT * REMOVE_COUNT);
}

#[test]
fn any_source_item_removed_removes_all_source_values() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(MARKET_COUNT);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_range(&markets);
    for (index, market) in markets.iter().enumerate() {
        market.add_unique_prices(index as i32, PRICES, ITEM_ID_STRIDE, || fixture.random_price());
    }

    // when
    fixture.market_list.edit(|list: &mut dyn IExtendedList<Rc<dyn IMarket>>| list.remove_range(0, REMOVE_COUNT));

    // then
    assert_eq!(fixture.market_list_results.data().count(), MARKET_COUNT - REMOVE_COUNT);
    assert_eq!(results.data().count(), (MARKET_COUNT - REMOVE_COUNT) * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, PRICES_PER_MARKET * REMOVE_COUNT);
}

/// `results.Data.Items.Should().BeSubsetOf(market.PricesCache.Items)` and the converse.
fn assert_same_prices(results: &PriceResults, market: &Rc<Market>) {
    let result_prices = results.data().items();
    let market_prices = market.prices_cache().items();
    for price in &result_prices {
        assert!(market_prices.contains(price), "the price {price} should be one of the market's");
    }
    for price in &market_prices {
        assert!(result_prices.contains(price), "the price {price} should be in the results");
    }
}

#[test]
fn changing_source_by_add_remove_removes_previous_and_adds_new_values() {
    let fixture = Fixture::new();

    // having
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    let market = Market::from_number(0);
    market.set_prices(0, PRICES * 2, || fixture.random_price());
    fixture.add(&market);
    let other_market = Market::from_number(1);
    other_market.set_prices(PRICES, PRICES * 3, || fixture.random_price());

    // when
    fixture.add(&other_market);
    fixture.remove(&market);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET * 2);
    let messages = results.messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].adds(), PRICES_PER_MARKET * 2);
    assert_eq!(messages[1].adds(), PRICES_PER_MARKET);
    assert_eq!(messages[2].updates(), PRICES_PER_MARKET);
    assert_eq!(messages[2].removes(), PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET * 3);
    assert_eq!(results.summary().overall.updates, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, PRICES_PER_MARKET);
    assert_same_prices(&results, &other_market);
}

#[test]
fn changing_source_by_remove_add_removes_previous_and_adds_new_values() {
    let fixture = Fixture::new();

    // having
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    let market = Market::from_number(0);
    market.set_prices(0, PRICES * 2, || fixture.random_price());
    fixture.add(&market);
    let other_market = Market::from_number(1);
    other_market.set_prices(PRICES, PRICES * 3, || fixture.random_price());

    // when
    fixture.remove(&market);
    fixture.add(&other_market);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET * 2);
    let messages = results.messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].adds(), PRICES_PER_MARKET * 2);
    assert_eq!(messages[1].removes(), PRICES_PER_MARKET * 2);
    assert_eq!(messages[2].adds(), PRICES_PER_MARKET * 2);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET * 4);
    assert_eq!(results.summary().overall.updates, 0);
    assert_eq!(results.summary().overall.removes, PRICES_PER_MARKET * 2);
    assert_same_prices(&results, &other_market);
}

#[test]
fn changing_source_by_replace_removes_previous_and_adds_new_values() {
    let fixture = Fixture::new();

    // having
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    let market = Market::from_number(0);
    market.set_prices(0, PRICES * 2, || fixture.random_price());
    fixture.add(&market);
    let other_market = Market::from_number(1);
    other_market.set_prices(PRICES, PRICES * 3, || fixture.random_price());

    // when
    fixture.replace(&market, &other_market);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET * 2);
    assert_same_prices(&results, &other_market);
}

#[test]
fn comparer_only_adds_better_added_values() {
    let fixture = Fixture::new();

    // having
    let high_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_high = Market::from_number(2);
    market_original.set_prices(0, PRICES, || fixture.random_price());
    fixture.add(&market_original);
    fixture.add(&market_low);
    fixture.add(&market_high);

    // when
    market_low.set_prices_to(0, PRICES, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES, HIGHEST_PRICE);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 3);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from(&low_price_results, &market_low);
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from(&high_price_results, &market_high);
}

#[test]
fn comparer_only_adds_better_existing_values() {
    let fixture = Fixture::new();

    // having
    let high_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_high = Market::from_number(2);
    market_original.set_prices(0, PRICES, || fixture.random_price());
    fixture.add(&market_original);
    market_low.set_prices_to(0, PRICES, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES, HIGHEST_PRICE);

    // when
    fixture.add(&market_low);
    fixture.add(&market_high);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 3);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from(&low_price_results, &market_low);
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from(&high_price_results, &market_high);
}

#[test]
fn comparer_only_adds_better_values_on_source_replace() {
    let fixture = Fixture::new();

    // having
    let high_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results: PriceResults = fixture
        .market_list
        .connect()
        .debug_spy_list("List")
        .merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::low_price_compare())
        .debug_spy_cache("MergedLow")
        .as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_low_low = Market::from_number(2);
    market_original.set_prices(0, PRICES, || fixture.random_price());
    market_low.set_prices_to(0, PRICES, LOWEST_PRICE);
    market_low_low.set_prices_to(0, PRICES, LOWEST_PRICE - 1.0);
    fixture.add(&market_original);
    fixture.add(&market_low);

    // when
    fixture.replace(&market_low, &market_low_low);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_all_from(&low_price_results, &market_low_low);
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, 0);
    assert_all_from(&high_price_results, &market_original);
}

#[test]
fn comparer_updates_to_correct_value_on_refresh() {
    let fixture = Fixture::new();

    // having
    let high_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_flip_flop = Market::from_number(1);
    market_original.set_prices(0, PRICES, || fixture.random_price());
    market_flip_flop.set_prices_to(0, PRICES, HIGHEST_PRICE);
    fixture.add(&market_original);
    fixture.add(&market_flip_flop);

    // when
    market_flip_flop.refresh_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.refreshes, 0);
    assert_all_from(&low_price_results, &market_flip_flop);
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_eq!(high_price_results.summary().overall.refreshes, 0);
    assert_all_from(&high_price_results, &market_original);
}

#[test]
fn comparer_updates_to_correct_value_on_remove() {
    let fixture = Fixture::new();

    // having
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    let low_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let high_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_high = Market::from_number(2);
    market_original.set_prices(0, PRICES, || fixture.random_price());
    fixture.add(&market_original);
    fixture.add(&market_low);
    fixture.add(&market_high);
    market_low.set_prices_to(0, PRICES, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES, HIGHEST_PRICE);

    // when
    fixture.remove(&market_low);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.updates, 0);
    assert_all_from(&results, &market_original);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_all_from(&low_price_results, &market_original);
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from(&high_price_results, &market_high);
}

#[test]
fn comparer_updates_to_correct_value_on_update() {
    let fixture = Fixture::new();

    // having
    let high_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_flip_flop = Market::from_number(1);
    market_original.set_prices(0, PRICES, || fixture.random_price());
    market_flip_flop.set_prices_to(0, PRICES, HIGHEST_PRICE);
    fixture.add(&market_original);
    fixture.add(&market_flip_flop);

    // when
    market_flip_flop.update_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.refreshes, 0);
    assert_all_from(&low_price_results, &market_flip_flop);
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_eq!(high_price_results.summary().overall.refreshes, 0);
    assert_all_from(&high_price_results, &market_original);
}

#[test]
fn comparer_only_updates_visible_values_on_update() {
    let fixture = Fixture::new();

    // having
    let high_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_ordered_by(latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    market_original.set_prices(0, PRICES, || fixture.random_price());
    market_low.set_prices_to(0, PRICES, LOWEST_PRICE);
    fixture.add(&market_original);
    fixture.add(&market_low);

    // when
    market_low.update_all_prices_to(LOWEST_PRICE - 1.0);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_eq!(low_price_results.summary().overall.refreshes, 0);
    assert_all_from(&low_price_results, &market_low);
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, 0);
    assert_eq!(high_price_results.summary().overall.refreshes, 0);
    assert_all_from(&high_price_results, &market_original);
}

#[test]
fn comparer_only_refreshes_visible_values() {
    let fixture = Fixture::new();

    // having
    let equality: Rc<dyn Fn(&Rc<MarketPrice>, &Rc<MarketPrice>) -> bool> = Rc::new(MarketPrice::equality_comparer());
    let high_price_results: PriceResults =
        fixture.market_list.connect().merge_many_cache_change_sets_with(latest_prices, Some(equality.clone()), Some(MarketPrice::high_price_compare())).as_aggregator();
    let low_price_results: PriceResults =
        fixture.market_list.connect().merge_many_cache_change_sets_with(latest_prices, Some(equality), Some(MarketPrice::low_price_compare())).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    market_original.set_prices(0, PRICES, || fixture.random_price());
    market_low.set_prices_to(0, PRICES, LOWEST_PRICE);
    fixture.add(&market_original);
    fixture.add(&market_low);

    // when
    market_low.refresh_all_prices_to(LOWEST_PRICE - 1.0);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.refreshes, PRICES_PER_MARKET);
    assert_all_from(&low_price_results, &market_low);
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, 0);
    assert_eq!(high_price_results.summary().overall.refreshes, 0);
    assert_all_from(&high_price_results, &market_original);
}

#[test]
fn equality_comparer_hides_updates_without_changes() {
    let fixture = Fixture::new();

    // having
    let market = Market::from_number(0);
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets_with_comparer(latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    market.set_prices_to(0, PRICES, LOWEST_PRICE);
    fixture.add(&market);

    // when
    market.update_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(fixture.market_list_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), 1);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
    assert_eq!(results.summary().overall.refreshes, 0);
}

/// `Enumerable.Range(0, MarketCount).Select(n => new FixedMarket(GetRandomPrice, n * ItemIdStride,
/// (n * ItemIdStride) + PricesPerMarket, completable))`.
fn fixed_markets(fixture: &Fixture, completable: bool) -> Vec<Rc<dyn IMarket>> {
    (0..MARKET_COUNT as i32)
        .map(|n| {
            let randomizer = fixture.randomizer.clone();
            let market: Rc<dyn IMarket> = FixedMarket::new(move || get_random_price(&randomizer), n * ITEM_ID_STRIDE, (n * ITEM_ID_STRIDE) + PRICES, completable);
            market
        })
        .collect()
}

#[test]
fn every_item_visible_when_sequence_completes() {
    let fixture = Fixture::new();

    // having
    let markets = fixed_markets(&fixture, true);
    fixture.market_list.edit(|list: &mut dyn IExtendedList<Rc<dyn IMarket>>| list.add_range(markets));

    // when
    let results: PriceResults = fixture.market_list.connect().merge_many_cache_change_sets(latest_prices).as_aggregator();
    fixture.dispose_markets();

    // then
    assert_eq!(results.data().count(), PRICES_PER_MARKET * MARKET_COUNT);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET * MARKET_COUNT);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
    assert_eq!(results.summary().overall.refreshes, 0);
}

fn merged_observable_completes_only_when_source_and_all_children_complete(complete_source: bool, complete_children: bool) {
    let fixture = Fixture::new();

    // having
    let markets = fixed_markets(&fixture, complete_children);
    fixture.market_list.edit(|list: &mut dyn IExtendedList<Rc<dyn IMarket>>| list.add_range(markets));
    let has_source_sequence_completed = Rc::new(Cell::new(false));
    let has_merged_sequence_completed = Rc::new(Cell::new(false));

    let cleanup = {
        let (source_completed, merged_completed) = (has_source_sequence_completed.clone(), has_merged_sequence_completed.clone());
        fixture
            .market_list
            .connect()
            .do_with_completed(|_| {}, move || source_completed.set(true))
            .merge_many_cache_change_sets(latest_prices)
            .subscribe_next_completed(|_| {}, move || merged_completed.set(true))
    };

    // when
    if complete_source {
        fixture.dispose_markets();
    }

    // then
    assert_eq!(has_source_sequence_completed.get(), complete_source);
    assert_eq!(has_merged_sequence_completed.get(), complete_source && complete_children);

    cleanup.dispose();
}

#[test]
fn merged_observable_completes_only_when_source_and_all_children_complete_case1() {
    merged_observable_completes_only_when_source_and_all_children_complete(false, false);
}

#[test]
fn merged_observable_completes_only_when_source_and_all_children_complete_case2() {
    merged_observable_completes_only_when_source_and_all_children_complete(false, true);
}

#[test]
fn merged_observable_completes_only_when_source_and_all_children_complete_case3() {
    merged_observable_completes_only_when_source_and_all_children_complete(true, false);
}

#[test]
fn merged_observable_completes_only_when_source_and_all_children_complete_case4() {
    merged_observable_completes_only_when_source_and_all_children_complete(true, true);
}

#[test]
fn merged_observable_will_fail_if_source_fails() {
    let fixture = Fixture::new();

    // having
    let markets = new_markets(MARKET_COUNT);
    fixture.add_range(&markets);
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let expected_error: ObservableError = new_error("Test exception");
    let throw_observable = Observable::throw::<list::ChangeSet<Rc<dyn IMarket>>>(expected_error.clone());

    let cleanup = {
        let received_error = received_error.clone();
        fixture
            .market_list
            .connect()
            .concat(throw_observable)
            .merge_many_cache_change_sets(latest_prices)
            .subscribe_next_error(|_| {}, move |error: ObservableError| *received_error.borrow_mut() = Some(error))
    };

    // when
    fixture.dispose_markets();

    // then
    let error = received_error.borrow().clone().expect("the merged sequence failed");
    assert!(Rc::ptr_eq(&error, &expected_error));

    cleanup.dispose();
}

// Not ported: SourceListMergeCacheChangeSets - list of observables (items need equality)
