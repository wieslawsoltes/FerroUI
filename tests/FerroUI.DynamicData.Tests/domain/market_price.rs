//! `MarketPrice` (`Domain/MarketPrice.cs`).

use super::{compare_f64, Guid, Randomizer};
use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

/// The clock of the time stamps of prices.
static CLOCK: AtomicU64 = AtomicU64::new(0);

/// The price of an item in a market (`MarketPrice`): the item, the market, the price and the time
/// it was set. Prices are held in `Rc`.
///
/// Deviation (DEVIATIONS.md, DynamicData): `decimal` is `f64`, and the time stamp
/// (`DateTimeOffset.UtcNow`) is the reading of a clock of the process that advances with every
/// price set, so that later prices always have later stamps.
pub struct MarketPrice {
    item_id: i32,
    market_id: Guid,
    price: Cell<f64>,
    time_stamp: Cell<u64>,
}

/// An equality comparer of prices (`IEqualityComparer<MarketPrice>`).
pub type MarketPriceEqualityComparer = fn(&Rc<MarketPrice>, &Rc<MarketPrice>) -> bool;

/// A comparer of prices (`IComparer<MarketPrice>`).
pub type MarketPriceComparer = Rc<dyn Fn(&Rc<MarketPrice>, &Rc<MarketPrice>) -> Ordering>;

impl MarketPrice {
    /// `new MarketPrice(itemId, price, marketId)`.
    pub fn new(item_id: i32, price: f64, market_id: Guid) -> Rc<Self> {
        let market_price = Rc::new(Self { item_id, market_id, price: Cell::new(0.0), time_stamp: Cell::new(0) });
        market_price.set_price(price);
        market_price
    }

    /// The price (`Price`).
    pub fn price(&self) -> f64 {
        self.price.get()
    }

    /// Sets the price and stamps the time (`Price`'s setter).
    pub fn set_price(&self, value: f64) {
        self.price.set(value);
        self.time_stamp.set(CLOCK.fetch_add(1, AtomicOrdering::Relaxed) + 1);
    }

    /// When the price was set (`TimeStamp`).
    pub fn time_stamp(&self) -> u64 {
        self.time_stamp.get()
    }

    /// The market (`MarketId`).
    pub fn market_id(&self) -> Guid {
        self.market_id
    }

    /// The item (`ItemId`).
    pub fn item_id(&self) -> i32 {
        self.item_id
    }

    /// `basePrice` plus up to `offset` (`RandomPrice(Random, decimal, decimal)` and
    /// `RandomPrice(Randomizer, decimal, decimal)`, which take the same generator here).
    pub fn random_price(random: &Randomizer, base_price: f64, offset: f64) -> f64 {
        random.decimal_range(base_price, base_price + offset)
    }

    /// Prices of the same item at the same price are equal (`EqualityComparer`). Upstream compares
    /// the market of `x` with itself (`x.MarketId.Equals(x.MarketId)`), which always holds, so
    /// prices of different markets can be equal; the port leaves that term out.
    pub fn equality_comparer() -> MarketPriceEqualityComparer {
        |x, y| x.item_id == y.item_id && x.price.get() == y.price.get()
    }

    /// [`equality_comparer`](Self::equality_comparer) and the same time stamp
    /// (`EqualityComparerWithTimeStamp`).
    pub fn equality_comparer_with_time_stamp() -> MarketPriceEqualityComparer {
        |x, y| (Self::equality_comparer())(x, y) && x.time_stamp.get() == y.time_stamp.get()
    }

    /// The higher price first (`HighPriceCompare`).
    pub fn high_price_compare() -> MarketPriceComparer {
        Rc::new(|x: &Rc<MarketPrice>, y: &Rc<MarketPrice>| {
            debug_assert_eq!(x.item_id, y.item_id);
            compare_f64(y.price.get(), x.price.get())
        })
    }

    /// The lower price first (`LowPriceCompare`).
    pub fn low_price_compare() -> MarketPriceComparer {
        Rc::new(|x: &Rc<MarketPrice>, y: &Rc<MarketPrice>| {
            debug_assert_eq!(x.item_id, y.item_id);
            compare_f64(x.price.get(), y.price.get())
        })
    }

    /// The later price first (`LatestPriceCompare`).
    pub fn latest_price_compare() -> MarketPriceComparer {
        Rc::new(|x: &Rc<MarketPrice>, y: &Rc<MarketPrice>| {
            debug_assert_eq!(x.item_id, y.item_id);
            y.time_stamp.get().cmp(&x.time_stamp.get())
        })
    }
}

impl PartialEq for MarketPrice {
    /// The same price, time stamp, market and item (`Equals(object)`).
    fn eq(&self, other: &Self) -> bool {
        self.price.get() == other.price.get() && self.time_stamp.get() == other.time_stamp.get() && self.market_id == other.market_id && self.item_id == other.item_id
    }
}

impl Hash for MarketPrice {
    /// `GetHashCode`: `HashCode.Combine(Price, TimeStamp, MarketId, ItemId)`.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.price.get().to_bits().hash(state);
        self.time_stamp.get().hash(state);
        self.market_id.hash(state);
        self.item_id.hash(state);
    }
}

impl fmt::Display for MarketPrice {
    /// `ToString`: `{ItemId:D5} - {Price:c} ({MarketId}) [{TimeStamp}]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:05} - {:.2} ({}) [{}]", self.item_id, self.price.get(), self.market_id, self.time_stamp.get())
    }
}

impl fmt::Debug for MarketPrice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
