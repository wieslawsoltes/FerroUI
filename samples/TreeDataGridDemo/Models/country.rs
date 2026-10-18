//! Port of `Models/Country.cs`.

use ferroui_base::ferro_markup_type;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A country of the table.
pub struct Country {
    name: RefCell<Option<String>>,
    region: RefCell<String>,
    population: Cell<i32>,
    // Square miles.
    area: Cell<i32>,
    // Per square mile.
    population_density: Cell<f64>,
    // Coast / area.
    coast_line: Cell<f64>,
    net_migration: Cell<Option<f64>>,
    // Per 1000 births.
    infant_mortality: Cell<Option<f64>>,
    gdp: Cell<i32>,
    literacy_percent: Cell<Option<f64>>,
    // Per 1000.
    phones: Cell<Option<f64>>,
    birth_rate: Cell<Option<f64>>,
    death_rate: Cell<Option<f64>>,
}

impl PartialEq for Country {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Country {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        region: String,
        population: i32,
        area: i32,
        density: f64,
        coast: f64,
        migration: Option<f64>,
        infant_morality: Option<f64>,
        gdp: i32,
        literacy: Option<f64>,
        _phones: Option<f64>,
        birth: Option<f64>,
        death: Option<f64>,
    ) -> Rc<Self> {
        // The reference's constructor does not assign the phones.
        Rc::new(Self {
            name: RefCell::new(Some(name)),
            region: RefCell::new(region),
            population: Cell::new(population),
            area: Cell::new(area),
            population_density: Cell::new(density),
            coast_line: Cell::new(coast),
            net_migration: Cell::new(migration),
            infant_mortality: Cell::new(infant_morality),
            gdp: Cell::new(gdp),
            literacy_percent: Cell::new(literacy),
            phones: Cell::new(None),
            birth_rate: Cell::new(birth),
            death_rate: Cell::new(death),
        })
    }

    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
    }

    pub fn region(&self) -> String {
        self.region.borrow().clone()
    }

    /// The reference's setter is private; the editing template of the region column writes it.
    pub fn set_region(&self, value: String) {
        *self.region.borrow_mut() = value;
    }

    pub fn population(&self) -> i32 {
        self.population.get()
    }

    pub fn area(&self) -> i32 {
        self.area.get()
    }

    pub fn population_density(&self) -> f64 {
        self.population_density.get()
    }

    pub fn coast_line(&self) -> f64 {
        self.coast_line.get()
    }

    pub fn net_migration(&self) -> Option<f64> {
        self.net_migration.get()
    }

    pub fn infant_mortality(&self) -> Option<f64> {
        self.infant_mortality.get()
    }

    pub fn gdp(&self) -> i32 {
        self.gdp.get()
    }

    pub fn literacy_percent(&self) -> Option<f64> {
        self.literacy_percent.get()
    }

    pub fn phones(&self) -> Option<f64> {
        self.phones.get()
    }

    pub fn birth_rate(&self) -> Option<f64> {
        self.birth_rate.get()
    }

    pub fn death_rate(&self) -> Option<f64> {
        self.death_rate.get()
    }
}

ferro_markup_type!(class Country {
    this: Rc<Country>,
    handles: [Country, Rc<Country>, Option<Rc<Country>>],
    properties: [
        Name: Option<String> {
            get: |this: &Rc<Country>| this.name(),
            set: |this: &Rc<Country>, value: Option<String>| this.set_name(value)
        },
        Region: String {
            get: |this: &Rc<Country>| this.region(),
            set: |this: &Rc<Country>, value: String| this.set_region(value)
        },
        Population: i32 { get: |this: &Rc<Country>| this.population() },
        Area: i32 { get: |this: &Rc<Country>| this.area() },
        PopulationDensity: f64 { get: |this: &Rc<Country>| this.population_density() },
        CoastLine: f64 { get: |this: &Rc<Country>| this.coast_line() },
        NetMigration: Option<f64> { get: |this: &Rc<Country>| this.net_migration() },
        InfantMortality: Option<f64> { get: |this: &Rc<Country>| this.infant_mortality() },
        GDP: i32 { get: |this: &Rc<Country>| this.gdp() },
        LiteracyPercent: Option<f64> { get: |this: &Rc<Country>| this.literacy_percent() },
        Phones: Option<f64> { get: |this: &Rc<Country>| this.phones() },
        BirthRate: Option<f64> { get: |this: &Rc<Country>| this.birth_rate() },
        DeathRate: Option<f64> { get: |this: &Rc<Country>| this.death_rate() },
    ],
});
