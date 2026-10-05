//! Cached boxes of the two boolean values: reading a boolean into an
//! untyped value does not allocate a new box per read.

use crate::BoxedValue;
use std::rc::Rc;

thread_local! {
    static TRUE: BoxedValue = Rc::new(true);
    static FALSE: BoxedValue = Rc::new(false);
}

/// The cached boxes of `true` and `false`.
pub struct BooleanBoxes;

impl BooleanBoxes {
    /// The box of `true` (one per thread).
    pub fn true_() -> BoxedValue {
        TRUE.with(Clone::clone)
    }

    /// The box of `false` (one per thread).
    pub fn false_() -> BoxedValue {
        FALSE.with(Clone::clone)
    }

    /// The cached box of `value`.
    pub fn box_(value: bool) -> BoxedValue {
        if value {
            Self::true_()
        } else {
            Self::false_()
        }
    }

    /// Boxes `value`: a boolean as its cached box, any other value as a new
    /// box.
    pub fn box_value<T: PartialEq + Clone + 'static>(value: &T) -> BoxedValue {
        let any: &dyn std::any::Any = value;
        match any.downcast_ref::<bool>() {
            Some(value) => Self::box_(*value),
            None => Rc::new(value.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booleans_share_their_boxes() {
        assert!(Rc::ptr_eq(&BooleanBoxes::box_(true), &BooleanBoxes::true_()));
        assert!(Rc::ptr_eq(&BooleanBoxes::box_(false), &BooleanBoxes::false_()));
        assert!(!Rc::ptr_eq(&BooleanBoxes::box_(true), &BooleanBoxes::box_(false)));
        assert_eq!(BooleanBoxes::box_(true).downcast_ref::<bool>(), Some(&true));
        assert_eq!(BooleanBoxes::box_(false).downcast_ref::<bool>(), Some(&false));
        assert!(Rc::ptr_eq(&BooleanBoxes::box_value(&true), &BooleanBoxes::box_value(&true)));
        let (a, b) = (BooleanBoxes::box_value(&1i32), BooleanBoxes::box_value(&1i32));
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(a.downcast_ref::<i32>(), Some(&1));
    }
}
