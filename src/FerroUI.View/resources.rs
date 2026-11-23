//! Resource references of the builders: a property that follows a resource
//! ([`dynamic_resource`](crate::El::dynamic_resource)) or takes it once
//! ([`static_resource`](crate::El::static_resource)), with a typed key of a theme or text.
//!
//! ```ignore
//! use ferroui_themes_fluent::keys;
//!
//! border()
//!     .dynamic_resource(Border::background_property(), keys::SystemControlBackgroundAltHighBrush)
//!     .static_resource(Border::corner_radius_property(), keys::ControlCornerRadius)
//! ```
//!
//! A typed key does not compile with a property its value does not convert to. The reference
//! is a value of the element, and follows the resource through the resource observable of the
//! control, so a change of the theme variant reaches the property with no component running
//! again.

use crate::builder::IntoValue;
use crate::conversions::brush_of;
use ferroui_base::controls::{ResourceKey, ThemeResourceKey};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{BoxedValue, StyledValue};
use std::any::TypeId;
use std::rc::Rc;

/// The value of a property from the value of a resource; `None` when it is not one.
pub type ResourceValueConverter = fn(&Option<BoxedValue>) -> Option<BoxedValue>;

/// A key a property with a value of type `T` can follow: a typed key of a theme whose value
/// converts to `T`, or text, which is checked when the application runs.
///
/// ```
/// use ferroui_base::media::Color;
/// use ferroui_view::prelude::*;
/// const ACCENT: ThemeResourceKey<Color> = ThemeResourceKey::new("Accent");
/// let _ = text_block().dynamic_resource(TextBlock::foreground_property(), ACCENT);
/// ```
///
/// A value that does not convert to the property does not compile:
///
/// ```compile_fail
/// use ferroui_base::media::Color;
/// use ferroui_view::prelude::*;
/// const ACCENT: ThemeResourceKey<Color> = ThemeResourceKey::new("Accent");
/// let _ = text_block().dynamic_resource(TextBlock::font_size_property(), ACCENT);
/// ```
///
/// A key of a theme that does not exist does not compile, because the keys are constants of the
/// `keys` module of the theme. A key the theme has with no single value type is a
/// `ThemeResourceKey<BoxedValue>`, used through its text: `key.name()`.
pub trait ResourceKeyFor<T> {
    /// The key.
    fn resource_key(self) -> ResourceKey;
    /// The value of the property from the value of the resource.
    fn converter() -> ResourceValueConverter;
}

fn typed_value<T: StyledValue, V: IntoValue<T> + Clone + 'static>(value: &Option<BoxedValue>) -> Option<BoxedValue> {
    from_markup_value::<V>(value).map(|value| Rc::new(value.into_value()) as BoxedValue)
}

impl<T: StyledValue, V: IntoValue<T> + Clone + 'static> ResourceKeyFor<T> for ThemeResourceKey<V> {
    fn resource_key(self) -> ResourceKey {
        self.into()
    }

    fn converter() -> ResourceValueConverter {
        typed_value::<T, V>
    }
}

/// The value of a resource named by text: of the type of the property, a brush for a color.
fn text_value<T: StyledValue>(value: &Option<BoxedValue>) -> Option<BoxedValue> {
    if TypeId::of::<T>() == TypeId::of::<Option<Rc<dyn IBrush>>>() {
        let brush = from_markup_value::<Rc<dyn IBrush>>(value)
            .or_else(|| from_markup_value::<Color>(value).map(brush_of));
        return brush.map(|brush| Rc::new(Some(brush)) as BoxedValue);
    }
    from_markup_value::<T>(value).map(|value| Rc::new(value) as BoxedValue)
}

impl<T: StyledValue> ResourceKeyFor<T> for &str {
    fn resource_key(self) -> ResourceKey {
        self.into()
    }

    fn converter() -> ResourceValueConverter {
        text_value::<T>
    }
}
