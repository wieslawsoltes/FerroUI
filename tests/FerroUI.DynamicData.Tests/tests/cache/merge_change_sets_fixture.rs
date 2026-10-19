//! `MergeChangeSetsFixture` (`DynamicData.Tests/Cache/MergeChangeSetsFixture.cs`): the change sets
//! of several caches merged into those of one, with equality comparers, comparers, a scheduler and
//! the completion of the sources.

use crate::domain::{FixedMarket, Guid, IMarket, Market, MarketPrice, MarketPriceEqualityComparer, PriceChangeSets, Randomizer};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{CacheChangeSetSourcesEx, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheChangeSetsEx, ObservableCacheEx};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{duration_to_ticks, new_error, TestScheduler};
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::rc::Rc;
use std::time::Duration;

const MARKET_COUNT: i32 = if cfg!(debug_assertions) { 5 } else { 101 };
const PRICES_PER_MARKET: i32 = if cfg!(debug_assertions) { 7 } else { 103 };
const REMOVE_COUNT: i32 = if cfg!(debug_assertions) { 3 } else { 53 };

const ITEM_ID_STRIDE: i32 = 1000;
const BASE_PRICE: f64 = 10.0;
const PRICE_OFFSET: f64 = 10.0;
const HIGHEST_PRICE: f64 = BASE_PRICE + PRICE_OFFSET + 1.0;
const LOWEST_PRICE: f64 = BASE_PRICE - 1.0;

const INTERVAL: Duration = Duration::from_secs(1);

/// The seed of upstream's `Random` (`new(0x12291977)`). Upstream shares one static generator among
/// the tests of the class; here each fixture starts a generator of that seed, so that a test does
/// not depend on the order the tests run in.
const RANDOM_SEED: i32 = 0x12291977;

/// An equality comparer of prices as the operators take it.
type PriceEquality = Rc<dyn Fn(&Rc<MarketPrice>, &Rc<MarketPrice>) -> bool>;

/// The aggregator of the merged prices.
type PriceAggregator = ChangeSetAggregator<Rc<MarketPrice>, i32>;

/// A count of the constants as a `usize`.
fn count(value: i32) -> usize {
    value as usize
}

/// `Some` of an equality comparer of the domain, as the operators take it.
fn equality(comparer: MarketPriceEqualityComparer) -> Option<PriceEquality> {
    let comparer: PriceEquality = Rc::new(comparer);
    Some(comparer)
}

/// The fixture: the list of markets (`_marketList`), disposed with the fixture (`Dispose`), and the
/// generator of the random prices.
struct Fixture {
    market_list: Vec<Rc<Market>>,
    random: Randomizer,
}

impl Fixture {
    fn new() -> Self {
        Self { market_list: Vec::new(), random: Randomizer::new(RANDOM_SEED) }
    }

    /// `GetRandomPrice`.
    fn get_random_price(&self) -> f64 {
        MarketPrice::random_price(&self.random, BASE_PRICE, PRICE_OFFSET)
    }

    /// `GetRandomPrice` as a function that outlives the fixture (the generator is shared).
    fn price_generator(&self) -> impl Fn() -> f64 + 'static {
        let random = self.random.clone();
        move || MarketPrice::random_price(&random, BASE_PRICE, PRICE_OFFSET)
    }

    /// `Add`.
    fn add(&mut self, market: Rc<Market>) -> Rc<Market> {
        self.market_list.push(market.clone());
        market
    }

    /// `_marketList.AddRange(Enumerable.Range(0, count).Select(n => new Market(n)))`.
    fn add_markets(&mut self, count: i32) {
        self.market_list.extend((0..count).map(Market::from_number));
    }

    /// `_marketList.ForEach((m, index) => m.AddUniquePrices(index, PricesPerMarket, ItemIdStride,
    /// GetRandomPrice))`.
    fn add_unique_prices_to_all(&self) {
        for (index, market) in self.market_list.iter().enumerate() {
            market.add_unique_prices(index as i32, PRICES_PER_MARKET, ITEM_ID_STRIDE, || self.get_random_price());
        }
    }

    /// `_marketList.Select(m => m.LatestPrices)`.
    fn latest_prices(&self) -> Vec<PriceChangeSets> {
        self.market_list.iter().map(|market| market.latest_prices()).collect()
    }

    /// `_marketList.Sum(m => m.PricesCache.Count)`.
    fn total_prices(&self) -> usize {
        self.market_list.iter().map(|market| market.prices_cache().count()).sum()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for market in &self.market_list {
            market.dispose();
        }
    }
}

/// `results.Data.Items.Select(cp => cp.MarketId).ForEach(guid => guid.Should().Be(id))`.
fn assert_all_from_market(results: &PriceAggregator, id: Guid) {
    for price in results.data().items() {
        assert_eq!(price.market_id(), id);
    }
}

/// `results.Data.Items.Zip(market.PricesCache.Items).ForEach(pair =>
/// pair.First.Should().Be(pair.Second))`.
fn assert_items_match_market(results: &PriceAggregator, market: &Market) {
    let expected = market.prices_cache().items();
    for (actual, expected) in results.data().items().iter().zip(expected.iter()) {
        assert_eq!(actual, expected);
    }
}

/// `FixedMarket`s of `MarketCount` sections of `PricesPerMarket` items.
fn fixed_markets(fixture: &Fixture, completable: bool) -> Vec<Rc<FixedMarket>> {
    (0..MARKET_COUNT).map(|n| FixedMarket::new(fixture.price_generator(), n * ITEM_ID_STRIDE, (n * ITEM_ID_STRIDE) + PRICES_PER_MARKET, completable)).collect()
}

/// `fixedMarkets.Select(m => m.LatestPrices)`.
fn fixed_latest_prices(markets: &[Rc<FixedMarket>]) -> Vec<PriceChangeSets> {
    markets.iter().map(|market| market.latest_prices()).collect()
}

// Not ported: NullChecks - argument validation

#[test]
fn all_existing_items_present_in_result() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    fixture.add_unique_prices_to_all();

    // when
    let prices_cache = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_observable_cache();
    let results = prices_cache.connect().as_aggregator();

    // then
    assert_eq!(fixture.market_list.len(), count(MARKET_COUNT));
    assert_eq!(fixture.total_prices(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.data().count(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.messages().len(), 1);
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
    assert_eq!(summary.overall.refreshes, 0);

    drop(results);
    prices_cache.dispose();
}

#[test]
fn all_new_sub_items_present_in_result() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    let prices_cache = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_observable_cache();
    let results = prices_cache.connect().as_aggregator();

    // when
    fixture.add_unique_prices_to_all();

    // then
    assert_eq!(fixture.market_list.len(), count(MARKET_COUNT));
    assert_eq!(fixture.total_prices(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.data().count(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.messages().len(), count(MARKET_COUNT));
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
    assert_eq!(summary.overall.refreshes, 0);

    drop(results);
    prices_cache.dispose();
}

#[test]
fn all_refreshed_sub_items_are_refreshed() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    let results = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_unique_prices_to_all();

    // when
    for market in &fixture.market_list {
        market.refresh_all_prices(|| fixture.get_random_price());
    }

    // then
    assert_eq!(fixture.market_list.len(), count(MARKET_COUNT));
    assert_eq!(results.data().count(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.messages().len(), count(MARKET_COUNT * 2));
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
    assert_eq!(summary.overall.refreshes, count(MARKET_COUNT * PRICES_PER_MARKET));
}

#[test]
fn any_duplicate_key_values_should_be_hidden() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(2);
    let results = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_aggregator();

    // when
    fixture.market_list[0].set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    fixture.market_list[1].set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(results.data().count(), count(PRICES_PER_MARKET));
    assert_items_match_market(&results, &fixture.market_list[0]);
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
}

#[test]
fn any_duplicate_values_should_be_no_op_when_removed() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(2);
    let results = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_aggregator();
    fixture.market_list[0].set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    fixture.market_list[1].set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());

    // when
    fixture.market_list[1].remove_all_prices();

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(results.data().count(), count(PRICES_PER_MARKET));
    assert_items_match_market(&results, &fixture.market_list[0]);
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
}

#[test]
fn any_duplicate_values_should_be_unhidden_when_other_is_removed() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(2);
    let results = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_aggregator();
    fixture.market_list[0].set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    fixture.market_list[1].set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());

    // when
    fixture.market_list[0].remove_all_prices();

    // then
    assert_eq!(results.data().count(), count(PRICES_PER_MARKET));
    assert_items_match_market(&results, &fixture.market_list[1]);
    let messages = results.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].updates(), count(PRICES_PER_MARKET));
}

#[test]
fn any_duplicate_values_should_not_refresh_when_hidden() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(2);
    let results = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_aggregator();
    fixture.market_list[0].set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    fixture.market_list[1].set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());

    // when
    fixture.market_list[1].refresh_all_prices(|| fixture.get_random_price());

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(results.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(results.summary().overall.refreshes, 0);
    assert_items_match_market(&results, &fixture.market_list[0]);
}

#[test]
fn any_removed_sub_item_is_removed() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    let results = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_aggregator();
    fixture.add_unique_prices_to_all();

    // when
    for market in &fixture.market_list {
        market.prices_cache().edit(|updater| {
            let keys: Vec<i32> = updater.keys().into_iter().take(count(REMOVE_COUNT)).collect();
            updater.remove_keys(&mut keys.into_iter());
        });
    }

    // then
    assert_eq!(fixture.market_list.len(), count(MARKET_COUNT));
    assert_eq!(results.data().count(), count(MARKET_COUNT * (PRICES_PER_MARKET - REMOVE_COUNT)));
    let messages = results.messages();
    assert_eq!(messages.len(), count(MARKET_COUNT * 2));
    assert_eq!(messages[0].adds(), count(PRICES_PER_MARKET));
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, count(MARKET_COUNT * REMOVE_COUNT));
}

#[test]
fn comparer_only_adds_better_added_values() {
    // having
    let mut fixture = Fixture::new();
    let market_original = fixture.add(Market::from_number(0));
    let market_low = fixture.add(Market::from_number(1));
    let market_high = fixture.add(Market::from_number(2));
    let others: Vec<PriceChangeSets> = vec![market_low.latest_prices(), market_high.latest_prices()];
    let high_price_results = market_original.latest_prices().merge_change_sets_many_with(others.clone(), None, Some(MarketPrice::high_price_compare()), None, true).as_aggregator();
    let low_price_results = market_original.latest_prices().merge_change_sets_many_with(others, None, Some(MarketPrice::low_price_compare()), None, true).as_aggregator();
    market_original.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());

    // when
    market_low.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES_PER_MARKET, HIGHEST_PRICE);

    // then
    assert_eq!(fixture.market_list.len(), 3);
    assert_eq!(low_price_results.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(low_price_results.summary().overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(low_price_results.summary().overall.updates, count(PRICES_PER_MARKET));
    assert_all_from_market(&low_price_results, market_low.id());
    assert_eq!(high_price_results.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(high_price_results.summary().overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(high_price_results.summary().overall.updates, count(PRICES_PER_MARKET));
    assert_all_from_market(&high_price_results, market_high.id());
}

#[test]
fn comparer_only_adds_better_existing_values() {
    // having
    let mut fixture = Fixture::new();
    let market_original = fixture.add(Market::from_number(0));
    let market_low = fixture.add(Market::from_number(1));
    let market_high = fixture.add(Market::from_number(2));
    let others: Vec<PriceChangeSets> = vec![market_low.latest_prices(), market_high.latest_prices()];
    market_original.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market_low.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES_PER_MARKET, HIGHEST_PRICE);

    // when
    let high_price_results = market_original.latest_prices().merge_change_sets_many_with(others.clone(), None, Some(MarketPrice::high_price_compare()), None, true).as_aggregator();
    let low_price_results = market_original.latest_prices().merge_change_sets_many_with(others, None, Some(MarketPrice::low_price_compare()), None, true).as_aggregator();

    // then
    assert_eq!(fixture.market_list.len(), 3);
    assert_eq!(low_price_results.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(low_price_results.summary().overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(low_price_results.summary().overall.updates, count(PRICES_PER_MARKET));
    assert_all_from_market(&low_price_results, market_low.id());
    assert_eq!(high_price_results.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(high_price_results.summary().overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(high_price_results.summary().overall.updates, count(PRICES_PER_MARKET));
    assert_all_from_market(&high_price_results, market_high.id());
}

#[test]
fn comparer_updates_to_correct_value_on_refresh() {
    // having
    let mut fixture = Fixture::new();
    let market_original = fixture.add(Market::from_number(0));
    let market_flip_flop = fixture.add(Market::from_number(1));
    let high_price_results = market_original.latest_prices().merge_change_sets_ordered_by(market_flip_flop.latest_prices(), MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = market_original.latest_prices().merge_change_sets_ordered_by(market_flip_flop.latest_prices(), MarketPrice::low_price_compare()).as_aggregator();
    market_original.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market_flip_flop.set_prices_to(0, PRICES_PER_MARKET, HIGHEST_PRICE);

    // when
    market_flip_flop.refresh_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(low_price_results.data().count(), count(PRICES_PER_MARKET));
    let low = low_price_results.summary();
    assert_eq!(low.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(low.overall.removes, 0);
    assert_eq!(low.overall.updates, count(PRICES_PER_MARKET));
    assert_eq!(low.overall.refreshes, 0);
    assert_all_from_market(&low_price_results, market_flip_flop.id());
    assert_eq!(high_price_results.data().count(), count(PRICES_PER_MARKET));
    let high = high_price_results.summary();
    assert_eq!(high.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(high.overall.removes, 0);
    assert_eq!(high.overall.updates, count(PRICES_PER_MARKET * 2));
    assert_eq!(high.overall.refreshes, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn comparer_updates_to_correct_value_on_remove() {
    // having
    let mut fixture = Fixture::new();
    let market_original = fixture.add(Market::from_number(0));
    let market_low = fixture.add(Market::from_number(1));
    let market_high = fixture.add(Market::from_number(2));
    let results = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_aggregator();
    let low_price_results = fixture.latest_prices().merge_change_sets_ordered_by(MarketPrice::low_price_compare()).as_aggregator();
    let high_price_results = fixture.latest_prices().merge_change_sets_ordered_by(MarketPrice::high_price_compare()).as_aggregator();
    market_original.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market_low.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES_PER_MARKET, HIGHEST_PRICE);

    // when
    market_low.remove_all_prices();

    // then
    assert_eq!(fixture.market_list.len(), 3);
    assert_eq!(results.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(results.summary().overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(results.summary().overall.updates, 0);
    assert_all_from_market(&results, market_original.id());
    assert_eq!(low_price_results.data().count(), count(PRICES_PER_MARKET));
    let low = low_price_results.summary();
    assert_eq!(low.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(low.overall.removes, 0);
    assert_eq!(low.overall.updates, count(PRICES_PER_MARKET * 2));
    assert_all_from_market(&low_price_results, market_original.id());
    assert_eq!(high_price_results.data().count(), count(PRICES_PER_MARKET));
    let high = high_price_results.summary();
    assert_eq!(high.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(high.overall.removes, 0);
    assert_eq!(high.overall.updates, count(PRICES_PER_MARKET));
    assert_all_from_market(&high_price_results, market_high.id());
}

#[test]
fn comparer_updates_to_correct_value_on_update() {
    // having
    let mut fixture = Fixture::new();
    let market_original = fixture.add(Market::from_number(0));
    let market_flip_flop = fixture.add(Market::from_number(1));
    let high_price_results = fixture.latest_prices().merge_change_sets_ordered_by(MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = fixture.latest_prices().merge_change_sets_ordered_by(MarketPrice::low_price_compare()).as_aggregator();
    market_original.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market_flip_flop.set_prices_to(0, PRICES_PER_MARKET, HIGHEST_PRICE);

    // when
    market_flip_flop.update_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(low_price_results.data().count(), count(PRICES_PER_MARKET));
    let low = low_price_results.summary();
    assert_eq!(low.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(low.overall.removes, 0);
    assert_eq!(low.overall.updates, count(PRICES_PER_MARKET));
    assert_eq!(low.overall.refreshes, 0);
    assert_all_from_market(&low_price_results, market_flip_flop.id());
    assert_eq!(high_price_results.data().count(), count(PRICES_PER_MARKET));
    let high = high_price_results.summary();
    assert_eq!(high.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(high.overall.removes, 0);
    assert_eq!(high.overall.updates, count(PRICES_PER_MARKET * 2));
    assert_eq!(high.overall.refreshes, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn comparer_only_updates_visible_values_on_update() {
    // having
    let mut fixture = Fixture::new();
    let market_original = fixture.add(Market::from_number(0));
    let market_low = fixture.add(Market::from_number(1));
    let high_price_results = fixture.latest_prices().merge_change_sets_ordered_by(MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = fixture.latest_prices().merge_change_sets_ordered_by(MarketPrice::low_price_compare()).as_aggregator();
    market_original.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market_low.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);

    // when
    market_low.update_all_prices_to(LOWEST_PRICE - 1.0);

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(low_price_results.data().count(), count(PRICES_PER_MARKET));
    let low = low_price_results.summary();
    assert_eq!(low.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(low.overall.removes, 0);
    assert_eq!(low.overall.updates, count(PRICES_PER_MARKET * 2));
    assert_eq!(low.overall.refreshes, 0);
    assert_all_from_market(&low_price_results, market_low.id());
    assert_eq!(high_price_results.data().count(), count(PRICES_PER_MARKET));
    let high = high_price_results.summary();
    assert_eq!(high.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(high.overall.removes, 0);
    assert_eq!(high.overall.updates, 0);
    assert_eq!(high.overall.refreshes, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn comparer_only_refreshes_visible_values() {
    // having
    let mut fixture = Fixture::new();
    let market_original = fixture.add(Market::from_number(0));
    let market_low = fixture.add(Market::from_number(1));
    let high_price_results =
        fixture.latest_prices().merge_change_sets_with(equality(MarketPrice::equality_comparer()), Some(MarketPrice::high_price_compare()), None, true).as_aggregator();
    let low_price_results =
        fixture.latest_prices().merge_change_sets_with(equality(MarketPrice::equality_comparer()), Some(MarketPrice::low_price_compare()), None, true).as_aggregator();
    market_original.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market_low.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);

    // when
    market_low.refresh_all_prices_to(LOWEST_PRICE - 1.0);

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(low_price_results.data().count(), count(PRICES_PER_MARKET));
    let low = low_price_results.summary();
    assert_eq!(low.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(low.overall.removes, 0);
    assert_eq!(low.overall.updates, count(PRICES_PER_MARKET));
    assert_eq!(low.overall.refreshes, count(PRICES_PER_MARKET));
    assert_all_from_market(&low_price_results, market_low.id());
    assert_eq!(high_price_results.data().count(), count(PRICES_PER_MARKET));
    let high = high_price_results.summary();
    assert_eq!(high.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(high.overall.removes, 0);
    assert_eq!(high.overall.updates, 0);
    assert_eq!(high.overall.refreshes, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn enum_observable_uses_the_scheduler() {
    // having
    let scheduler = TestScheduler::new();
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    fixture.add_unique_prices_to_all();
    let prices_cache = fixture.latest_prices().merge_change_sets_with(equality(MarketPrice::equality_comparer()), None, Some(scheduler.scheduler()), true).as_observable_cache();
    let results = prices_cache.connect().as_aggregator();

    // when
    // Do not advance the scheduler so that nothing happens

    // then
    assert_eq!(fixture.market_list.len(), count(MARKET_COUNT));
    assert_eq!(fixture.total_prices(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.data().count(), 0);
    assert_eq!(results.messages().len(), 0);
    let summary = results.summary();
    assert_eq!(summary.overall.adds, 0);
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
    assert_eq!(summary.overall.refreshes, 0);

    drop(results);
    prices_cache.dispose();
}

#[test]
fn enum_observable_uses_the_scheduler_and_emits_all() {
    // having
    let scheduler = TestScheduler::new();
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    fixture.add_unique_prices_to_all();
    let prices_cache = fixture.latest_prices().merge_change_sets_with(equality(MarketPrice::equality_comparer()), None, Some(scheduler.scheduler()), true).as_observable_cache();
    let results = prices_cache.connect().as_aggregator();

    // when
    scheduler.advance_by(i64::from(MARKET_COUNT));

    // then
    assert_eq!(fixture.market_list.len(), count(MARKET_COUNT));
    assert_eq!(fixture.total_prices(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.data().count(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.messages().len(), count(MARKET_COUNT));
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
    assert_eq!(summary.overall.refreshes, 0);

    drop(results);
    prices_cache.dispose();
}

#[test]
fn equality_comparer_hides_updates_without_changes() {
    // having
    let mut fixture = Fixture::new();
    let market = fixture.add(Market::from_number(0));
    let results = fixture.latest_prices().merge_change_sets_with_comparer(MarketPrice::equality_comparer()).as_aggregator();
    market.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);

    // when
    market.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);

    // then
    assert_eq!(fixture.market_list.len(), 1);
    assert_eq!(results.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(results.messages().len(), 1);
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
    assert_eq!(summary.overall.refreshes, 0);
}

#[test]
fn equality_comparer_and_comparer_work_together_for_updates() {
    // having
    let mut fixture = Fixture::new();
    let market1 = fixture.add(Market::from_number(0));
    let market2 = fixture.add(Market::from_number(1));

    let results = market1
        .latest_prices()
        .merge_change_sets_with(market2.latest_prices(), equality(MarketPrice::equality_comparer()), Some(MarketPrice::latest_price_compare()), None, true)
        .as_aggregator();
    let results_time_stamp = market1
        .latest_prices()
        .merge_change_sets_with(market2.latest_prices(), equality(MarketPrice::equality_comparer_with_time_stamp()), Some(MarketPrice::latest_price_compare()), None, true)
        .as_aggregator();
    market1.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market2.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);

    // when
    market2.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(results.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(results.messages().len(), 2);
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.refreshes, 0);
    assert_eq!(results_time_stamp.messages().len(), 3);
    let summary = results_time_stamp.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, count(PRICES_PER_MARKET * 2));
    assert_eq!(summary.overall.refreshes, 0);
}

#[test]
fn equality_comparer_and_comparer_work_together_for_refreshes() {
    // having
    let mut fixture = Fixture::new();
    let market1 = fixture.add(Market::from_number(0));
    let market2 = fixture.add(Market::from_number(1));

    let results1 =
        fixture.latest_prices().merge_change_sets_with(equality(MarketPrice::equality_comparer()), Some(MarketPrice::latest_price_compare()), None, true).as_aggregator();
    let results2 = fixture
        .latest_prices()
        .merge_change_sets_with(equality(MarketPrice::equality_comparer_with_time_stamp()), Some(MarketPrice::latest_price_compare()), None, true)
        .as_aggregator();
    market1.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market2.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);
    // Update again, but only the timestamp will change, so results1 will ignore
    market2.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE);

    // when
    // results1 won't see the refresh because it ignored the update
    // results2 will see the refreshes because it didn't
    market2.refresh_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(results1.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(results1.messages().len(), 3);
    let summary = results1.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.refreshes, count(PRICES_PER_MARKET));
    assert_eq!(results2.messages().len(), 4);
    let summary = results2.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, count(PRICES_PER_MARKET * 2));
    assert_eq!(summary.overall.refreshes, count(PRICES_PER_MARKET));
}

#[test]
fn equality_comparer_and_comparer_refreshes_become_updates() {
    // having
    let mut fixture = Fixture::new();
    let market1 = fixture.add(Market::from_number(0));
    let market2 = fixture.add(Market::from_number(1));

    let results1 =
        fixture.latest_prices().merge_change_sets_with(equality(MarketPrice::equality_comparer()), Some(MarketPrice::latest_price_compare()), None, true).as_aggregator();
    let results2 = fixture
        .latest_prices()
        .merge_change_sets_with(equality(MarketPrice::equality_comparer_with_time_stamp()), Some(MarketPrice::latest_price_compare()), None, true)
        .as_aggregator();
    market1.set_prices(0, PRICES_PER_MARKET, || fixture.get_random_price());
    market2.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE - 1.0);
    // Update again, but only the timestamp will change, so results1 will ignore
    market2.set_prices_to(0, PRICES_PER_MARKET, LOWEST_PRICE - 1.0);

    // when
    // results1 will see this as an update because it ignored the last update
    // results2 will see the refreshes
    market2.refresh_all_prices(|| fixture.get_random_price());

    // then
    assert_eq!(fixture.market_list.len(), 2);
    assert_eq!(results1.data().count(), count(PRICES_PER_MARKET));
    assert_eq!(results1.messages().len(), 3);
    let summary = results1.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, count(PRICES_PER_MARKET * 2));
    assert_eq!(summary.overall.refreshes, 0);
    assert_eq!(results2.messages().len(), 4);
    let summary = results2.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, count(PRICES_PER_MARKET * 2));
    assert_eq!(summary.overall.refreshes, count(PRICES_PER_MARKET));
}

#[test]
fn every_item_visible_when_sequence_completes() {
    // having
    let fixture = Fixture::new();
    let fixed_market_list = fixed_markets(&fixture, true);

    // when
    let results = fixed_latest_prices(&fixed_market_list).merge_change_sets_with(None, None, None, true).as_aggregator();

    // then
    assert!(results.is_completed());
    assert_eq!(results.data().count(), count(PRICES_PER_MARKET * MARKET_COUNT));
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(PRICES_PER_MARKET * MARKET_COUNT));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
    assert_eq!(summary.overall.refreshes, 0);
}

/// `MergedObservableCompletesWhenAllSourcesComplete(bool completeSources)`.
fn merged_observable_completes_when_all_sources_complete(complete_sources: bool) {
    // having
    let fixture = Fixture::new();
    let fixed_market_list = fixed_markets(&fixture, complete_sources);

    // when
    let results = fixed_latest_prices(&fixed_market_list).merge_change_sets_with(None, None, None, true).as_aggregator();

    // then
    assert_eq!(results.is_completed(), complete_sources);
}

#[test]
fn merged_observable_completes_when_all_sources_complete_case1() {
    merged_observable_completes_when_all_sources_complete(false);
}

#[test]
fn merged_observable_completes_when_all_sources_complete_case2() {
    merged_observable_completes_when_all_sources_complete(true);
}

/// `MergedObservableRespectsCompletableFlag(bool completeSource, bool completeChildren)`.
fn merged_observable_respects_completable_flag(complete_source: bool, complete_children: bool) {
    // having
    let fixture = Fixture::new();
    let fixed_market_list = fixed_markets(&fixture, complete_children);

    // when
    let results = fixed_latest_prices(&fixed_market_list).merge_change_sets_with(None, None, None, complete_source).as_aggregator();

    // then
    assert_eq!(results.is_completed(), complete_source && complete_children);
}

#[test]
fn merged_observable_respects_completable_flag_case1() {
    merged_observable_respects_completable_flag(false, false);
}

#[test]
fn merged_observable_respects_completable_flag_case2() {
    merged_observable_respects_completable_flag(false, true);
}

#[test]
fn merged_observable_respects_completable_flag_case3() {
    merged_observable_respects_completable_flag(true, false);
}

#[test]
fn merged_observable_respects_completable_flag_case4() {
    merged_observable_respects_completable_flag(true, true);
}

/// `Observable.Interval(Interval, scheduler).Select(n => _marketList[(int)n].LatestPrices)`.
fn interval_of_latest_prices(fixture: &Fixture, scheduler: &TestScheduler) -> Rc<dyn IObservable<PriceChangeSets>> {
    let markets: Vec<Rc<Market>> = fixture.market_list.clone();
    let market_obs: Rc<dyn IObservable<Rc<Market>>> = Observable::interval_on(INTERVAL, scheduler.scheduler()).select(move |n: i64| markets[n as usize].clone());
    market_obs.select(|market: Rc<Market>| market.latest_prices())
}

#[test]
fn observable_observable_contains_all_added_values() {
    // having
    let scheduler = TestScheduler::new();
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    let observables = interval_of_latest_prices(&fixture, &scheduler);
    let results = ObservableCacheChangeSetsEx::merge_change_sets_with_comparer(&observables, MarketPrice::equality_comparer()).as_aggregator();
    for _ in 0..MARKET_COUNT {
        scheduler.advance_by(duration_to_ticks(INTERVAL));
    }

    // when
    fixture.add_unique_prices_to_all();

    // then
    assert_eq!(results.data().count(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.messages().len(), count(MARKET_COUNT));
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
}

#[test]
fn observable_observable_contains_all_existing_values() {
    // having
    let scheduler = TestScheduler::new();
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    fixture.add_unique_prices_to_all();
    let observables = interval_of_latest_prices(&fixture, &scheduler);
    let results = ObservableCacheChangeSetsEx::merge_change_sets_with_comparer(&observables, MarketPrice::equality_comparer()).as_aggregator();

    // when
    for _ in 0..MARKET_COUNT {
        scheduler.advance_by(duration_to_ticks(INTERVAL));
    }

    // then
    assert_eq!(results.data().count(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.messages().len(), count(MARKET_COUNT));
    let summary = results.summary();
    assert_eq!(summary.overall.adds, count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
}

#[test]
fn merged_observable_will_fail_if_any_change_change_set_fails() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    fixture.add_unique_prices_to_all();
    let expected_error: ObservableError = new_error("Test exception");
    let mut enum_observable = fixture.latest_prices();
    enum_observable.push(Observable::throw::<ChangeSet<Rc<MarketPrice>, i32>>(expected_error.clone()));

    // when
    let results = enum_observable.merge_change_sets().as_aggregator();

    // then
    assert_eq!(fixture.market_list.len(), count(MARKET_COUNT));
    assert_eq!(fixture.total_prices(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.data().count(), count(MARKET_COUNT * PRICES_PER_MARKET));
    let error = results.error().expect("the merged sequence should have failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}

#[test]
fn observable_observable_will_fail_if_source_fails() {
    // having
    let mut fixture = Fixture::new();
    fixture.add_markets(MARKET_COUNT);
    fixture.add_unique_prices_to_all();
    let expected_error: ObservableError = new_error("Test exception");
    let observables: Rc<dyn IObservable<PriceChangeSets>> = Observable::to_observable(fixture.latest_prices()).concat(Observable::throw::<PriceChangeSets>(expected_error.clone()));

    // when
    let results = ObservableCacheChangeSetsEx::merge_change_sets(&observables).as_aggregator();

    // then
    assert_eq!(fixture.market_list.len(), count(MARKET_COUNT));
    assert_eq!(fixture.total_prices(), count(MARKET_COUNT * PRICES_PER_MARKET));
    assert_eq!(results.data().count(), count(MARKET_COUNT * PRICES_PER_MARKET));
    let error = results.error().expect("the merged sequence should have failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}

/// `ObservableObservableCompletesIfAndOnlyIfSourceAndAllChildrenComplete(bool completeSource, bool
/// completeChildren)`.
fn observable_observable_completes_if_and_only_if_source_and_all_children_complete(complete_source: bool, complete_children: bool) {
    // having
    let fixture = Fixture::new();
    let fixed_markets = fixed_markets(&fixture, complete_children);
    let mut observable_observable: Rc<dyn IObservable<PriceChangeSets>> = Observable::to_observable(fixed_latest_prices(&fixed_markets));
    if !complete_source {
        observable_observable = observable_observable.concat(Observable::never::<PriceChangeSets>());
    }

    // when
    let results = ObservableCacheChangeSetsEx::merge_change_sets(&observable_observable).as_aggregator();

    // then
    assert_eq!(results.is_completed(), complete_source && complete_children);
}

#[test]
fn observable_observable_completes_if_and_only_if_source_and_all_children_complete_case1() {
    observable_observable_completes_if_and_only_if_source_and_all_children_complete(false, false);
}

#[test]
fn observable_observable_completes_if_and_only_if_source_and_all_children_complete_case2() {
    observable_observable_completes_if_and_only_if_source_and_all_children_complete(false, true);
}

#[test]
fn observable_observable_completes_if_and_only_if_source_and_all_children_complete_case3() {
    observable_observable_completes_if_and_only_if_source_and_all_children_complete(true, false);
}

#[test]
fn observable_observable_completes_if_and_only_if_source_and_all_children_complete_case4() {
    observable_observable_completes_if_and_only_if_source_and_all_children_complete(true, true);
}
