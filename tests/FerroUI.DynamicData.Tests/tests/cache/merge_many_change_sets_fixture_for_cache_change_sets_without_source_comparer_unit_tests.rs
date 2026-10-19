//! `MergeManyChangeSetsFixture.ForCacheChangeSets.WithoutSourceComparer.UnitTests`
//! (`DynamicData.Tests/Cache/MergeManyChangeSetsFixture.ForCacheChangeSets.WithoutSourceComparer.UnitTests.cs`):
//! the change sets of the caches of the items of a cache, merged, without a comparer of the
//! parents.
//!
//! The constants and helpers of the partial classes (`MergeManyChangeSetsFixture.ForCacheChangeSets`)
//! are written here privately, with the values of upstream's debug build.

use crate::domain::{FixedMarket, Guid, IMarket, Market, MarketPrice, PriceChangeSets, Randomizer};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeReason, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

const MARKET_COUNT: usize = 5;
const PRICES_PER_MARKET: usize = 7;
const REMOVE_COUNT: usize = 3;

const ITEM_ID_STRIDE: i32 = 1000;
const BASE_PRICE: f64 = 10.0;
const PRICE_OFFSET: f64 = 10.0;
const HIGHEST_PRICE: f64 = BASE_PRICE + PRICE_OFFSET + 1.0;
const LOWEST_PRICE: f64 = BASE_PRICE - 1.0;

/// The seed of upstream's randomizers.
const SEED: i32 = 0x2112_3737;

type MarketCache = SourceCache<Rc<dyn IMarket>, Guid>;

/// The equality comparer of prices as an optional argument of the full overload.
type PriceEquality = Rc<dyn Fn(&Rc<MarketPrice>, &Rc<MarketPrice>) -> bool>;

/// The error of `MergedObservableWillFailIfSourceFails` (`new Exception("Test exception")`).
#[derive(Debug)]
struct TestError;

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Test exception")
    }
}

impl std::error::Error for TestError {}

/// `GetRandomPrice`.
fn get_random_price(randomizer: &Randomizer) -> f64 {
    MarketPrice::random_price(randomizer, BASE_PRICE, PRICE_OFFSET)
}

/// `new SourceCache<IMarket, Guid>(p => p.Id)`.
fn new_market_cache() -> MarketCache {
    SourceCache::new(|market: &Rc<dyn IMarket>| market.id())
}

/// A market as the item of the cache of markets.
fn as_market(market: &Rc<Market>) -> Rc<dyn IMarket> {
    market.clone()
}

/// `m => m.LatestPrices`.
fn select_latest_prices(market: &Rc<dyn IMarket>) -> PriceChangeSets {
    market.latest_prices()
}

/// `Enumerable.Range(0, count).Select(n => new Market(n)).ToArray()`.
fn new_markets(count: usize) -> Vec<Rc<Market>> {
    (0..count as i32).map(Market::from_number).collect()
}

/// `marketCache.AddOrUpdate(market)`.
fn add_market(cache: &MarketCache, market: &Rc<Market>) {
    let item = as_market(market);
    cache.edit(|updater| updater.add_or_update(item));
}

/// `marketCache.AddOrUpdate(markets)`.
fn add_markets(cache: &MarketCache, markets: &[Rc<Market>]) {
    cache.edit(|updater| updater.add_or_update_items(&mut markets.iter().map(as_market)));
}

/// `AddUniquePrices(Market[], Randomizer)`.
fn add_unique_prices(markets: &[Rc<Market>], randomizer: &Randomizer) {
    for market in markets {
        market.add_unique_prices_count(PRICES_PER_MARKET, |_| get_random_price(randomizer));
    }
}

/// `MarketCount` fixed markets of `PricesPerMarket` prices each, with item identifiers
/// `n * ItemIdStride ..`.
fn fixed_markets(randomizer: &Randomizer, completable: bool) -> Vec<Rc<dyn IMarket>> {
    (0..MARKET_COUNT as i32)
        .map(|n| {
            let randomizer = randomizer.clone();
            let market: Rc<dyn IMarket> = FixedMarket::new(move || get_random_price(&randomizer), n * ITEM_ID_STRIDE, (n * ITEM_ID_STRIDE) + PRICES_PER_MARKET as i32, completable);
            market
        })
        .collect()
}

/// `results.Data.Items.Zip(expected).ForEach(pair => pair.First.Should().Be(pair.Second))`.
fn assert_zip_equal(actual: &[Rc<MarketPrice>], expected: &[Rc<MarketPrice>]) {
    for (first, second) in actual.iter().zip(expected.iter()) {
        assert_eq!(first, second);
    }
}

/// `results.Data.Items.Select(cp => cp.MarketId).ForEach(guid => guid.Should().Be(id))`.
fn assert_all_from_market(results: &ChangeSetAggregator<Rc<MarketPrice>, i32>, id: Guid) {
    for price in results.data().items() {
        assert_eq!(price.market_id(), id);
    }
}

#[test]
fn able_to_invoke_factory() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    // having
    let invoked = Rc::new(Cell::new(false));
    let factory = {
        let invoked = invoked.clone();
        move |m: &Rc<dyn IMarket>| -> PriceChangeSets {
            invoked.set(true);
            m.latest_prices()
        }
    };
    let _sub = market_cache.connect().merge_many_change_sets(factory).subscribe_ignore();

    // when
    add_market(&market_cache, &Market::from_number(0));

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert!(invoked.get());
}

#[test]
fn able_to_invoke_factory_with_key() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    // having
    let invoked = Rc::new(Cell::new(false));
    let factory = {
        let invoked = invoked.clone();
        move |m: &Rc<dyn IMarket>, _g: &Guid| -> PriceChangeSets {
            invoked.set(true);
            m.latest_prices()
        }
    };
    let _sub = market_cache.connect().merge_many_change_sets_with_key(factory).subscribe_ignore();

    // when
    add_market(&market_cache, &Market::from_number(0));

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert!(invoked.get());
}

#[test]
fn all_existing_sub_items_present_in_result() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_unique_prices(&markets, &randomizer);

    // when
    add_markets(&market_cache, &markets);

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT);
    assert_eq!(markets.iter().map(|m| m.prices_cache().count()).sum::<usize>(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), 1);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn all_new_sub_items_present_in_result() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_markets(&market_cache, &markets);

    // when
    add_unique_prices(&markets, &randomizer);

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT);
    assert_eq!(markets.iter().map(|m| m.prices_cache().count()).sum::<usize>(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), MARKET_COUNT);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn all_refreshed_sub_items_are_refreshed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_markets(&market_cache, &markets);
    add_unique_prices(&markets, &randomizer);

    // when
    for market in &markets {
        market.refresh_all_prices(|| get_random_price(&randomizer));
    }

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), MARKET_COUNT * 2);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
    assert_eq!(results.summary().overall.refreshes, MARKET_COUNT * PRICES_PER_MARKET);
}

#[test]
fn any_duplicate_key_values_should_be_hidden() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_markets(&market_cache, &markets);

    // when
    markets[0].set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_zip_equal(&results.data().items(), &markets[0].prices_cache().items());
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn any_duplicate_values_should_be_no_op_when_removed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_markets(&market_cache, &markets);
    markets[0].set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));

    // when
    markets[1].remove_all_prices();

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_zip_equal(&results.data().items(), &markets[0].prices_cache().items());
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn any_duplicate_values_should_be_unhidden_when_other_is_removed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_markets(&market_cache, &markets);
    markets[0].set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));

    // when
    let removed = as_market(&markets[0]);
    market_cache.edit(|updater| updater.remove_item(&removed));

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_zip_equal(&results.data().items(), &markets[1].prices_cache().items());
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.messages()[1].updates(), PRICES_PER_MARKET);
}

#[test]
fn any_duplicate_values_should_not_refresh_when_hidden() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_markets(&market_cache, &markets);
    markets[0].set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));

    // when
    markets[1].refresh_all_prices(|| get_random_price(&randomizer));

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.refreshes, 0);
    assert_zip_equal(&results.data().items(), &markets[0].prices_cache().items());
}

#[test]
fn any_removed_sub_item_is_removed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_markets(&market_cache, &markets);
    add_unique_prices(&markets, &randomizer);

    // when
    for market in &markets {
        market.prices_cache().edit(|updater| {
            let keys: Vec<i32> = updater.keys().into_iter().take(REMOVE_COUNT).collect();
            updater.remove_keys(&mut keys.into_iter());
        });
    }

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT);
    assert_eq!(results.data().count(), MARKET_COUNT * (PRICES_PER_MARKET - REMOVE_COUNT));
    assert_eq!(results.messages().len(), MARKET_COUNT * 2);
    assert_eq!(results.messages()[0].adds(), PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, MARKET_COUNT * REMOVE_COUNT);
}

#[test]
fn any_source_item_removed_removes_all_source_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_unique_prices(&markets, &randomizer);
    add_markets(&market_cache, &markets);

    // when
    market_cache.edit(|updater| {
        let keys: Vec<Guid> = updater.keys().into_iter().take(REMOVE_COUNT).collect();
        updater.remove_keys(&mut keys.into_iter());
    });

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT - REMOVE_COUNT);
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.data().count(), (MARKET_COUNT - REMOVE_COUNT) * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, PRICES_PER_MARKET * REMOVE_COUNT);
}

#[test]
fn changing_source_by_update_removes_previous_and_adds_new_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    let market = Market::from_number(0);
    market.set_prices(0, (PRICES_PER_MARKET * 2) as i32, || get_random_price(&randomizer));
    add_market(&market_cache, &market);
    let updated_market = Market::copy_of(&market);
    updated_market.set_prices(PRICES_PER_MARKET as i32, (PRICES_PER_MARKET * 3) as i32, || get_random_price(&randomizer));

    // when
    add_market(&market_cache, &updated_market);

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET * 2);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET * 3);
    assert_eq!(results.summary().overall.updates, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, PRICES_PER_MARKET);
    assert_zip_equal(&results.data().items(), &updated_market.prices_cache().items());
}

#[test]
fn clearing_parent_emits_single_change_set() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    add_unique_prices(&markets, &randomizer);
    add_markets(&market_cache, &markets);

    // when
    market_cache.edit(|updater| updater.clear());

    // then
    assert_eq!(market_cache_results.data().count(), 0);
    assert_eq!(results.data().count(), 0);
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.summary().overall.adds, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.updates, 0);
}

#[test]
fn comparer_only_adds_better_added_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let high_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_high = Market::from_number(2);
    market_original.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    add_market(&market_cache, &market_original);
    add_market(&market_cache, &market_low);
    add_market(&market_cache, &market_high);

    // when
    market_low.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES_PER_MARKET as i32, HIGHEST_PRICE);

    // then
    assert_eq!(market_cache_results.data().count(), 3);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from_market(&low_price_results, market_low.id());
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from_market(&high_price_results, market_high.id());
}

#[test]
fn comparer_only_adds_better_existing_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let high_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_high = Market::from_number(2);
    market_original.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    add_market(&market_cache, &market_original);
    market_low.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES_PER_MARKET as i32, HIGHEST_PRICE);

    // when
    add_market(&market_cache, &market_low);
    add_market(&market_cache, &market_high);

    // then
    assert_eq!(market_cache_results.data().count(), 3);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from_market(&low_price_results, market_low.id());
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from_market(&high_price_results, market_high.id());
}

#[test]
fn comparer_only_adds_better_values_on_source_update() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let high_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_low_low = Market::copy_of(&market_low);
    market_original.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    market_low.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE);
    market_low_low.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE - 1.0);
    add_market(&market_cache, &market_original);
    add_market(&market_cache, &market_low);

    // when
    add_market(&market_cache, &market_low_low);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_all_from_market(&low_price_results, market_low_low.id());
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn comparer_only_refreshes_visible_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let high_equality: PriceEquality = Rc::new(MarketPrice::equality_comparer());
    let low_equality: PriceEquality = Rc::new(MarketPrice::equality_comparer());
    let high_price_results = market_cache
        .connect()
        .merge_many_change_sets_with(|m: &Rc<dyn IMarket>, _: &Guid| m.latest_prices(), Some(high_equality), Some(MarketPrice::high_price_compare()))
        .as_aggregator();
    let low_price_results = market_cache
        .connect()
        .merge_many_change_sets_with(|m: &Rc<dyn IMarket>, _: &Guid| m.latest_prices(), Some(low_equality), Some(MarketPrice::low_price_compare()))
        .as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    market_original.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    market_low.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE);
    add_market(&market_cache, &market_original);
    add_market(&market_cache, &market_low);

    // when
    market_low.refresh_all_prices_to(LOWEST_PRICE - 1.0);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.refreshes, PRICES_PER_MARKET);
    assert_all_from_market(&low_price_results, market_low.id());
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, 0);
    assert_eq!(high_price_results.summary().overall.refreshes, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn comparer_only_updates_visible_values_on_update() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let high_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    market_original.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    market_low.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE);
    add_market(&market_cache, &market_original);
    add_market(&market_cache, &market_low);

    // when
    market_low.update_all_prices_to(LOWEST_PRICE - 1.0);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_eq!(low_price_results.summary().overall.refreshes, 0);
    assert_all_from_market(&low_price_results, market_low.id());
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, 0);
    assert_eq!(high_price_results.summary().overall.refreshes, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn comparer_updates_to_correct_value_on_refresh() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let high_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_flip_flop = Market::from_number(1);
    market_original.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    market_flip_flop.set_prices_to(0, PRICES_PER_MARKET as i32, HIGHEST_PRICE);
    add_market(&market_cache, &market_original);
    add_market(&market_cache, &market_flip_flop);

    // when
    market_flip_flop.refresh_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.refreshes, 0);
    assert_all_from_market(&low_price_results, market_flip_flop.id());
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_eq!(high_price_results.summary().overall.refreshes, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn comparer_updates_to_correct_value_on_remove() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    let low_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let high_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_high = Market::from_number(2);
    market_original.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    add_market(&market_cache, &market_original);
    add_market(&market_cache, &market_low);
    add_market(&market_cache, &market_high);
    market_low.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE);
    market_high.set_prices_to(0, PRICES_PER_MARKET as i32, HIGHEST_PRICE);

    // when
    let removed = as_market(&market_low);
    market_cache.edit(|updater| updater.remove_item(&removed));

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.updates, 0);
    assert_all_from_market(&results, market_original.id());
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_all_from_market(&low_price_results, market_original.id());
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_all_from_market(&high_price_results, market_high.id());
}

#[test]
fn comparer_updates_to_correct_value_on_update() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let high_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::high_price_compare()).as_aggregator();
    let low_price_results = market_cache.connect().merge_many_change_sets_ordered_by(select_latest_prices, MarketPrice::low_price_compare()).as_aggregator();
    let market_original = Market::from_number(0);
    let market_flip_flop = Market::from_number(1);
    market_original.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    market_flip_flop.set_prices_to(0, PRICES_PER_MARKET as i32, HIGHEST_PRICE);
    add_market(&market_cache, &market_original);
    add_market(&market_cache, &market_flip_flop);

    // when
    market_flip_flop.update_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(low_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.removes, 0);
    assert_eq!(low_price_results.summary().overall.updates, PRICES_PER_MARKET);
    assert_eq!(low_price_results.summary().overall.refreshes, 0);
    assert_all_from_market(&low_price_results, market_flip_flop.id());
    assert_eq!(high_price_results.data().count(), PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(high_price_results.summary().overall.removes, 0);
    assert_eq!(high_price_results.summary().overall.updates, PRICES_PER_MARKET * 2);
    assert_eq!(high_price_results.summary().overall.refreshes, 0);
    assert_all_from_market(&high_price_results, market_original.id());
}

#[test]
fn equality_comparer_hides_updates_without_changes() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    // having
    let market = Market::from_number(0);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    market.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE);
    add_market(&market_cache, &market);

    // when
    market.set_prices_to(0, PRICES_PER_MARKET as i32, LOWEST_PRICE);

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), 1);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
    assert_eq!(results.summary().overall.refreshes, 0);
}

#[test]
fn every_item_visible_when_sequence_completes() {
    let market_cache = new_market_cache();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = fixed_markets(&randomizer, true);
    market_cache.edit(|updater| updater.add_or_update_items(&mut markets.into_iter()));

    // when
    let results = market_cache.connect().merge_many_change_sets(select_latest_prices).as_aggregator();

    // `marketCache.Items.ForEach(m => (m as IDisposable)?.Dispose())`: a fixed market is not
    // disposable, so only the cache is disposed.
    market_cache.dispose();

    // then
    assert_eq!(results.data().count(), PRICES_PER_MARKET * MARKET_COUNT);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET * MARKET_COUNT);
    assert_eq!(results.summary().overall.removes, 0);
    assert_eq!(results.summary().overall.updates, 0);
    assert_eq!(results.summary().overall.refreshes, 0);
}

/// `MergedObservableCompletesOnlyWhenSourceAndAllChildrenComplete(bool completeSource, bool
/// completeChildren)`.
fn merged_observable_completes_only_when_source_and_all_children_complete(complete_source: bool, complete_children: bool) {
    let market_cache = new_market_cache();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = fixed_markets(&randomizer, complete_children);
    market_cache.edit(|updater| updater.add_or_update_items(&mut markets.into_iter()));
    let has_source_sequence_completed = Rc::new(Cell::new(false));
    let has_merged_sequence_completed = Rc::new(Cell::new(false));

    let source_completed = has_source_sequence_completed.clone();
    let merged_completed = has_merged_sequence_completed.clone();
    let _cleanup = market_cache
        .connect()
        .do_with_completed(|_| {}, move || source_completed.set(true))
        .merge_many_change_sets(select_latest_prices)
        .subscribe_next_completed(|_| {}, move || merged_completed.set(true));

    // when
    if complete_source {
        // A fixed market is not disposable: only the cache is disposed.
        market_cache.dispose();
    }

    // then
    assert_eq!(has_source_sequence_completed.get(), complete_source);
    assert_eq!(has_merged_sequence_completed.get(), complete_source && complete_children);
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
    let market_cache = new_market_cache();

    // having
    let markets = new_markets(MARKET_COUNT);
    add_markets(&market_cache, &markets);
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let expected_error: ObservableError = Rc::new(TestError);
    let throw_observable = Observable::throw::<ChangeSet<Rc<dyn IMarket>, Guid>>(expected_error.clone());

    let received = received_error.clone();
    let _cleanup = market_cache
        .connect()
        .concat(throw_observable)
        .merge_many_change_sets(select_latest_prices)
        .subscribe_next_error(|_| {}, move |error| *received.borrow_mut() = Some(error));

    // when
    for market in &markets {
        market.dispose();
    }
    market_cache.dispose();

    // then
    let received_error = received_error.borrow().clone().expect("the merged sequence should fail");
    assert!(Rc::ptr_eq(&received_error, &expected_error));
}

#[test]
fn merge_many_change_sets_works_correctly_with_value_types() {
    let market_cache = new_market_cache();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    add_markets(&market_cache, &markets);
    for market in &markets {
        market.set_prices(0, PRICES_PER_MARKET as i32, || get_random_price(&randomizer));
    }
    let results = market_cache
        .connect()
        .merge_many_change_sets(|m: &Rc<dyn IMarket>| m.latest_prices().transform(|p: &Rc<MarketPrice>| p.price()))
        .as_aggregator();

    // when
    for market in &markets {
        market.remove_all_prices();
    }

    // then
    assert_eq!(results.data().count(), 0);
    assert_eq!(results.summary().overall.adds, PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.removes, PRICES_PER_MARKET);
}

// Not ported: NullChecks - argument validation

/// `OrderOfChangesIsPreserved(bool removeFirst)`.
fn order_of_changes_is_preserved(remove_first: bool) {
    let market_cache = new_market_cache();

    let randomizer = Randomizer::new(SEED);

    // Arrange
    let markets = new_markets(MARKET_COUNT);
    add_unique_prices(&markets, &randomizer);
    add_markets(&market_cache, &markets);
    let markets2 = new_markets(MARKET_COUNT);
    add_unique_prices(&markets2, &randomizer);
    let results = market_cache.connect().merge_many_change_sets_with_comparer(select_latest_prices, MarketPrice::equality_comparer()).as_aggregator();
    let (first_reason, next_reason, expected_changes) = if remove_first {
        (ChangeReason::Remove, ChangeReason::Add, 2 * MARKET_COUNT * PRICES_PER_MARKET)
    } else {
        (ChangeReason::Add, ChangeReason::Remove, 3 * MARKET_COUNT * PRICES_PER_MARKET)
    };

    // Act
    market_cache.edit(|updater| {
        if remove_first {
            updater.clear();
            updater.add_or_update_items(&mut markets2.iter().map(as_market));
        } else {
            updater.add_or_update_items(&mut markets2.iter().map(as_market));
            updater.clear();
        }
    });

    // Assert
    let messages = results.messages();
    assert_eq!(messages.len(), 2);
    assert!(messages[0].changes().iter().all(|change| change.reason() == ChangeReason::Add));
    assert_eq!(messages[1].count(), expected_changes);
    assert!(messages[1].changes().iter().take(MARKET_COUNT * PRICES_PER_MARKET).all(|change| change.reason() == first_reason));
    assert!(messages[1].changes().iter().skip(MARKET_COUNT * PRICES_PER_MARKET).all(|change| change.reason() == next_reason));
}

#[test]
fn order_of_changes_is_preserved_case1() {
    order_of_changes_is_preserved(true);
}

#[test]
fn order_of_changes_is_preserved_case2() {
    order_of_changes_is_preserved(false);
}
