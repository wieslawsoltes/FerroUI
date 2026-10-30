// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::data::core::ValueTypes;
use ferroui_base::BoxedValue;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct ValueKey(pub BoxedValue);
impl PartialEq for ValueKey {
    fn eq(&self, other: &Self) -> bool {
        if Rc::ptr_eq(&self.0, &other.0) {
            return true;
        }
        match (
            ValueTypes::as_object(&*self.0),
            ValueTypes::as_object(&*other.0),
        ) {
            (Some(a), Some(b)) => a.ptr_eq(&b),
            _ => (*self.0).value_eq(&*other.0),
        }
    }
}
impl Eq for ValueKey {}
impl Hash for ValueKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(object) = ValueTypes::as_object(&*self.0) {
            // Native class handles have reference identity regardless of the
            // base/derived handle type carried by the box.
            object.hash(state);
            return;
        }
        self.0.value_type_id().hash(state);
        macro_rules! scalar {
            ($($t:ty),+ $(,)?) => { $(if let Some(v) = self.0.downcast_ref::<$t>() {v.hash(state);return;})+ };
        }
        scalar!(String, bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
        if let Some(value) = self.0.downcast_ref::<f64>() {
            (if value.is_nan() {
                f64::NAN.to_bits()
            } else if *value == 0.0 {
                0
            } else {
                value.to_bits()
            })
            .hash(state);
        } else if let Some(value) = self.0.downcast_ref::<f32>() {
            (if value.is_nan() {
                f32::NAN.to_bits()
            } else if *value == 0.0 {
                0
            } else {
                value.to_bits()
            })
            .hash(state);
        }
        // Shared AnyValue exposes equality, but no custom GetHashCode. A common
        // bucket for other values of the same type retains their full equality
        // contract. Arbitrary-value hash performance remains a runtime gap.
    }
}
