//! The pseudo-random number generator of the runtime library (`System.Random`) as the view
//! models of the sample use it (not a port of a file of the sample): the subtractive
//! generator of the runtime library, with a seed that differs per instance.

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

    /// `Next(maxValue)`: a non-negative number less than `max_value`.
    ///
    /// # Panics
    /// Panics if `max_value` is negative.
    pub(crate) fn next_max(&mut self, max_value: i32) -> i32 {
        assert!(max_value >= 0, "'maxValue' must be greater than zero.");
        (self.sample() * f64::from(max_value)) as i32
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_seeded_generator_produces_the_sequence_of_the_runtime_library() {
        let mut random = Random::with_seed(42);
        assert_eq!(1_434_747_710, random.internal_sample());
        assert_eq!(302_596_119, random.internal_sample());
        assert_eq!(269_548_474, random.internal_sample());
    }

    #[test]
    fn the_numbers_stay_in_their_range() {
        let mut random = Random::new();
        for _ in 0..1000 {
            assert!((0..10).contains(&random.next_max(10)));
        }
        assert_eq!(0, random.next_max(0));
    }
}
