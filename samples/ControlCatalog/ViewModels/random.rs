//! The pseudo-random number generator of the runtime library
//! (`System.Random`) as the view models of the sample use it.
//!
//! The sample depends on the sequence of a seeded generator (the items of
//! the wrap panel page are generated with the seed 42), so the algorithm of
//! the seeded generator of the runtime library is reproduced: the
//! subtractive generator, with its table of 56 entries.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

/// `System.Random`.
pub(crate) struct Random {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl Random {
    /// `new Random()`: a generator with a seed that differs per instance.
    pub(crate) fn new() -> Self {
        // The hasher of the standard library is keyed with random data per instance.
        let seed = RandomState::new().build_hasher().finish();
        Self::with_seed((seed ^ (seed >> 32)) as u32 as i32)
    }

    /// `new Random(Seed)`.
    pub(crate) fn with_seed(seed: i32) -> Self {
        let mut seed_array = [0i32; 56];

        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = 161_803_398 - subtraction;
        seed_array[55] = mj;
        let mut mk = 1;

        let mut ii = 0;
        for _ in 1..55 {
            ii += 21;
            if ii >= 55 {
                ii -= 55;
            }

            seed_array[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += i32::MAX;
            }

            mj = seed_array[ii];
        }

        for _ in 1..5 {
            for i in 1..56 {
                let mut n = i + 30;
                if n >= 55 {
                    n -= 55;
                }

                seed_array[i] = seed_array[i].wrapping_sub(seed_array[1 + n]);
                if seed_array[i] < 0 {
                    seed_array[i] += i32::MAX;
                }
            }
        }

        Self { seed_array, inext: 0, inextp: 21 }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut loc_inext = self.inext + 1;
        if loc_inext >= 56 {
            loc_inext = 1;
        }

        let mut loc_inextp = self.inextp + 1;
        if loc_inextp >= 56 {
            loc_inextp = 1;
        }

        let mut ret_val = self.seed_array[loc_inext].wrapping_sub(self.seed_array[loc_inextp]);

        if ret_val == i32::MAX {
            ret_val -= 1;
        }
        if ret_val < 0 {
            ret_val += i32::MAX;
        }

        self.seed_array[loc_inext] = ret_val;
        self.inext = loc_inext;
        self.inextp = loc_inextp;

        ret_val
    }

    fn sample(&mut self) -> f64 {
        f64::from(self.internal_sample()) * (1.0 / f64::from(i32::MAX))
    }

    /// `Next()`: a non-negative number less than `i32::MAX`.
    #[allow(dead_code)] // Part of the generator; the view models ported so far use the ranged forms.
    pub(crate) fn next(&mut self) -> i32 {
        self.internal_sample()
    }

    /// `Next(maxValue)`: a non-negative number less than `max_value`.
    ///
    /// # Panics
    /// Panics if `max_value` is negative.
    pub(crate) fn next_max(&mut self, max_value: i32) -> i32 {
        assert!(max_value >= 0, "'maxValue' must be greater than zero.");
        (self.sample() * f64::from(max_value)) as i32
    }

    /// `Next(minValue, maxValue)`: a number of the range
    /// `min_value..max_value`.
    ///
    /// # Panics
    /// Panics if `min_value` is greater than `max_value`.
    pub(crate) fn next_range(&mut self, min_value: i32, max_value: i32) -> i32 {
        assert!(min_value <= max_value, "'minValue' cannot be greater than maxValue.");
        let range = i64::from(max_value) - i64::from(min_value);
        if range <= i64::from(i32::MAX) {
            (self.sample() * range as f64) as i32 + min_value
        } else {
            (self.sample_for_large_range() * range as f64) as i64 as i32 + min_value
        }
    }

    fn sample_for_large_range(&mut self) -> f64 {
        let mut result = self.internal_sample();
        let negative = self.internal_sample() % 2 == 0;
        if negative {
            result = -result;
        }
        let mut d = f64::from(result);
        d += f64::from(i32::MAX - 1);
        d /= 2.0 * f64::from(i32::MAX as u32) - 1.0;
        d
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_seeded_generator_produces_the_sequence_of_the_runtime_library() {
        let mut random = Random::with_seed(42);
        assert_eq!(1_434_747_710, random.next());
        assert_eq!(302_596_119, random.next());
        assert_eq!(269_548_474, random.next());
    }

    #[test]
    fn the_numbers_stay_in_their_range() {
        let mut random = Random::new();
        for _ in 0..1000 {
            let value = random.next_range(15, 56);
            assert!((15..56).contains(&value));
            assert!((0..10).contains(&random.next_max(10)));
        }
        assert_eq!(0, random.next_max(0));
    }
}
