//! `IMarket`, `Market` and `FixedMarket` (`Domain/Market.cs`).

use super::{compare_f64, Guid, MarketPrice, Randomizer};
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, SourceCache};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering as AtomicOrdering};

/// The prices of a market as a sequence of change sets keyed by the item.
pub type PriceChangeSets = Rc<dyn IObservable<ChangeSet<Rc<MarketPrice>, i32>>>;

/// A market (`IMarket`): a name, a rating, an identifier and its latest prices. Markets are held
/// as `Rc<dyn IMarket>` (or `Rc<Market>`); two are equal only when they are the same one.
pub trait IMarket: fmt::Display {
    /// The name (`Name`).
    fn name(&self) -> String;
    /// The rating (`Rating`).
    fn rating(&self) -> f64;
    /// Sets the rating.
    fn set_rating(&self, value: f64);
    /// The identifier (`Id`).
    fn id(&self) -> Guid;
    /// The latest prices (`LatestPrices`).
    fn latest_prices(&self) -> PriceChangeSets;
}

impl PartialEq for dyn IMarket {
    /// The same market (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

impl fmt::Debug for dyn IMarket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// The counter of the identifiers of unique prices (`s_UniquePriceId`).
static UNIQUE_PRICE_ID: AtomicI32 = AtomicI32::new(0);

/// A market whose prices are edited (`Market`): a cache of prices keyed by the item. Markets are
/// held in `Rc`; the methods that return the market upstream (to chain calls) return the `Rc`.
pub struct Market {
    name: String,
    id: Guid,
    rating: Cell<f64>,
    latest_prices: SourceCache<Rc<MarketPrice>, i32>,
}

impl Market {
    fn create(name: String, id: Guid) -> Rc<Self> {
        Rc::new(Self { name, id, rating: Cell::new(0.0), latest_prices: SourceCache::new(|price: &Rc<MarketPrice>| price.item_id()) })
    }

    /// `new Market(string name)`: a new identifier.
    pub fn new(name: impl Into<String>) -> Rc<Self> {
        Self::create(name.into(), Guid::new_guid())
    }

    /// `new Market(int name)`: the name `Market #{name}`.
    pub fn from_number(name: i32) -> Rc<Self> {
        Self::create(format!("Market #{name}"), Guid::new_guid())
    }

    /// `new Market(Market market)`: the name and identifier of `market`, without its prices.
    pub fn copy_of(market: &Market) -> Rc<Self> {
        Self::create(market.name.clone(), market.id)
    }

    /// Higher ratings first (`RatingCompare`).
    pub fn rating_compare() -> Rc<dyn Fn(&Rc<dyn IMarket>, &Rc<dyn IMarket>) -> Ordering> {
        Rc::new(|x: &Rc<dyn IMarket>, y: &Rc<dyn IMarket>| compare_f64(y.rating(), x.rating()))
    }

    /// The cache of the prices (`PricesCache`).
    pub fn prices_cache(&self) -> &SourceCache<Rc<MarketPrice>, i32> {
        &self.latest_prices
    }

    /// A price of this market (`CreatePrice`).
    pub fn create_price(&self, item_id: i32, price: f64) -> Rc<MarketPrice> {
        MarketPrice::new(item_id, price, self.id)
    }

    /// A price of an item of a new identifier, from a counter of the process
    /// (`CreateUniquePrice`).
    pub fn create_unique_price(&self, get_price: impl FnOnce(i32) -> f64) -> Rc<MarketPrice> {
        let id = UNIQUE_PRICE_ID.fetch_add(1, AtomicOrdering::Relaxed) + 1;
        self.create_price(id, get_price(id))
    }

    /// Adds prices of `count` different items drawn from `min_id..max_id`
    /// (`AddRandomIdPrices(Random, int, int, int, Func<decimal>)`).
    pub fn add_random_id_prices(self: &Rc<Self>, random: &Randomizer, count: usize, min_id: i32, max_id: i32, mut rand_prices: impl FnMut() -> f64) -> Rc<Self> {
        let mut seen = HashSet::new();
        let mut prices = Vec::with_capacity(count);
        while prices.len() < count {
            let id = random.next(min_id, max_id);
            if seen.insert(id) {
                prices.push(self.create_price(id, rand_prices()));
            }
        }
        self.latest_prices.edit(|updater| updater.add_or_update_items(&mut prices.into_iter()));
        self.clone()
    }

    /// Sets the prices of the items `section * stride` to `section * stride + count`
    /// (`AddUniquePrices(int section, int count, int stride, Func<decimal>)`).
    pub fn add_unique_prices(self: &Rc<Self>, section: i32, count: i32, stride: i32, get_price: impl FnMut() -> f64) -> Rc<Self> {
        self.set_prices(section * stride, section * stride + count, get_price)
    }

    /// Adds prices of `count` items of new identifiers (`AddUniquePrices(int count, Func<int,
    /// decimal>)`).
    pub fn add_unique_prices_count(self: &Rc<Self>, count: usize, mut get_price: impl FnMut(i32) -> f64) -> Rc<Self> {
        let prices: Vec<_> = (0..count).map(|_| self.create_unique_price(&mut get_price)).collect();
        self.latest_prices.edit(|updater| updater.add_or_update_items(&mut prices.into_iter()));
        self.clone()
    }

    /// Sets the price of the item `id` and refreshes it, if the market has it
    /// (`RefreshPrice(int, decimal)`).
    pub fn refresh_price(self: &Rc<Self>, id: i32, new_price: f64) -> Rc<Self> {
        self.latest_prices.edit(|updater| {
            if let Some(price) = updater.lookup(&id) {
                price.set_price(new_price);
                updater.refresh_item(&price);
            }
        });
        self.clone()
    }

    /// Sets every price to the price of its item and refreshes it
    /// (`RefreshAllPrices(Func<int, decimal>)`).
    pub fn refresh_all_prices_by_id(self: &Rc<Self>, mut get_new_price: impl FnMut(i32) -> f64) -> Rc<Self> {
        self.latest_prices.edit(|updater| {
            for price in updater.items() {
                price.set_price(get_new_price(price.item_id()));
                updater.refresh_item(&price);
            }
        });
        self.clone()
    }

    /// Sets every price and refreshes it (`RefreshAllPrices(Func<decimal>)`).
    pub fn refresh_all_prices(self: &Rc<Self>, mut get_new_price: impl FnMut() -> f64) -> Rc<Self> {
        self.refresh_all_prices_by_id(|_| get_new_price())
    }

    /// Sets every price to `new_price` and refreshes it (`RefreshAllPrices(decimal)`).
    pub fn refresh_all_prices_to(self: &Rc<Self>, new_price: f64) -> Rc<Self> {
        self.refresh_all_prices_by_id(|_| new_price)
    }

    /// Removes every price (`RemoveAllPrices`).
    pub fn remove_all_prices(&self) {
        self.latest_prices.edit(|updater| updater.clear());
    }

    /// Removes the price of an item (`RemovePrice`).
    pub fn remove_price(&self, item_id: i32) {
        self.latest_prices.edit(|updater| updater.remove_key(&item_id));
    }

    /// Replaces every price with a new price of its item (`UpdateAllPrices(Func<int, decimal>)`).
    pub fn update_all_prices_by_id(self: &Rc<Self>, mut get_new_price: impl FnMut(i32) -> f64) -> Rc<Self> {
        self.latest_prices.edit(|updater| {
            let prices: Vec<_> = updater.items().iter().map(|price| self.create_price(price.item_id(), get_new_price(price.item_id()))).collect();
            updater.add_or_update_items(&mut prices.into_iter());
        });
        self.clone()
    }

    /// Replaces every price with a new one (`UpdateAllPrices(Func<decimal>)`).
    pub fn update_all_prices(self: &Rc<Self>, mut get_new_price: impl FnMut() -> f64) -> Rc<Self> {
        self.update_all_prices_by_id(|_| get_new_price())
    }

    /// Replaces every price with `new_price` (`UpdateAllPrices(decimal)`).
    pub fn update_all_prices_to(self: &Rc<Self>, new_price: f64) -> Rc<Self> {
        self.update_all_prices_by_id(|_| new_price)
    }

    /// Sets the prices of the items `min_id..max_id` (`SetPrices(int, int, Func<int, decimal>)`).
    pub fn set_prices_by_id(self: &Rc<Self>, min_id: i32, max_id: i32, mut get_price: impl FnMut(i32) -> f64) -> Rc<Self> {
        let prices: Vec<_> = (min_id..max_id).map(|id| self.create_price(id, get_price(id))).collect();
        self.latest_prices.edit(|updater| updater.add_or_update_items(&mut prices.into_iter()));
        self.clone()
    }

    /// Sets the prices of the items `min_id..max_id` (`SetPrices(int, int, Func<decimal>)`).
    pub fn set_prices(self: &Rc<Self>, min_id: i32, max_id: i32, mut get_price: impl FnMut() -> f64) -> Rc<Self> {
        self.set_prices_by_id(min_id, max_id, |_| get_price())
    }

    /// Sets the prices of the items `min_id..max_id` to `new_price` (`SetPrices(int, int,
    /// decimal)`).
    pub fn set_prices_to(self: &Rc<Self>, min_id: i32, max_id: i32, new_price: f64) -> Rc<Self> {
        self.set_prices_by_id(min_id, max_id, |_| new_price)
    }

    /// Sets the price of the item `id` (`SetPrice(int, Func<decimal>)`).
    pub fn set_price(self: &Rc<Self>, id: i32, get_price: impl FnOnce() -> f64) -> Rc<Self> {
        let price = self.create_price(id, get_price());
        self.latest_prices.edit(|updater| updater.add_or_update(price));
        self.clone()
    }
}

impl IMarket for Market {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn rating(&self) -> f64 {
        self.rating.get()
    }

    fn set_rating(&self, value: f64) {
        self.rating.set(value);
    }

    fn id(&self) -> Guid {
        self.id
    }

    fn latest_prices(&self) -> PriceChangeSets {
        self.latest_prices.connect()
    }
}

impl IDisposable for Market {
    /// Disposes the cache of the prices (`Dispose`).
    fn dispose(&self) {
        self.latest_prices.dispose();
    }
}

impl PartialEq for Market {
    /// The same market (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for Market {
    /// `ToString`: `Market '{Name}' [{Id}] (Rating: {Rating})`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Market '{}' [{}] (Rating: {})", self.name, self.id, self.rating.get())
    }
}

impl fmt::Debug for Market {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// A market whose prices are fixed (`FixedMarket`): each subscription to its prices receives one
/// change set that adds a new price of every item of `min_id..max_id`, and then the end of the
/// sequence when the market is completable.
pub struct FixedMarket {
    id: Guid,
    rating: Cell<f64>,
    latest_prices: PriceChangeSets,
}

impl FixedMarket {
    /// `new FixedMarket(Func<decimal> getPrice, int minId, int maxId, bool completable = true)`.
    ///
    /// The prices are those of `Enumerable.Range(minId, maxId - minId).Select(..)
    /// .AsObservableChangeSet(cp => cp.ItemId, completable)`, written out here: upstream's
    /// enumerable is lazy, so each subscription makes new prices with `get_price`.
    pub fn new(get_price: impl Fn() -> f64 + 'static, min_id: i32, max_id: i32, completable: bool) -> Rc<Self> {
        let id = Guid::new_guid();
        let get_price: Rc<dyn Fn() -> f64> = Rc::new(get_price);
        let latest_prices = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<MarketPrice>, i32>>>| {
            let changes: ChangeSet<Rc<MarketPrice>, i32> = (min_id..max_id)
                .map(|item_id| {
                    let price = MarketPrice::new(item_id, get_price(), id);
                    Change::new(ChangeReason::Add, price.item_id(), price)
                })
                .collect();
            observer.on_next(changes);
            if completable {
                observer.on_completed();
            }
            Disposable::empty()
        });
        Rc::new(Self { id, rating: Cell::new(0.0), latest_prices })
    }
}

impl IMarket for FixedMarket {
    /// The identifier in braces (`Id.ToString("B")`).
    fn name(&self) -> String {
        self.id.to_string_b()
    }

    fn rating(&self) -> f64 {
        self.rating.get()
    }

    fn set_rating(&self, value: f64) {
        self.rating.set(value);
    }

    fn id(&self) -> Guid {
        self.id
    }

    fn latest_prices(&self) -> PriceChangeSets {
        self.latest_prices.clone()
    }
}

impl PartialEq for FixedMarket {
    /// The same market (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for FixedMarket {
    /// `ToString`: `Fixed Market '{Name}' (Rating: {Rating})`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed Market '{}' (Rating: {})", self.name(), self.rating.get())
    }
}

impl fmt::Debug for FixedMarket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
