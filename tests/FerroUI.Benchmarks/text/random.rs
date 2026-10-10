//! The generator of the inputs of the text benchmarks.
//!
//! Not a port: upstream draws its inputs from the seeded generator of its
//! platform, whose sequence cannot be reproduced here. This one has the same
//! members (a value, a value below a bound, a value in a range) over a
//! SplitMix64 state, so the same seed gives the same input on every run and
//! the distributions and lengths of the inputs are those of upstream.

pub(crate) struct Random {
    state: u64,
}

impl Random {
    pub(crate) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// A value that is not negative.
    pub(crate) fn next(&mut self) -> i32 {
        (self.next_u64() >> 33) as i32
    }

    /// A value from zero up to `max`, which is not included.
    pub(crate) fn next_max(&mut self, max: i32) -> i32 {
        self.next_range(0, max)
    }

    /// A value from `min` up to `max`, which is not included.
    pub(crate) fn next_range(&mut self, min: i32, max: i32) -> i32 {
        let span = i64::from(max) - i64::from(min);
        if span <= 0 {
            return min;
        }
        let offset = (self.next_u64() >> 11) % span as u64;
        (i64::from(min) + offset as i64) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::Random;

    #[test]
    fn the_same_seed_gives_the_same_values_inside_their_bounds() {
        let mut first = Random::new(42);
        let mut second = Random::new(42);
        for _ in 0..1000 {
            let value = first.next_range(0x20, 0x7F);
            assert_eq!(value, second.next_range(0x20, 0x7F));
            assert!((0x20..0x7F).contains(&value));
            assert!(first.next() >= 0);
            assert!(second.next() >= 0);
            assert!((0..4).contains(&first.next_max(4)));
            assert!((0..4).contains(&second.next_max(4)));
        }
    }
}
