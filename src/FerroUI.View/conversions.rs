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
//! | `f64`, `(f64, f64)`, `(f64, f64, f64, f64)` | `CornerRadius`       |
//! | `Color`, a color name or `#rgb` text | `Option<Rc<dyn IBrush>>`    |
//! | `&str`                        | `Option<BoxedValue>` (text content)|
//!
//! A color text that does not parse panics with the text: it is a mistake
//! in the source, like an attribute XAML does not compile.
//!
//! The brush of a color is one object for every element that names the
//! color ([`brush_of`]): a list of a thousand rows written with `"#1E293B"`
//! holds one brush, not a thousand.

use crate::builder::{FromValue, IntoValue};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brush, Color, IBrush};
use ferroui_base::{BoxedValue, CornerRadius, PropertyValue, Thickness};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The colors whose brush is kept. A value that runs through colors (an
/// animated one, a color computed from data) gets a brush of its own for
/// each color past this many.
const KEPT_BRUSHES: usize = 1024;

ferroui_base::ferro_thread_local! {
    static BRUSHES: RefCell<HashMap<u32, Rc<dyn IBrush>>> = RefCell::new(HashMap::new());
}

/// The brush of `color`: the same object for the same color, on one thread,
/// for the first [`KEPT_BRUSHES`] colors. The brush cannot change, so what
/// shares it cannot tell.
pub(crate) fn brush_of(color: Color) -> Rc<dyn IBrush> {
    let make = || -> Rc<dyn IBrush> { Rc::new(ImmutableSolidColorBrush::new(color)) };
    BRUSHES
        .try_with(|brushes| {
            let mut brushes = brushes.borrow_mut();
            if let Some(brush) = brushes.get(&color.to_uint32()) {
                return brush.clone();
            }
            let brush = make();
            if brushes.len() < KEPT_BRUSHES {
                brushes.insert(color.to_uint32(), brush.clone());
            }
            brush
        })
        .unwrap_or_else(|_| make())
}

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

impl IntoValue<CornerRadius> for (f64, f64) {
    fn into_value(self) -> CornerRadius {
        CornerRadius::top_bottom(self.0, self.1)
    }
}

impl IntoValue<CornerRadius> for (f64, f64, f64, f64) {
    fn into_value(self) -> CornerRadius {
        CornerRadius::new(self.0, self.1, self.2, self.3)
    }
}

impl IntoValue<Option<Rc<dyn IBrush>>> for Color {
    fn into_value(self) -> Option<Rc<dyn IBrush>> {
        Some(brush_of(self))
    }
}

impl IntoValue<Option<Rc<dyn IBrush>>> for &str {
    fn into_value(self) -> Option<Rc<dyn IBrush>> {
        Some(match Color::try_parse(self) {
            Some(color) => brush_of(color),
            None => Brush::parse(self).unwrap_or_else(|_| panic!("`{self}` is not a color")),
        })
    }
}
