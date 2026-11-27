//! Comparing two boxed property values (#566): `value_eq`, what the property system calls for an
//! untyped value, on the kinds of comparison that take different paths through it. The boxes are
//! distinct, so no benchmark stops at the identity of the box.

use std::hint::black_box;
use std::rc::Rc;

use ferroui_base::BoxedValue;

use crate::harness::Registry;

/// The comparisons of one invocation.
const COMPARES: u32 = 256;

pub struct BoxedCompare {
    left: BoxedValue,
    right: BoxedValue,
}

impl BoxedCompare {
    fn new<A: PartialEq + 'static, B: PartialEq + 'static>(left: A, right: B) -> Self {
        Self { left: Rc::new(left), right: Rc::new(right) }
    }

    /// How many of the comparisons found the values equal.
    pub fn compare(&self) -> u32 {
        let mut equal = 0;
        for _ in 0..COMPARES {
            equal += u32::from(black_box(&*self.left).value_eq(black_box(&*self.right)));
        }
        equal
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("property_subsystem", "BoxedCompare");
    class
        .benchmark("equal", "", || BoxedCompare::new(1.5f64, 1.5f64), |b| b.compare())
        .operations_per_invoke(COMPARES);
    class
        .benchmark("different", "", || BoxedCompare::new(1.5f64, 2.5f64), |b| b.compare())
        .operations_per_invoke(COMPARES);
    class
        .benchmark("different_type", "", || BoxedCompare::new(1.5f64, 1i32), |b| b.compare())
        .operations_per_invoke(COMPARES);
    class
        .benchmark("not_a_number", "", || BoxedCompare::new(f64::NAN, f64::NAN), |b| b.compare())
        .operations_per_invoke(COMPARES);
    class
        .benchmark("nullable", "", || BoxedCompare::new(Some(1.5f64), Some(1.5f64)), |b| b.compare())
        .operations_per_invoke(COMPARES);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "BoxedCompare");
    }

    #[test]
    fn each_case_compares_as_its_name_says() {
        assert_eq!(BoxedCompare::new(1.5f64, 1.5f64).compare(), COMPARES);
        assert_eq!(BoxedCompare::new(1.5f64, 2.5f64).compare(), 0);
        assert_eq!(BoxedCompare::new(1.5f64, 1i32).compare(), 0);
        assert_eq!(BoxedCompare::new(f64::NAN, f64::NAN).compare(), COMPARES);
        assert_eq!(BoxedCompare::new(Some(1.5f64), Some(1.5f64)).compare(), COMPARES);
    }
}
