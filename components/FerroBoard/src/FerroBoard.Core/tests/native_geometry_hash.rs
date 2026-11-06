//! Rust hash-key behavior for geometry values, independent of CLR hash numbers.
use ferroboard_core::layouts::{KeyRect, KeyboardGeometryOptions, KeyboardGeometryOptionsValues};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

fn hash(value: &impl Hash) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

#[test]
fn equal_geometry_handles_signed_zero_and_nan_in_rust_maps() {
    let a = KeyRect::new(0.0, f64::NAN, 32.25, 14.5);
    let b = KeyRect::new(-0.0, f64::from_bits(0xfff8_0000_0000_0007), 32.25, 14.5);
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));
    let mut values = HashMap::new();
    values.insert(a, "same rectangle");
    assert_eq!(values.get(&b), Some(&"same rectangle"));

    let a = KeyboardGeometryOptions::from_values(KeyboardGeometryOptionsValues {
        key_spacing_horizontal: 0.0,
        key_spacing_vertical: f64::NAN,
        ..Default::default()
    });
    let b = KeyboardGeometryOptions::from_values(KeyboardGeometryOptionsValues {
        key_spacing_horizontal: -0.0,
        key_spacing_vertical: f64::from_bits(0x7ff0_0000_0000_0042),
        ..Default::default()
    });
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));
}
