//! The conversions that keep setters short: what a setter of a property
//! takes besides the value of the property itself (see
//! [`IntoValue`](crate::builder::IntoValue)).
//!
//! | given                         | property value                     |
//! |-------------------------------|------------------------------------|
//! | `T`                           | `Option<T>`                        |
//! | `&str`, `&String`             | `String`, `Option<String>`         |
//! | an integer, `f64`             | `Option<String>` (its text)        |
//! | an integer, `f32`             | `f64`                              |
//! | `f64`, `(f64, f64)`, `(f64, f64, f64, f64)` | `Thickness`          |
//! | `f64`                         | `CornerRadius`                     |
//! | `Color`, a color name or `#rgb` text | `Option<Rc<dyn IBrush>>`    |
//! | `&str`                        | `Option<BoxedValue>` (text content)|
//!
//! A color text that does not parse panics with the text: it is a mistake
//! in the source, like an attribute XAML does not compile.

use crate::builder::{FromValue, IntoValue};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brush, Color, IBrush};
use ferroui_base::{BoxedValue, CornerRadius, PropertyValue, Thickness};
use std::rc::Rc;

impl<T: PropertyValue> IntoValue<Option<T>> for T {
    fn into_value(self) -> Option<T> {
        Some(self)
    }
}

impl<T: Clone> FromValue<Option<T>> for T {
    fn from_value(value: &Option<T>) -> Option<Self> {
        value.clone()
    }
}

impl IntoValue<String> for &str {
    fn into_value(self) -> String {
        self.to_string()
    }
}

impl IntoValue<Option<String>> for &str {
    fn into_value(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoValue<Option<String>> for &String {
    fn into_value(self) -> Option<String> {
        Some(self.clone())
    }
}

impl IntoValue<Option<BoxedValue>> for &str {
    fn into_value(self) -> Option<BoxedValue> {
        Some(Rc::new(self.to_string()))
    }
}

impl IntoValue<Option<BoxedValue>> for String {
    fn into_value(self) -> Option<BoxedValue> {
        Some(Rc::new(self))
    }
}

macro_rules! number_to_text {
    ($($t:ty),*) => {$(
        impl IntoValue<Option<String>> for $t {
            fn into_value(self) -> Option<String> {
                Some(self.to_string())
            }
        }
        impl IntoValue<Option<BoxedValue>> for $t {
            fn into_value(self) -> Option<BoxedValue> {
                Some(Rc::new(self.to_string()))
            }
        }
    )*};
}

number_to_text!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

macro_rules! number_to_f64 {
    ($($t:ty),*) => {$(
        impl IntoValue<f64> for $t {
            fn into_value(self) -> f64 {
                f64::from(self)
            }
        }
    )*};
}

number_to_f64!(i8, i16, i32, u8, u16, u32, f32);

impl IntoValue<Thickness> for f64 {
    fn into_value(self) -> Thickness {
        Thickness::uniform(self)
    }
}

impl IntoValue<Thickness> for i32 {
    fn into_value(self) -> Thickness {
        Thickness::uniform(f64::from(self))
    }
}

impl IntoValue<Thickness> for (f64, f64) {
    fn into_value(self) -> Thickness {
        Thickness::symmetric(self.0, self.1)
    }
}

impl IntoValue<Thickness> for (f64, f64, f64, f64) {
    fn into_value(self) -> Thickness {
        Thickness::new(self.0, self.1, self.2, self.3)
    }
}

impl IntoValue<CornerRadius> for f64 {
    fn into_value(self) -> CornerRadius {
        CornerRadius::uniform(self)
    }
}

impl IntoValue<CornerRadius> for i32 {
    fn into_value(self) -> CornerRadius {
        CornerRadius::uniform(f64::from(self))
    }
}

impl IntoValue<Option<Rc<dyn IBrush>>> for Color {
    fn into_value(self) -> Option<Rc<dyn IBrush>> {
        Some(Rc::new(ImmutableSolidColorBrush::new(self)))
    }
}

impl IntoValue<Option<Rc<dyn IBrush>>> for &str {
    fn into_value(self) -> Option<Rc<dyn IBrush>> {
        Some(Brush::parse(self).unwrap_or_else(|_| panic!("`{self}` is not a color")))
    }
}
