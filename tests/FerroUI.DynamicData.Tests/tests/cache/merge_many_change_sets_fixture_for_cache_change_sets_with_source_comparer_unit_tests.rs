//! `MergeManyChangeSetsFixture.ForCacheChangeSets.WithSourceComparer.UnitTests`
//! (`DynamicData.Tests/Cache/MergeManyChangeSetsFixture.ForCacheChangeSets.WithSourceComparer.UnitTests.cs`):
//! the change sets of the prices of markets, merged so that the prices of the best market (by a
//! comparer of the markets) win, then those of the best price (by a comparer of the prices).
//!
//! The constants and helpers of the partial classes (`MergeManyChangeSetsFixture.ForCacheChangeSets`)
//! are repeated here, with the values of a debug build. Upstream's `DebugSpy` calls, which only
//! log, are left out.

use crate::domain::{FixedMarket, Guid, IMarket, Market, MarketPrice, MarketPriceComparer, MarketPriceEqualityComparer, PriceChangeSets, Randomizer};
use crate::utilities::inverted_comparer;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, ObservableError};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

// The constants of `MergeManyChangeSetsFixture.ForCacheChangeSets` (the `DEBUG` values).
const MARKET_COUNT: usize = 5;
const PRICES_PER_MARKET: usize = 7;
const REMOVE_COUNT: usize = 3;
const ITEM_ID_STRIDE: i32 = 1000;
const BASE_PRICE: f64 = 10.0;
const PRICE_OFFSET: f64 = 10.0;
const HIGHEST_PRICE: f64 = BASE_PRICE + PRICE_OFFSET + 1.0;
const LOWEST_PRICE: f64 = BASE_PRICE - 1.0;

/// `PricesPerMarket` as an item identifier.
const PRICES_PER_MARKET_ID: i32 = PRICES_PER_MARKET as i32;

/// The seed of the randomizers of the tests.
const SEED: i32 = 0x10012022;

type MarketCache = SourceCache<Rc<dyn IMarket>, Guid>;
type MarketComparer = Rc<dyn Fn(&Rc<dyn IMarket>, &Rc<dyn IMarket>) -> Ordering>;
type PriceEquality = Rc<dyn Fn(&Rc<MarketPrice>, &Rc<MarketPrice>) -> bool>;
type PriceResults = ChangeSetAggregator<Rc<MarketPrice>, i32>;

/// `new SourceCache<IMarket, Guid>(p => p.Id)`.
fn new_market_cache() -> MarketCache {
    SourceCache::new(|market: &Rc<dyn IMarket>| market.id())
}

/// `GetRandomPrice`.
fn get_random_price(randomizer: &Randomizer) -> f64 {
    MarketPrice::random_price(randomizer, BASE_PRICE, PRICE_OFFSET)
}

/// `Enumerable.Range(0, count).Select(n => new Market(n)).ToArray()`.
fn new_markets(count: usize) -> Vec<Rc<Market>> {
    (0..count as i32).map(Market::from_number).collect()
}

/// `marketCache.AddOrUpdate(markets)`.
fn add_or_update<M: IMarket + 'static>(market_cache: &MarketCache, markets: &[Rc<M>]) {
    market_cache.edit(|updater| updater.add_or_update_items(&mut markets.iter().map(|market| market.clone() as Rc<dyn IMarket>)));
}

/// `marketCache.Remove(market)`.
fn remove(market_cache: &MarketCache, market: &Rc<Market>) {
    let key = market.id();
    market_cache.edit(|updater| updater.remove_key(&key));
}

/// `SetRating`: sets the rating of the market and refreshes it in the cache.
fn set_rating(market_cache: &MarketCache, market: &Rc<Market>, new_rating: f64) {
    market.set_rating(new_rating);
    let key = market.id();
    market_cache.edit(|updater| updater.refresh_key(&key));
}

/// `markets.Select((m, index) => ..).ForEach(m => m.Market.SetPrices(m.Index * ItemIdStride,
/// (m.Index * ItemIdStride) + PricesPerMarket, () => GetRandomPrice(randomizer)))`.
fn set_sectioned_prices(markets: &[Rc<Market>], randomizer: &Randomizer) {
    for (index, market) in markets.iter().enumerate() {
        let index = index as i32;
        market.set_prices(index * ITEM_ID_STRIDE, index * ITEM_ID_STRIDE + PRICES_PER_MARKET_ID, || get_random_price(randomizer));
    }
}

/// The overall counts of the results (`Summary.Overall.Adds`, `Removes`, `Updates`, `Refreshes`).
fn assert_overall(results: &PriceResults, adds: usize, removes: usize, updates: usize, refreshes: usize) {
    let overall = results.summary().overall;
    assert_eq!(overall.adds, adds, "Summary.Overall.Adds");
    assert_eq!(overall.removes, removes, "Summary.Overall.Removes");
    assert_eq!(overall.updates, updates, "Summary.Overall.Updates");
    assert_eq!(overall.refreshes, refreshes, "Summary.Overall.Refreshes");
}

/// `results.Data.Items.Select(cp => cp.MarketId).ForEach(guid => guid.Should().Be(marketId))`.
fn assert_all_from(results: &PriceResults, market_id: Guid) {
    for price in results.data().items() {
        assert_eq!(price.market_id(), market_id);
    }
}

/// `results.Data.Items.Zip(market.PricesCache.Items).ForEach(pair => pair.First.Should().Be(pair.Second))`.
fn assert_same_prices(results: &PriceResults, market: &Market) {
    let actual = results.data().items();
    let expected = market.prices_cache().items();
    for (actual, expected) in actual.iter().zip(expected.iter()) {
        assert_eq!(actual, expected);
    }
}

fn to_equality(equality_comparer: MarketPriceEqualityComparer) -> PriceEquality {
    Rc::new(equality_comparer)
}

/// `CreateChangeSet`: the prices of the markets, merged by `source_comp` (the rating when `None`),
/// then by `child_compare`, with `equality_comparer`.
fn create_change_set(
    market_cache: &MarketCache,
    source_comp: Option<MarketComparer>,
    child_compare: Option<MarketPriceComparer>,
    equality_comparer: Option<MarketPriceEqualityComparer>,
    resort_on_refresh: bool,
) -> PriceChangeSets {
    market_cache.connect().merge_many_change_sets_with_source_comparer_with(
        |market: &Rc<dyn IMarket>, _: &Guid| market.latest_prices(),
        source_comp.unwrap_or_else(Market::rating_compare),
        resort_on_refresh,
        equality_comparer.map(to_equality),
        child_compare,
    )
}

/// `ChangeSetByRating`.
fn change_set_by_rating(market_cache: &MarketCache, resort_on_refresh: bool) -> PriceChangeSets {
    create_change_set(market_cache, None, None, None, resort_on_refresh)
}

/// `ChangeSetByRatingThenHighPrice`.
fn change_set_by_rating_then_high_price(market_cache: &MarketCache, resort_on_refresh: bool) -> PriceChangeSets {
    create_change_set(market_cache, Some(Market::rating_compare()), Some(MarketPrice::high_price_compare()), None, resort_on_refresh)
}

/// `ChangeSetByRatingThenLowPrice`.
fn change_set_by_rating_then_low_price(market_cache: &MarketCache, resort_on_refresh: bool) -> PriceChangeSets {
    create_change_set(market_cache, Some(Market::rating_compare()), Some(MarketPrice::low_price_compare()), None, resort_on_refresh)
}

/// `ChangeSetByRatingThenRecent`.
fn change_set_by_rating_then_recent(market_cache: &MarketCache, resort_on_refresh: bool) -> PriceChangeSets {
    create_change_set(market_cache, Some(Market::rating_compare()), Some(MarketPrice::latest_price_compare()), Some(MarketPrice::equality_comparer()), resort_on_refresh)
}

/// `ChangeSetByRatingThenTimeStamp`.
fn change_set_by_rating_then_time_stamp(market_cache: &MarketCache, resort_on_refresh: bool) -> PriceChangeSets {
    create_change_set(
        market_cache,
        Some(Market::rating_compare()),
        Some(MarketPrice::latest_price_compare()),
        Some(MarketPrice::equality_comparer_with_time_stamp()),
        resort_on_refresh,
    )
}

/// `ChangeSetByLowRating`.
fn change_set_by_low_rating(market_cache: &MarketCache, resort_on_refresh: bool) -> PriceChangeSets {
    create_change_set(market_cache, Some(inverted_comparer(Market::rating_compare())), None, None, resort_on_refresh)
}

/// `ChangeSetByLowRatingThenHighPrice`.
fn change_set_by_low_rating_then_high_price(market_cache: &MarketCache, resort_on_refresh: bool) -> PriceChangeSets {
    create_change_set(market_cache, Some(inverted_comparer(Market::rating_compare())), Some(MarketPrice::high_price_compare()), None, resort_on_refresh)
}

/// `ChangeSetByLowRatingThenLowPrice`.
fn change_set_by_low_rating_then_low_price(market_cache: &MarketCache, resort_on_refresh: bool) -> PriceChangeSets {
    create_change_set(market_cache, Some(inverted_comparer(Market::rating_compare())), Some(MarketPrice::low_price_compare()), None, resort_on_refresh)
}

#[test]
fn able_to_invoke_factory() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    // having
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let factory = move |market: &Rc<dyn IMarket>| -> PriceChangeSets {
        flag.set(true);
        market.latest_prices()
    };
    let sub = market_cache.connect().merge_many_change_sets_with_source_comparer(factory, Market::rating_compare()).subscribe_ignore();

    // when
    add_or_update(&market_cache, &[Market::from_number(0)]);

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert!(invoked.get());

    sub.dispose();
    market_cache.dispose();
}

#[test]
fn able_to_invoke_factory_with_key() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    // having
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let factory = move |market: &Rc<dyn IMarket>, _: &Guid| -> PriceChangeSets {
        flag.set(true);
        market.latest_prices()
    };
    let sub = market_cache.connect().merge_many_change_sets_with_source_comparer_with(factory, Market::rating_compare(), true, None, None).subscribe_ignore();

    // when
    add_or_update(&market_cache, &[Market::from_number(0)]);

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert!(invoked.get());

    sub.dispose();
    market_cache.dispose();
}

#[test]
fn all_existing_sub_items_present_in_result() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    set_sectioned_prices(&markets, &randomizer);

    // when
    add_or_update(&market_cache, &markets);

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT);
    assert_eq!(markets.iter().map(|market| market.prices_cache().count()).sum::<usize>(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), 1);
    assert_overall(&results, MARKET_COUNT * PRICES_PER_MARKET, 0, 0, 0);
}

#[test]
fn all_new_sub_items_present_in_result() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    add_or_update(&market_cache, &markets);

    // when
    set_sectioned_prices(&markets, &randomizer);

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT);
    assert_eq!(markets.iter().map(|market| market.prices_cache().count()).sum::<usize>(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), MARKET_COUNT);
    assert_overall(&results, MARKET_COUNT * PRICES_PER_MARKET, 0, 0, 0);
}

#[test]
fn all_refreshed_sub_items_are_refreshed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    set_sectioned_prices(&markets, &randomizer);
    add_or_update(&market_cache, &markets);

    // when
    for market in &markets {
        market.refresh_all_prices(|| get_random_price(&randomizer));
    }

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT);
    assert_eq!(results.data().count(), MARKET_COUNT * PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), MARKET_COUNT + 1);
    assert_overall(&results, MARKET_COUNT * PRICES_PER_MARKET, 0, 0, MARKET_COUNT * PRICES_PER_MARKET);
}

#[test]
fn any_duplicate_key_values_should_be_hidden() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    markets[0].set_rating(1.0);
    add_or_update(&market_cache, &markets);

    // when
    markets[0].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_same_prices(&results, &markets[0]);
    assert_overall(&results, PRICES_PER_MARKET, 0, 0, 0);
}

#[test]
fn any_duplicate_values_should_be_no_op_when_removed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    markets[0].set_rating(1.0);
    add_or_update(&market_cache, &markets);
    markets[0].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));

    // when
    markets[1].remove_all_prices();

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_same_prices(&results, &markets[0]);
    assert_overall(&results, PRICES_PER_MARKET, 0, 0, 0);
}

#[test]
fn any_duplicate_values_should_be_unhidden_when_other_is_removed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    markets[0].set_rating(1.0);
    add_or_update(&market_cache, &markets);
    markets[0].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));

    // when
    remove(&market_cache, &markets[0]);

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_same_prices(&results, &markets[1]);
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.messages()[1].updates(), PRICES_PER_MARKET);
    assert_overall(&results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
}

#[test]
fn any_duplicate_values_should_not_refresh_when_hidden() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    markets[0].set_rating(1.0);
    add_or_update(&market_cache, &markets);
    markets[0].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));

    // when
    markets[1].refresh_all_prices(|| get_random_price(&randomizer));

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_eq!(results.summary().overall.refreshes, 0);
    assert_same_prices(&results, &markets[0]);
    assert_overall(&results, PRICES_PER_MARKET, 0, 0, 0);
}

#[test]
fn any_removed_sub_item_is_removed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    add_or_update(&market_cache, &markets);
    set_sectioned_prices(&markets, &randomizer);

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
    assert_overall(&results, MARKET_COUNT * PRICES_PER_MARKET, MARKET_COUNT * REMOVE_COUNT, 0, 0);
}

#[test]
fn any_source_item_removed_removes_all_source_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(MARKET_COUNT);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    add_or_update(&market_cache, &markets);
    set_sectioned_prices(&markets, &randomizer);

    // when
    market_cache.edit(|updater| {
        let keys: Vec<Guid> = updater.keys().into_iter().take(REMOVE_COUNT).collect();
        updater.remove_keys(&mut keys.into_iter());
    });

    // then
    assert_eq!(market_cache_results.data().count(), MARKET_COUNT - REMOVE_COUNT);
    assert_eq!(results.data().count(), (MARKET_COUNT - REMOVE_COUNT) * PRICES_PER_MARKET);
    assert_overall(&results, MARKET_COUNT * PRICES_PER_MARKET, PRICES_PER_MARKET * REMOVE_COUNT, 0, 0);
}

#[test]
fn best_choice_from_duplicates_selected_when_change_set_created() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let market_original = Market::from_number(0);
    let market_better = Market::from_number(1);
    market_better.set_rating(1.0);
    market_original.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_better.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_original.clone()]);
    add_or_update(&market_cache, &[market_better.clone()]);

    // when
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    let results_low = change_set_by_low_rating(&market_cache, false).as_aggregator();

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_overall(&results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
    assert_all_from(&results, market_better.id());
    assert_eq!(results_low.data().count(), PRICES_PER_MARKET);
    assert_overall(&results_low, PRICES_PER_MARKET, 0, 0, 0);
    assert_all_from(&results_low, market_original.id());
}

#[test]
fn changing_source_by_update_removes_previous_and_adds_new_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    let market = Market::from_number(0);
    market.set_prices(0, PRICES_PER_MARKET_ID * 2, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market.clone()]);
    let updated_market = Market::copy_of(&market);
    updated_market.set_prices(PRICES_PER_MARKET_ID, PRICES_PER_MARKET_ID * 3, || get_random_price(&randomizer));

    // when
    add_or_update(&market_cache, &[updated_market.clone()]);

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET * 2);
    assert_overall(&results, PRICES_PER_MARKET * 3, PRICES_PER_MARKET, PRICES_PER_MARKET, 0);
    assert_same_prices(&results, &updated_market);
}

#[test]
fn changing_source_by_update_removes_previous_and_emits_better_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    let market = Market::from_number(0);
    let market_worse = Market::from_number(1);
    set_rating(&market_cache, &market_worse, -1.0);
    market.set_prices(0, PRICES_PER_MARKET_ID * 2, || get_random_price(&randomizer));
    market_worse.set_prices(0, PRICES_PER_MARKET_ID * 2, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market.clone()]);
    add_or_update(&market_cache, &[market_worse.clone()]);

    let updated_market = Market::copy_of(&market);
    updated_market.set_prices(PRICES_PER_MARKET_ID, PRICES_PER_MARKET_ID * 3, || get_random_price(&randomizer));

    // when
    add_or_update(&market_cache, &[updated_market.clone()]);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET * 3);
    assert_overall(&results, PRICES_PER_MARKET * 3, 0, PRICES_PER_MARKET * 2, 0);
    let items = results.data().items();
    for price in items.iter().take(PRICES_PER_MARKET) {
        assert_eq!(price.market_id(), market_worse.id());
    }
    for price in items.iter().skip(PRICES_PER_MARKET) {
        assert_eq!(price.market_id(), updated_market.id());
    }
}

#[test]
fn child_comparer_only_refreshes_visible_values() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    let low_rating_low_price_results = change_set_by_low_rating_then_low_price(&market_cache, false).as_aggregator();
    let low_rating_high_price_results = change_set_by_low_rating_then_high_price(&market_cache, false).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_lowest = Market::from_number(2);

    market_low.set_rating(-1.0);
    market_lowest.set_rating(-1.0);
    market_original.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_low.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_lowest.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);
    add_or_update(&market_cache, &[market_original.clone()]);
    add_or_update(&market_cache, &[market_low.clone()]);
    add_or_update(&market_cache, &[market_lowest.clone()]);

    // when
    market_lowest.refresh_all_prices_to(LOWEST_PRICE - 1.0);

    // then
    assert_eq!(market_cache_results.data().count(), 3);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_overall(&results, PRICES_PER_MARKET, 0, 0, 0);
    assert_all_from(&results, market_original.id());
    assert_eq!(low_rating_low_price_results.data().count(), PRICES_PER_MARKET);
    assert_overall(&low_rating_low_price_results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 2, PRICES_PER_MARKET);
    assert_all_from(&low_rating_low_price_results, market_lowest.id());
    assert_eq!(low_rating_high_price_results.data().count(), PRICES_PER_MARKET);
    assert_overall(&low_rating_high_price_results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
    assert_all_from(&low_rating_high_price_results, market_low.id());
}

#[test]
fn child_comparer_only_updates_visible_values_on_update() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    let low_rating_low_price_results = change_set_by_low_rating_then_low_price(&market_cache, false).as_aggregator();
    let low_rating_high_price_results = change_set_by_low_rating_then_high_price(&market_cache, false).as_aggregator();
    let market_original = Market::from_number(0);
    let market_low = Market::from_number(1);
    let market_lowest = Market::from_number(2);

    market_low.set_rating(-1.0);
    market_lowest.set_rating(-1.0);
    market_original.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_low.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);
    market_lowest.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_original.clone()]);
    add_or_update(&market_cache, &[market_low.clone()]);
    add_or_update(&market_cache, &[market_lowest.clone()]);

    // when
    market_lowest.update_all_prices_to(LOWEST_PRICE - 1.0);

    // then
    assert_eq!(market_cache_results.data().count(), 3);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_overall(&results, PRICES_PER_MARKET, 0, 0, 0);
    assert_all_from(&results, market_original.id());
    assert_eq!(low_rating_low_price_results.data().count(), PRICES_PER_MARKET);
    assert_overall(&low_rating_low_price_results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 2, 0);
    assert_all_from(&low_rating_low_price_results, market_lowest.id());
    assert_eq!(low_rating_high_price_results.data().count(), PRICES_PER_MARKET);
    assert_overall(&low_rating_high_price_results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 3, 0);
    assert_all_from(&low_rating_high_price_results, market_low.id());
}

#[test]
fn child_comparer_updates_to_correct_value_on_update() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results_low = change_set_by_low_rating(&market_cache, false).as_aggregator();
    let results_low_price = change_set_by_rating_then_low_price(&market_cache, false).as_aggregator();
    let results_high_price = change_set_by_rating_then_high_price(&market_cache, false).as_aggregator();
    let market_original = Market::from_number(0);
    let market_highest = Market::from_number(1);
    let market_lowest = Market::from_number(2);
    market_highest.set_rating(1.0);
    market_lowest.set_rating(1.0);
    market_original.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_highest.set_prices_to(0, PRICES_PER_MARKET_ID, HIGHEST_PRICE);
    market_lowest.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_original.clone()]);
    add_or_update(&market_cache, &[market_highest.clone()]);
    add_or_update(&market_cache, &[market_lowest.clone()]);

    // when
    market_lowest.update_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(market_cache_results.data().count(), 3);
    assert_eq!(results_low.data().count(), PRICES_PER_MARKET);
    assert_overall(&results_low, PRICES_PER_MARKET, 0, 0, 0);
    assert_all_from(&results_low, market_original.id());

    assert_eq!(results_low_price.data().count(), PRICES_PER_MARKET);
    assert_overall(&results_low_price, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 3, 0);
    assert_all_from(&results_low_price, market_lowest.id());

    assert_eq!(results_high_price.data().count(), PRICES_PER_MARKET);
    assert_overall(&results_high_price, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
    assert_all_from(&results_high_price, market_highest.id());
}

#[test]
fn equality_comparer_and_child_comparer_refreshes_become_updates() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results_low = change_set_by_low_rating(&market_cache, true).as_aggregator();
    let results_recent = change_set_by_rating_then_recent(&market_cache, true).as_aggregator();
    let results_time_stamp = change_set_by_rating_then_time_stamp(&market_cache, true).as_aggregator();
    let market_low = Market::from_number(0);
    let market = Market::from_number(1);
    market_low.set_rating(-1.0);
    market_low.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_low.clone()]);
    add_or_update(&market_cache, &[market.clone()]);
    market.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);
    // Update again, but only the timestamp will change, so resultsRecent will ignore
    market.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);

    // when
    // resultsRecent won't see the refresh because it ignored the update
    // resultsTimeStamp will see the refreshes because it didn't
    market.refresh_all_prices(|| get_random_price(&randomizer));

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results_low.data().count(), PRICES_PER_MARKET);
    assert_eq!(results_low.messages().len(), 1);
    assert_overall(&results_low, PRICES_PER_MARKET, 0, 0, 0);
    assert_eq!(results_recent.messages().len(), 4);
    assert_overall(&results_recent, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 3, 0);
    assert_eq!(results_time_stamp.messages().len(), 5);
    assert_overall(&results_time_stamp, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 3, PRICES_PER_MARKET);
}

#[test]
fn equality_comparer_and_child_comparer_work_together_for_refreshes() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results_low = change_set_by_low_rating(&market_cache, true).as_aggregator();
    let results_recent = change_set_by_rating_then_recent(&market_cache, true).as_aggregator();
    let results_time_stamp = change_set_by_rating_then_time_stamp(&market_cache, true).as_aggregator();
    let market_low = Market::from_number(0);
    let market = Market::from_number(1);
    market_low.set_rating(-1.0);
    market_low.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_low.clone()]);
    add_or_update(&market_cache, &[market.clone()]);
    market.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);
    // Update again, but only the timestamp will change, so resultsRecent will ignore
    market.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);

    // when
    // resultsRecent won't see the refresh because it ignored the update
    // resultsTimeStamp will see the refreshes because it didn't
    market.refresh_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results_low.data().count(), PRICES_PER_MARKET);
    assert_eq!(results_low.messages().len(), 1);
    assert_overall(&results_low, PRICES_PER_MARKET, 0, 0, 0);
    assert_eq!(results_recent.messages().len(), 4);
    assert_overall(&results_recent, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 2, PRICES_PER_MARKET);
    assert_eq!(results_time_stamp.messages().len(), 5);
    assert_overall(&results_time_stamp, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 3, PRICES_PER_MARKET);
}

#[test]
fn equality_comparer_and_child_comparer_work_together_for_updates() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results_low = change_set_by_low_rating(&market_cache, true).as_aggregator();
    let results_recent = change_set_by_rating_then_recent(&market_cache, true).as_aggregator();
    let results_time_stamp = change_set_by_rating_then_time_stamp(&market_cache, true).as_aggregator();
    let market_low = Market::from_number(0);
    let market = Market::from_number(1);
    market_low.set_rating(-1.0);
    market_low.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_low.clone()]);
    add_or_update(&market_cache, &[market.clone()]);
    market.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);

    // when
    market.update_all_prices_to(LOWEST_PRICE);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results_low.data().count(), PRICES_PER_MARKET);
    assert_eq!(results_low.messages().len(), 1);
    assert_overall(&results_low, PRICES_PER_MARKET, 0, 0, 0);
    assert_eq!(results_recent.messages().len(), 3);
    assert_overall(&results_recent, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 2, 0);
    assert_eq!(results_time_stamp.messages().len(), 4);
    assert_overall(&results_time_stamp, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 3, 0);
}

#[test]
fn equality_comparer_hides_updates_without_changes() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    // having
    let market = Market::from_number(0);
    let results = create_change_set(&market_cache, Some(Market::rating_compare()), None, Some(MarketPrice::equality_comparer()), true).as_aggregator();
    market.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);
    add_or_update(&market_cache, &[market.clone()]);

    // when
    market.set_prices_to(0, PRICES_PER_MARKET_ID, LOWEST_PRICE);

    // then
    assert_eq!(market_cache_results.data().count(), 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_eq!(results.messages().len(), 1);
    assert_overall(&results, PRICES_PER_MARKET, 0, 0, 0);
}

#[test]
fn every_item_visible_when_sequence_completes() {
    let market_cache = new_market_cache();
    let _market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets: Vec<Rc<FixedMarket>> = (0..MARKET_COUNT as i32)
        .map(|n| {
            let randomizer = randomizer.clone();
            FixedMarket::new(move || get_random_price(&randomizer), n * ITEM_ID_STRIDE, n * ITEM_ID_STRIDE + PRICES_PER_MARKET_ID, true)
        })
        .collect();
    add_or_update(&market_cache, &markets);

    // when
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    // `marketCache.Items.ForEach(m => (m as IDisposable)?.Dispose())`: a fixed market is not
    // disposable.
    market_cache.dispose();

    // then
    assert_eq!(results.data().count(), PRICES_PER_MARKET * MARKET_COUNT);
    assert_overall(&results, PRICES_PER_MARKET * MARKET_COUNT, 0, 0, 0);
}

/// `MergedObservableCompletesOnlyWhenSourceAndAllChildrenComplete(bool completeSource, bool
/// completeChildren)`.
fn merged_observable_completes_only_when_source_and_all_children_complete(complete_source: bool, complete_children: bool) {
    let market_cache = new_market_cache();
    let _market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets: Vec<Rc<FixedMarket>> = (0..MARKET_COUNT as i32)
        .map(|n| {
            let randomizer = randomizer.clone();
            FixedMarket::new(move || get_random_price(&randomizer), n * ITEM_ID_STRIDE, n * ITEM_ID_STRIDE + PRICES_PER_MARKET_ID, complete_children)
        })
        .collect();
    add_or_update(&market_cache, &markets);
    let has_source_sequence_completed = Rc::new(Cell::new(false));
    let has_merged_sequence_completed = Rc::new(Cell::new(false));

    let source_completed = has_source_sequence_completed.clone();
    let merged_completed = has_merged_sequence_completed.clone();
    let cleanup = market_cache
        .connect()
        .do_with_completed(|_| {}, move || source_completed.set(true))
        .merge_many_change_sets_with_source_comparer(|market: &Rc<dyn IMarket>| market.latest_prices(), Market::rating_compare())
        .subscribe_next_completed(|_| {}, move || merged_completed.set(true));

    // when
    if complete_source {
        // `marketCache.Items.ForEach(m => (m as IDisposable)?.Dispose())`: a fixed market is not
        // disposable.
        market_cache.dispose();
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
    let market_cache = new_market_cache();
    let _market_cache_results = market_cache.connect().as_aggregator();

    // having
    let markets = new_markets(MARKET_COUNT);
    add_or_update(&market_cache, &markets);
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let expected_error = new_error("Test exception");
    let throw_observable = Observable::throw::<ChangeSet<Rc<dyn IMarket>, Guid>>(expected_error.clone());

    let target = received_error.clone();
    let cleanup = market_cache
        .connect()
        .concat(throw_observable)
        .merge_many_change_sets_with_source_comparer(|market: &Rc<dyn IMarket>| market.latest_prices(), Market::rating_compare())
        .subscribe_next_error(|_| {}, move |error: ObservableError| *target.borrow_mut() = Some(error));

    // when
    for market in &markets {
        market.dispose();
    }
    market_cache.dispose();

    // then
    let received = received_error.borrow().clone();
    assert!(received.is_some_and(|error| Rc::ptr_eq(&error, &expected_error)), "the error of the source");

    cleanup.dispose();
}

// Not ported: NullChecks - argument validation

#[test]
fn only_adds_better_values_on_source_update() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    let results_low = change_set_by_low_rating(&market_cache, false).as_aggregator();
    let market_original = Market::from_number(0);
    let market_better = Market::from_number(1);
    market_better.set_rating(1.0);
    market_original.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_original.clone()]);
    add_or_update(&market_cache, &[market_better.clone()]);

    // when
    market_better.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_overall(&results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
    assert_all_from(&results, market_better.id());
    assert_eq!(results_low.data().count(), PRICES_PER_MARKET);
    assert_overall(&results_low, PRICES_PER_MARKET, 0, 0, 0);
    assert_all_from(&results_low, market_original.id());
}

#[test]
fn only_updates_on_duplicate_if_new_item_is_from_better_parent() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    let results_low = change_set_by_low_rating(&market_cache, false).as_aggregator();
    let market_original = Market::from_number(0);
    let market_better = Market::from_number(1);
    market_better.set_rating(1.0);
    market_original.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_better.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_original.clone()]);

    // when
    add_or_update(&market_cache, &[market_better.clone()]);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_overall(&results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
    assert_all_from(&results, market_better.id());
    assert_eq!(results_low.data().count(), PRICES_PER_MARKET);
    assert_overall(&results_low, PRICES_PER_MARKET, 0, 0, 0);
    assert_all_from(&results_low, market_original.id());
}

#[test]
fn source_refresh_does_nothing_if_disabled() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    markets[0].set_rating(1.0);
    add_or_update(&market_cache, &markets);
    markets[0].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));

    // when
    set_rating(&market_cache, &markets[1], 2.0);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_same_prices(&results, &markets[0]);
    assert_overall(&results, PRICES_PER_MARKET, 0, 0, 0);
}

#[test]
fn source_refresh_generates_updates_as_needed() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let markets = new_markets(2);
    let results = change_set_by_rating(&market_cache, true).as_aggregator();
    markets[0].set_rating(1.0);
    add_or_update(&market_cache, &markets);
    markets[0].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    markets[1].set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));

    // when
    set_rating(&market_cache, &markets[1], 2.0);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_same_prices(&results, &markets[1]);
    assert_overall(&results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
}

#[test]
fn updates_to_correct_value_on_refresh() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let results = change_set_by_rating(&market_cache, false).as_aggregator();
    let results_low = change_set_by_low_rating(&market_cache, false).as_aggregator();
    let results_refresh = change_set_by_rating(&market_cache, true).as_aggregator();
    let results_low_refresh = change_set_by_low_rating(&market_cache, true).as_aggregator();
    let market_original = Market::from_number(0);
    let market_better = Market::from_number(1);
    market_better.set_rating(-1.0);
    market_original.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_better.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_original.clone()]);
    add_or_update(&market_cache, &[market_better.clone()]);

    // when
    set_rating(&market_cache, &market_better, 2.0);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(market_cache_results.summary().overall.refreshes, 1);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_overall(&results, PRICES_PER_MARKET, 0, 0, 0);
    assert_all_from(&results, market_original.id());
    assert_eq!(results_low.data().count(), PRICES_PER_MARKET);
    assert_overall(&results_low, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
    assert_all_from(&results_low, market_better.id());
    assert_overall(&results_refresh, PRICES_PER_MARKET, 0, PRICES_PER_MARKET, 0);
    assert_all_from(&results_refresh, market_better.id());
    assert_eq!(results_low_refresh.data().count(), PRICES_PER_MARKET);
    assert_overall(&results_low_refresh, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 2, 0);
    assert_all_from(&results_low_refresh, market_original.id());
}

#[test]
fn updates_to_correct_value_on_remove() {
    let market_cache = new_market_cache();
    let market_cache_results = market_cache.connect().as_aggregator();

    let randomizer = Randomizer::new(SEED);

    // having
    let market_original = Market::from_number(0);
    let market_better = Market::from_number(1);
    let market_best = Market::from_number(2);
    market_better.set_rating(1.0);
    market_best.set_rating(5.0);
    market_original.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_better.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    market_best.set_prices(0, PRICES_PER_MARKET_ID, || get_random_price(&randomizer));
    add_or_update(&market_cache, &[market_original.clone()]);
    add_or_update(&market_cache, &[market_best.clone()]);
    add_or_update(&market_cache, &[market_better.clone()]);
    let results = change_set_by_rating(&market_cache, false).as_aggregator();

    // when
    remove(&market_cache, &market_best);

    // then
    assert_eq!(market_cache_results.data().count(), 2);
    assert_eq!(results.data().count(), PRICES_PER_MARKET);
    assert_overall(&results, PRICES_PER_MARKET, 0, PRICES_PER_MARKET * 2, 0);
    assert_all_from(&results, market_better.id());
}
