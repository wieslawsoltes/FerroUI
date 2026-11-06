//! Hash floating geometry consistently with its value equality.
use std::hash::{Hash, Hasher};

pub(super) fn hash_float<H: Hasher>(value: f64, state: &mut H) {
    let bits = if value == 0.0 {
        0
    } else if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    };
    bits.hash(state);
}
