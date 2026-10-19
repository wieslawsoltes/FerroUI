//! `RandomPersonGenerator` (`Domain/RandomPersonGenerator.cs`).

use super::{Person, Randomizer};
use std::collections::HashSet;
use std::rc::Rc;

const BOYS: &[&str] = &[
    "Sergio", "Daniel", "Carolina", "David", "Reina", "Saul", "Bernard", "Danny", "Dimas", "Yuri", "Ivan", "Laura", "John", "Bob", "Charles", "Rupert", "William",
    "Englebert", "Aaron", "Quasimodo", "Henry", "Edward", "Zak", "Kai", "Dominguez", "Escobar", "Martin", "Crespo", "Xavier", "Lyons", "Stephens", "Aaron",
];

const GIRLS: &[&str] = &[
    "Ruth", "Katy", "Patricia", "Nikki", "Zoe", "Esmerelda", "Fiona", "Amber", "Kirsty", "Zaira", "Claire", "Isabel", "Esmerelda", "Nicola", "Lucy", "Louise",
    "Elizabeth", "Anne", "Rebecca", "Rhian", "Beatrice",
];

const LAST_NAMES: &[&str] = &[
    "Johnson", "Williams", "Jones", "Brown", "David", "Miller", "Wilson", "Anderson", "Thomas", "Jackson", "White", "Robinson", "Williams", "Jones", "Windor", "McQueen",
    "X", "Black", "Green", "Chicken", "Partrige", "Broad", "Flintoff", "Root",
];

/// Makes people of every combination of a first name and two last names (`RandomPersonGenerator`),
/// in a random order, with random ages.
pub struct RandomPersonGenerator {
    random: Randomizer,
}

impl Default for RandomPersonGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomPersonGenerator {
    /// A generator of its own (`new RandomPersonGenerator()`, whose `Random` has no seed).
    pub fn new() -> Self {
        Self { random: Randomizer::new_unseeded() }
    }

    /// A generator drawn from `random`.
    pub fn with_randomizer(random: Randomizer) -> Self {
        Self { random }
    }

    /// Up to `number` people (`Take(int number = 10000)`): the girls, then the boys, each with
    /// every pair of last names, without repeats (`Union`); shuffled (`OrderBy(x => Guid.NewGuid())`);
    /// the last name is the pair joined by a hyphen, or the one name when both are the same; the
    /// age is in `0..100`.
    pub fn take(&self, number: usize) -> Vec<Rc<Person>> {
        let mut seen = HashSet::new();
        let mut combinations = Vec::new();
        for (names, gender) in [(GIRLS, "F"), (BOYS, "M")] {
            for first in names {
                for second in LAST_NAMES {
                    for third in LAST_NAMES {
                        let combination = (*first, *second, *third, gender);
                        if seen.insert(combination) {
                            combinations.push(combination);
                        }
                    }
                }
            }
        }
        let max_age = 100;
        self.random
            .shuffle(&combinations)
            .into_iter()
            .take(number)
            .map(|(first, second, third, gender)| {
                let last_name = if second == third { second.to_owned() } else { format!("{second}-{third}") };
                let age = self.random.next(0, max_age);
                Person::from_names_with(first, &last_name, age, gender, "")
            })
            .collect()
    }

    /// 10000 people (`Take()`).
    pub fn take_default(&self) -> Vec<Rc<Person>> {
        self.take(10000)
    }
}
