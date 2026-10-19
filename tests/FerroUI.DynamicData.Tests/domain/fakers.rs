//! `Fakers` (`Domain/Fakers.cs`) and the parts of Bogus that the tests use: a seeded random
//! generator ([`Randomizer`]), the data of one generated item ([`FakerContext`]: a person, product
//! names, words) and the generator of items ([`Faker<T>`]).
//!
//! Deviation (DEVIATIONS.md, DynamicData): Bogus is not ported. Its parts that the tests use are
//! written here over a deterministic generator (SplitMix64) and lists of names and words of their
//! own; a seed gives the same data on every run, but not the data Bogus gives for that seed. The
//! tests check properties of their results rather than particular values (docs/porting/reactive.md,
//! section 5.7).

use super::{Animal, AnimalFamily, AnimalOwner, Color, Market, Person};
use std::cell::Cell;
use std::rc::Rc;

/// The fewest animals of an owner of [`animal_owner_with_animals`] (`MinAnimals`).
pub const MIN_ANIMALS: usize = 3;

/// The most animals of an owner of [`animal_owner_with_animals`] (`MaxAnimals`: 7 in debug builds,
/// 23 otherwise).
pub const MAX_ANIMALS: usize = if cfg!(debug_assertions) { 7 } else { 23 };

/// The kinds of animals of each family, in the order of [`AnimalFamily`] (`AnimalTypeNames`).
pub const ANIMAL_TYPE_NAMES: [&[&str]; 5] = [
    // Mammal
    &["Dog", "Cat", "Ferret", "Hamster", "Gerbil", "Cavie", "Mouse", "Pot-Bellied Pig"],
    // Reptile
    &["Corn Snake", "Python", "Gecko", "Skink", "Monitor Lizard", "Chameleon", "Tortoise", "Box Turtle", "Iguana"],
    // Fish
    &["Betta", "Goldfish", "Angelfish", "Catfish", "Guppie", "Mollie", "Neon Tetra", "Platie", "Koi"],
    // Amphibian
    &["Frog", "Toad", "Salamander"],
    // Bird
    &["Parakeet", "Cockatoo", "Parrot", "Finch", "Conure", "Lovebird", "Cockatiel"],
];

/// The genders of the people of [`person`] (`PersonGenders`).
pub const PERSON_GENDERS: [&str; 3] = ["F", "M", "O"];

const FIRST_NAMES: &[&str] = &[
    "Aaron", "Abigail", "Adam", "Adrian", "Alice", "Amber", "Amelia", "Andrew", "Anna", "Arthur", "Audrey", "Barbara", "Benjamin", "Bernard", "Bianca", "Bruce", "Caleb",
    "Camille", "Carl", "Caroline", "Charles", "Chloe", "Claire", "Colin", "Daisy", "Daniel", "David", "Diana", "Dominic", "Dorothy", "Edgar", "Edith", "Edward", "Eleanor",
    "Elijah", "Elizabeth", "Emily", "Emma", "Ethan", "Evelyn", "Felix", "Fiona", "Florence", "Frank", "Gabriel", "Grace", "Gregory", "Hannah", "Harold", "Harriet", "Henry",
    "Hugo", "Ian", "Irene", "Isaac", "Isabel", "Jack", "Jacob", "James", "Jane", "Jasmine", "Joan", "John", "Joseph", "Julia", "Karen", "Katherine", "Kevin", "Laura",
    "Leo", "Lily", "Louis", "Lucy", "Luke", "Margaret", "Maria", "Martin", "Matthew", "Megan", "Michael", "Mia", "Nathan", "Nicole", "Noah", "Olivia", "Oscar", "Patricia",
    "Paul", "Peter", "Philip", "Rachel", "Rebecca", "Richard", "Robert", "Rose", "Ruth", "Samuel", "Sarah", "Simon", "Sophia", "Stephen", "Susan", "Thomas", "Victoria",
    "Vincent", "Walter", "William", "Zoe",
];

const LAST_NAMES: &[&str] = &[
    "Adams", "Allen", "Anderson", "Bailey", "Baker", "Barnes", "Bell", "Bennett", "Brooks", "Brown", "Butler", "Campbell", "Carter", "Clark", "Collins", "Cook", "Cooper",
    "Cox", "Davies", "Davis", "Edwards", "Evans", "Fisher", "Foster", "Garcia", "Gray", "Green", "Hall", "Harris", "Hill", "Howard", "Hughes", "Jackson", "James",
    "Jenkins", "Johnson", "Jones", "Kelly", "King", "Lee", "Lewis", "Long", "Martin", "Miller", "Mitchell", "Moore", "Morgan", "Morris", "Murphy", "Nelson", "Parker",
    "Perry", "Phillips", "Powell", "Price", "Reed", "Richardson", "Roberts", "Robinson", "Rogers", "Ross", "Russell", "Sanders", "Scott", "Smith", "Stewart", "Taylor",
    "Thomas", "Thompson", "Turner", "Walker", "Ward", "Watson", "White", "Williams", "Wilson", "Wood", "Wright", "Young",
];

const PRODUCT_ADJECTIVES: &[&str] = &[
    "Small", "Ergonomic", "Rustic", "Intelligent", "Gorgeous", "Incredible", "Fantastic", "Practical", "Sleek", "Awesome", "Generic", "Handcrafted", "Handmade",
    "Licensed", "Refined", "Unbranded", "Tasty", "Shiny", "Polished", "Sturdy", "Elegant", "Modern", "Vintage", "Bright", "Quiet",
];

const PRODUCT_MATERIALS: &[&str] = &[
    "Steel", "Wooden", "Concrete", "Plastic", "Cotton", "Granite", "Rubber", "Metal", "Soft", "Fresh", "Frozen", "Ceramic", "Leather", "Glass", "Bamboo",
];

const PRODUCTS: &[&str] = &[
    "Chair", "Car", "Computer", "Keyboard", "Mouse", "Bike", "Ball", "Gloves", "Pants", "Shirt", "Table", "Shoes", "Hat", "Towels", "Soap", "Tuna", "Chicken", "Fish",
    "Cheese", "Bacon", "Pizza", "Salad", "Sausages", "Chips", "Lamp", "Clock", "Kettle", "Bottle",
];

const HACKER_ADJECTIVES: &[&str] = &[
    "auxiliary", "primary", "back-end", "digital", "open-source", "virtual", "cross-platform", "redundant", "online", "haptic", "multi-byte", "bluetooth", "wireless",
    "1080p", "neural", "optical", "solid state", "mobile",
];

const HACKER_NOUNS: &[&str] = &[
    "driver", "protocol", "bandwidth", "panel", "microchip", "program", "port", "card", "array", "interface", "system", "sensor", "firewall", "hard drive", "pixel",
    "alarm", "feed", "monitor", "application", "transmitter", "bus", "circuit", "capacitor", "matrix",
];

const HACKER_VERBS: &[&str] = &[
    "back up", "bypass", "hack", "override", "compress", "copy", "navigate", "index", "connect", "generate", "quantify", "calculate", "synthesize", "input", "transmit",
    "program", "reboot", "parse",
];

const WORDS: &[&str] = &[
    "alpha", "amber", "anchor", "apple", "arrow", "autumn", "badge", "bamboo", "banner", "basket", "beacon", "berry", "blossom", "breeze", "bridge", "bronze", "cable",
    "candle", "canyon", "carbon", "castle", "cedar", "cherry", "circle", "cloud", "clover", "comet", "copper", "coral", "cotton", "crystal", "delta", "desert", "dragon",
    "echo", "ember", "falcon", "feather", "forest", "fossil", "galaxy", "garden", "glacier", "harbor", "hazel", "horizon", "island", "ivory", "jasper", "kernel", "lagoon",
    "lantern", "lemon", "maple", "marble", "meadow", "mirror", "nectar", "nova", "oasis", "orbit", "pebble", "pepper", "planet", "prism", "quartz", "raven", "river",
    "saddle", "shadow", "silver", "summit", "thunder", "timber", "tunnel", "velvet", "willow", "zephyr",
];

const ALPHA_NUMERIC: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// The seed of the generator of the thread, which the fakers without a seed of their own share
/// (Bogus's `Randomizer.Seed`).
const SHARED_SEED: i32 = 0x0D1A_5EED;

ferroui_system::ferro_thread_local! {
    static SHARED: Randomizer = Randomizer::new(SHARED_SEED);
}

/// An enumeration whose values a [`Randomizer`] can pick (`Randomizer.Enum<T>()`).
pub trait FakeEnum: Copy + PartialEq + 'static {
    /// Every value, in the order of declaration (`Enum.GetValues<T>()`).
    fn values() -> &'static [Self];
}

/// A deterministic random generator (Bogus's `Randomizer`, and `System.Random` where the domain
/// takes one).
///
/// A clone shares the state of the generator, as the reference to a `Randomizer` does upstream, so
/// that closures can draw from the generator of their test.
#[derive(Clone)]
pub struct Randomizer {
    state: Rc<Cell<u64>>,
}

impl std::fmt::Debug for Randomizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Randomizer").field("state", &self.state.get()).finish()
    }
}

impl Randomizer {
    /// A generator of `seed` (`new Randomizer(int seed)`, `new Random(int seed)`).
    pub fn new(seed: i32) -> Self {
        Self { state: Rc::new(Cell::new((seed as i64 as u64) ^ 0x9E37_79B9_7F4A_7C15)) }
    }

    /// A generator without a seed of its own (`new Randomizer()`, `new Random()`): seeded from the
    /// generator of the thread, which starts from a fixed seed, so that a run can be repeated.
    pub fn new_unseeded() -> Self {
        Self::new(Self::shared().int())
    }

    /// The generator of the thread, which the fakers without a seed share (`Randomizer.Seed`).
    pub fn shared() -> Self {
        SHARED.with(Clone::clone)
    }

    /// The next 64 random bits (SplitMix64).
    pub fn next_u64(&self) -> u64 {
        let state = self.state.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.state.set(state);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..bound`, without the bias of a modulo; 0 for a bound of 0.
    fn below(&self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % bound;
            }
        }
    }

    /// A number in `min..=max` of 64 bits.
    fn in_range_i64(&self, min: i64, max: i64) -> i64 {
        assert!(min <= max, "The min value must be less than or equal to the max value");
        let span = (max as i128 - min as i128 + 1) as u128;
        if span > u128::from(u64::MAX) {
            return self.next_u64() as i64;
        }
        (min as i128 + self.below(span as u64) as i128) as i64
    }

    /// Any number of 32 bits (`Int()`).
    pub fn int(&self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// A number in `min..=max` (`Int(int min, int max)`).
    pub fn int_range(&self, min: i32, max: i32) -> i32 {
        self.in_range_i64(i64::from(min), i64::from(max)) as i32
    }

    /// A number in `0..=max` (`Number(int max)`).
    pub fn number(&self, max: i32) -> i32 {
        self.int_range(0, max)
    }

    /// A number in `min..=max` (`Number(int min, int max)`).
    pub fn number_range(&self, min: i32, max: i32) -> i32 {
        self.int_range(min, max)
    }

    /// A number in `min..=max` of 64 bits (`Long(long min, long max)`).
    pub fn long_range(&self, min: i64, max: i64) -> i64 {
        self.in_range_i64(min, max)
    }

    /// A coin flip (`Bool()`).
    pub fn bool(&self) -> bool {
        self.next_u64() >> 63 == 1
    }

    /// True with the probability `weight` (`Bool(float weight)`).
    pub fn bool_weighted(&self, weight: f32) -> bool {
        self.double() < f64::from(weight)
    }

    /// A number in `[0, 1)` (`Double()`, `Random.NextDouble()`).
    pub fn double(&self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// A number in `[min, max)` (`Double(double min, double max)`).
    pub fn double_range(&self, min: f64, max: f64) -> f64 {
        min + self.double() * (max - min)
    }

    /// A number in `[min, max)` (`Decimal(decimal min, decimal max)`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): `decimal` is `f64` in the domain of the tests.
    pub fn decimal_range(&self, min: f64, max: f64) -> f64 {
        self.double_range(min, max)
    }

    /// A number in `min..max`, `min` when the range is empty (`Random.Next(int min, int max)`).
    pub fn next(&self, min: i32, max: i32) -> i32 {
        if max <= min {
            return min;
        }
        (i64::from(min) + self.below((i64::from(max) - i64::from(min)) as u64) as i64) as i32
    }

    /// A number in `0..max` (`Random.Next(int max)`).
    pub fn next_max(&self, max: i32) -> i32 {
        self.next(0, max)
    }

    /// A number in `[0, 1)` (`Random.NextDouble()`).
    pub fn next_double(&self) -> f64 {
        self.double()
    }

    /// One of the items (`ListItem`, `ArrayElement`, `PickRandom`).
    ///
    /// # Panics
    /// When there are no items.
    pub fn list_item<T: Clone>(&self, items: &[T]) -> T {
        assert!(!items.is_empty(), "There are no items to pick from");
        items[self.below(items.len() as u64) as usize].clone()
    }

    /// One of the items (`ArrayElement`).
    pub fn array_element<T: Clone>(&self, items: &[T]) -> T {
        self.list_item(items)
    }

    /// `count` different items, in a random order (`ListItems(IList<T>, int count)`).
    pub fn list_items<T: Clone>(&self, items: &[T], count: usize) -> Vec<T> {
        let mut shuffled = self.shuffle(items);
        shuffled.truncate(count);
        shuffled
    }

    /// The items in a random order (`Shuffle`).
    pub fn shuffle<T: Clone>(&self, items: &[T]) -> Vec<T> {
        let mut shuffled = items.to_vec();
        for index in (1..shuffled.len()).rev() {
            let other = self.below(index as u64 + 1) as usize;
            shuffled.swap(index, other);
        }
        shuffled
    }

    /// One of the items, by its weight (`WeightedRandom(T[] items, float[] weights)`).
    ///
    /// # Panics
    /// When the counts of items and weights differ, or there are no items.
    pub fn weighted_random<T: Clone>(&self, items: &[T], weights: &[f32]) -> T {
        assert_eq!(items.len(), weights.len(), "The items and the weights must have the same count");
        assert!(!items.is_empty(), "There are no items to pick from");
        let total: f64 = weights.iter().map(|weight| f64::from(*weight)).sum();
        let mut target = self.double() * total;
        for (item, weight) in items.iter().zip(weights) {
            target -= f64::from(*weight);
            if target < 0.0 {
                return item.clone();
            }
        }
        items[items.len() - 1].clone()
    }

    /// A word (`Word()`).
    pub fn word(&self) -> String {
        self.list_item(WORDS).to_owned()
    }

    /// `count` words separated by spaces (`Words(int count)`).
    pub fn words(&self, count: usize) -> String {
        (0..count).map(|_| self.word()).collect::<Vec<_>>().join(" ")
    }

    /// `length` lowercase letters and digits (`AlphaNumeric(int length)`).
    pub fn alpha_numeric(&self, length: usize) -> String {
        (0..length).map(|_| char::from(self.list_item(ALPHA_NUMERIC))).collect()
    }

    /// A value of an enumeration (`Enum<T>()`).
    pub fn enum_value<E: FakeEnum>(&self) -> E {
        self.list_item(E::values())
    }

    /// `count` different values of an enumeration, without those of `exclude`
    /// (`EnumValues<T>(int count, params T[] exclude)`).
    pub fn enum_values<E: FakeEnum>(&self, count: usize, exclude: &[E]) -> Vec<E> {
        let candidates: Vec<E> = E::values().iter().copied().filter(|value| !exclude.contains(value)).collect();
        self.list_items(&candidates, count)
    }

    /// A color other than `NotSpecified` (`Fakers.RandomColor(this Randomizer)`).
    pub fn random_color(&self) -> Color {
        self.enum_values(1, &[Color::NotSpecified])[0]
    }

    /// A color other than `NotSpecified` and `current` (`Fakers.RandomColor(this Randomizer,
    /// Color)`).
    pub fn random_color_except(&self, current: Color) -> Color {
        self.enum_values(1, &[Color::NotSpecified, current])[0]
    }
}

/// The person of one generated item (`Faker.Person`): one per item, as Bogus makes one per
/// generation.
#[derive(Clone, Debug)]
pub struct FakePerson {
    first_name: String,
    last_name: String,
}

impl FakePerson {
    fn new(random: &Randomizer) -> Self {
        Self { first_name: random.list_item(FIRST_NAMES).to_owned(), last_name: random.list_item(LAST_NAMES).to_owned() }
    }

    /// The first name (`FirstName`).
    pub fn first_name(&self) -> String {
        self.first_name.clone()
    }

    /// The last name (`LastName`).
    pub fn last_name(&self) -> String {
        self.last_name.clone()
    }

    /// The first and last names (`FullName`).
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

/// Names of products (`Faker.Commerce`).
pub struct Commerce<'a> {
    random: &'a Randomizer,
}

impl Commerce<'_> {
    /// An adjective of a product (`ProductAdjective()`).
    pub fn product_adjective(&self) -> String {
        self.random.list_item(PRODUCT_ADJECTIVES).to_owned()
    }

    /// A material (`ProductMaterial()`).
    pub fn product_material(&self) -> String {
        self.random.list_item(PRODUCT_MATERIALS).to_owned()
    }

    /// A product (`Product()`).
    pub fn product(&self) -> String {
        self.random.list_item(PRODUCTS).to_owned()
    }

    /// An adjective, a material and a product (`ProductName()`).
    pub fn product_name(&self) -> String {
        format!("{} {} {}", self.product_adjective(), self.product_material(), self.product())
    }
}

/// The words of computing (`Faker.Hacker`).
pub struct Hacker<'a> {
    random: &'a Randomizer,
}

impl Hacker<'_> {
    /// An adjective (`Adjective()`).
    pub fn adjective(&self) -> String {
        self.random.list_item(HACKER_ADJECTIVES).to_owned()
    }

    /// A noun (`Noun()`).
    pub fn noun(&self) -> String {
        self.random.list_item(HACKER_NOUNS).to_owned()
    }

    /// A verb (`Verb()`).
    pub fn verb(&self) -> String {
        self.random.list_item(HACKER_VERBS).to_owned()
    }
}

/// The data of one generated item (Bogus's `Faker`, the argument of `CustomInstantiator`,
/// `RuleFor` and `FinishWith`).
pub struct FakerContext {
    random: Randomizer,
    person: FakePerson,
}

impl FakerContext {
    /// The data of an item drawn from `random`.
    pub fn new(random: Randomizer) -> Self {
        let person = FakePerson::new(&random);
        Self { random, person }
    }

    /// The generator (`Random`).
    pub fn random(&self) -> &Randomizer {
        &self.random
    }

    /// The person of this item (`Person`).
    pub fn person(&self) -> &FakePerson {
        &self.person
    }

    /// Names of products (`Commerce`).
    pub fn commerce(&self) -> Commerce<'_> {
        Commerce { random: &self.random }
    }

    /// The words of computing (`Hacker`).
    pub fn hacker(&self) -> Hacker<'_> {
        Hacker { random: &self.random }
    }

    /// One of the items (`PickRandom<T>(params T[])`).
    pub fn pick_random<T: Clone>(&self, items: &[T]) -> T {
        self.random.list_item(items)
    }

    /// A value of an enumeration (`PickRandom<T>()` of an enum).
    pub fn pick_random_enum<E: FakeEnum>(&self) -> E {
        self.random.enum_value()
    }
}

type Instantiator<T> = Rc<dyn Fn(&FakerContext) -> T>;
type Rule<T> = Rc<dyn Fn(&T, &FakerContext)>;
type FinishAction<T> = Rc<dyn Fn(&FakerContext, &T)>;

struct FakerConfig<T> {
    instantiator: Instantiator<T>,
    rules: Vec<Rule<T>>,
    finish: Option<FinishAction<T>>,
    seed: Option<i32>,
}

impl<T> Clone for FakerConfig<T> {
    fn clone(&self) -> Self {
        Self { instantiator: self.instantiator.clone(), rules: self.rules.clone(), finish: self.finish.clone(), seed: self.seed }
    }
}

/// A generator of items (Bogus's `Faker<T>`): an instantiator, rules that set properties, a final
/// action, and a generator of random data, the faker's own when it has a seed, else the one of the
/// thread.
///
/// The builder methods (`use_seed`, `rule_for`, `finish_with`) return a new faker. `Clone` gives a
/// handle to the same faker, which keeps drawing from the same generator; [`duplicate`](Self::duplicate)
/// is Bogus's `Clone()`.
pub struct Faker<T> {
    config: Rc<FakerConfig<T>>,
    randomizer: Randomizer,
}

impl<T> Clone for Faker<T> {
    fn clone(&self) -> Self {
        Self { config: self.config.clone(), randomizer: self.randomizer.clone() }
    }
}

impl<T: 'static> Faker<T> {
    /// A faker that makes its items with `instantiator` (`new Faker<T>().CustomInstantiator(..)`).
    pub fn new(instantiator: impl Fn(&FakerContext) -> T + 'static) -> Self {
        Self { config: Rc::new(FakerConfig { instantiator: Rc::new(instantiator), rules: Vec::new(), finish: None, seed: None }), randomizer: Randomizer::shared() }
    }

    fn with_config(&self, change: impl FnOnce(&mut FakerConfig<T>)) -> Self {
        let mut config = (*self.config).clone();
        change(&mut config);
        Self { config: Rc::new(config), randomizer: self.randomizer.clone() }
    }

    /// The faker with a generator of its own, of `seed` (`UseSeed(int)`).
    pub fn use_seed(self, seed: i32) -> Self {
        let mut faker = self.with_config(|config| config.seed = Some(seed));
        faker.randomizer = Randomizer::new(seed);
        faker
    }

    /// The faker with a generator seeded from `randomizer` (`FakerExtensions.WithSeed`:
    /// `UseSeed(randomizer.Int())`).
    pub fn with_seed(self, randomizer: &Randomizer) -> Self {
        self.use_seed(randomizer.int())
    }

    /// The faker with a rule that sets a property of each item after it is made
    /// (`RuleFor(x => x.Property, f => ...)`): the rule assigns through the item's setter.
    pub fn rule_for(self, rule: impl Fn(&T, &FakerContext) + 'static) -> Self {
        self.with_config(|config| config.rules.push(Rc::new(rule)))
    }

    /// The faker with the action that ends the making of each item, in place of the previous one
    /// (`FinishWith`).
    pub fn finish_with(self, action: impl Fn(&FakerContext, &T) + 'static) -> Self {
        self.with_config(|config| config.finish = Some(Rc::new(action)))
    }

    /// A faker with the same rules and seed (`Clone()`): with a seed, its generator starts over
    /// from it; without one, it shares the generator of the thread.
    pub fn duplicate(&self) -> Self {
        let randomizer = match self.config.seed {
            Some(seed) => Randomizer::new(seed),
            None => Randomizer::shared(),
        };
        Self { config: self.config.clone(), randomizer }
    }

    /// The generator of the faker (`Random` of its `FakerHub`).
    pub fn randomizer(&self) -> &Randomizer {
        &self.randomizer
    }

    /// One item (`Generate()`).
    pub fn generate_one(&self) -> T {
        let context = FakerContext::new(self.randomizer.clone());
        let item = (self.config.instantiator)(&context);
        for rule in &self.config.rules {
            rule(&item, &context);
        }
        if let Some(finish) = &self.config.finish {
            finish(&context, &item);
        }
        item
    }

    /// `count` items (`Generate(int count)`).
    pub fn generate(&self, count: usize) -> Vec<T> {
        (0..count).map(|_| self.generate_one()).collect()
    }

    /// Between `min` and `max` items, inclusive (`GenerateBetween(int min, int max)`).
    pub fn generate_between(&self, min: usize, max: usize) -> Vec<T> {
        let count = self.randomizer.number_range(min as i32, max as i32);
        self.generate(count as usize)
    }
}

impl Faker<Rc<AnimalOwner>> {
    /// The faker whose owners start with between `min_count` and `max_count` animals of
    /// `animal_faker` (`WithInitialAnimals(Faker<Animal>, int minCount, int maxCount)`).
    pub fn with_initial_animals_between(self, animal_faker: &Faker<Rc<Animal>>, min_count: usize, max_count: usize) -> Self {
        let animal_faker = animal_faker.clone();
        self.finish_with(move |_, owner: &Rc<AnimalOwner>| {
            let animals = animal_faker.generate_between(min_count, max_count);
            owner.animals().edit(|list| list.add_range(animals));
        })
    }

    /// The faker whose owners start with up to `max_count` animals
    /// (`WithInitialAnimals(Faker<Animal>, int maxCount)`).
    pub fn with_initial_animals_max(self, animal_faker: &Faker<Rc<Animal>>, max_count: usize) -> Self {
        self.with_initial_animals_between(animal_faker, 0, max_count)
    }

    /// The faker whose owners start with between [`MIN_ANIMALS`] and [`MAX_ANIMALS`] animals
    /// (`WithInitialAnimals(Faker<Animal>)`).
    pub fn with_initial_animals(self, animal_faker: &Faker<Rc<Animal>>) -> Self {
        self.with_initial_animals_between(animal_faker, MIN_ANIMALS, MAX_ANIMALS)
    }
}

impl AnimalOwner {
    /// Adds between `min_count` and `max_count` animals of `animal_faker`, and returns the owner
    /// (`Fakers.AddAnimals(this AnimalOwner, Faker<Animal>, int, int)`).
    pub fn add_animals_between(self: &Rc<Self>, animal_faker: &Faker<Rc<Animal>>, min_count: usize, max_count: usize) -> Rc<Self> {
        let animals = animal_faker.generate_between(min_count, max_count);
        self.animals().edit(|list| list.add_range(animals));
        self.clone()
    }

    /// Adds `count` animals of `animal_faker`, and returns the owner
    /// (`Fakers.AddAnimals(this AnimalOwner, Faker<Animal>, int)`).
    pub fn add_animals(self: &Rc<Self>, animal_faker: &Faker<Rc<Animal>>, count: usize) -> Rc<Self> {
        let animals = animal_faker.generate(count);
        self.animals().edit(|list| list.add_range(animals));
        self.clone()
    }
}

/// Animals: a family, a kind of that family and a name (`Fakers.Animal`).
pub fn animal() -> Faker<Rc<Animal>> {
    Faker::new(|faker| {
        let family = faker.pick_random_enum::<AnimalFamily>();
        let animal_type = faker.pick_random(ANIMAL_TYPE_NAMES[family as usize]).to_owned();
        let name = format!("{} {}", faker.commerce().product_adjective(), faker.person().first_name());
        Animal::new(name, animal_type, family)
    })
}

/// Owners without animals (`Fakers.AnimalOwner`).
pub fn animal_owner() -> Faker<Rc<AnimalOwner>> {
    Faker::new(|faker| AnimalOwner::new(faker.person().full_name()))
}

/// Owners with between [`MIN_ANIMALS`] and [`MAX_ANIMALS`] animals (`Fakers.AnimalOwnerWithAnimals`).
pub fn animal_owner_with_animals() -> Faker<Rc<AnimalOwner>> {
    animal_owner().with_initial_animals(&animal())
}

/// Markets named after a product (`Fakers.Market`).
pub fn market() -> Faker<Rc<Market>> {
    Faker::new(|faker| Market::new(format!("{} Id#{}", faker.commerce().product_name(), faker.random().alpha_numeric(5))))
}

/// People with an age of 1 to 100, a gender, a favorite color and a kind of pet (`Fakers.Person`).
pub fn person() -> Faker<Rc<Person>> {
    Faker::new(|faker| {
        let person = Person::with_all(faker.person().full_name(), faker.random().int_range(1, 100), faker.pick_random(&PERSON_GENDERS), faker.person().first_name());
        person.set_favorite_color(faker.random().random_color());
        person.set_pet_type(faker.pick_random_enum::<AnimalFamily>());
        person
    })
}
