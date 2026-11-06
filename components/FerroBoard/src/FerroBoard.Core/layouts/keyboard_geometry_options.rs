//! Shared keyboard geometry settings. Clone copies values into an independent owner.
use std::{
    hash::{Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Copy, Debug)]
pub struct KeyboardGeometryOptionsValues {
    pub key_spacing_horizontal: f64,
    pub key_spacing_vertical: f64,
    pub split_gap: f64,
    pub units_per_row: f64,
}
impl Default for KeyboardGeometryOptionsValues {
    fn default() -> Self {
        Self {
            key_spacing_horizontal: 5.0,
            key_spacing_vertical: 10.0,
            split_gap: 0.0,
            units_per_row: 10.0,
        }
    }
}
#[derive(Debug)]
pub struct KeyboardGeometryOptions {
    key_spacing_horizontal: AtomicU64,
    key_spacing_vertical: AtomicU64,
    split_gap: AtomicU64,
    units_per_row: AtomicU64,
}
macro_rules! fields {
    ($($get:ident,$init:ident;)*)=>{$(
        pub fn $get(&self)->f64 {f64::from_bits(self.$get.load(Ordering::Relaxed))}
        pub fn $init(&self,value:f64){self.$get.store(value.to_bits(),Ordering::Relaxed);}
    )*};
}
impl KeyboardGeometryOptions {
    pub fn from_values(values: KeyboardGeometryOptionsValues) -> Self {
        Self {
            key_spacing_horizontal: AtomicU64::new(values.key_spacing_horizontal.to_bits()),
            key_spacing_vertical: AtomicU64::new(values.key_spacing_vertical.to_bits()),
            split_gap: AtomicU64::new(values.split_gap.to_bits()),
            units_per_row: AtomicU64::new(values.units_per_row.to_bits()),
        }
    }
    fields! {key_spacing_horizontal,set_key_spacing_horizontal;key_spacing_vertical,set_key_spacing_vertical;split_gap,set_split_gap;units_per_row,set_units_per_row;}
    pub fn values(&self) -> KeyboardGeometryOptionsValues {
        KeyboardGeometryOptionsValues {
            key_spacing_horizontal: self.key_spacing_horizontal(),
            key_spacing_vertical: self.key_spacing_vertical(),
            split_gap: self.split_gap(),
            units_per_row: self.units_per_row(),
        }
    }
}

impl Default for KeyboardGeometryOptions {
    fn default() -> Self {
        Self::from_values(KeyboardGeometryOptionsValues::default())
    }
}
impl Clone for KeyboardGeometryOptions {
    fn clone(&self) -> Self {
        Self::from_values(self.values())
    }
}
impl PartialEq for KeyboardGeometryOptions {
    fn eq(&self, other: &Self) -> bool {
        fn eq(a: f64, b: f64) -> bool {
            a == b || a.is_nan() && b.is_nan()
        }
        std::ptr::eq(self, other)
            || (eq(
                self.key_spacing_horizontal(),
                other.key_spacing_horizontal(),
            ) && eq(self.key_spacing_vertical(), other.key_spacing_vertical())
                && eq(self.split_gap(), other.split_gap())
                && eq(self.units_per_row(), other.units_per_row()))
    }
}
impl Eq for KeyboardGeometryOptions {}
impl Hash for KeyboardGeometryOptions {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for value in [
            self.key_spacing_horizontal(),
            self.key_spacing_vertical(),
            self.split_gap(),
            self.units_per_row(),
        ] {
            super::float_hash::hash_float(value, state);
        }
    }
}

#[cfg(test)]
#[path = "geometry_options_tests.rs"]
mod tests;
