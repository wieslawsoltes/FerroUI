//! Resource references of the builders: a property that follows a resource
//! ([`dynamic_resource`](crate::El::dynamic_resource)) or takes it once
//! ([`static_resource`](crate::El::static_resource)), with a typed key of a theme or text.
//!
//! ```ignore
//! use ferroui_themes_fluent::keys;
//!
//! border()
//!     .dynamic_resource(Border::background_property(), keys::CardBackgroundFillColorDefaultBrush)
//!     .static_resource(Border::corner_radius_property(), keys::ControlCornerRadius)
//! ```
//!
//! A typed key does not compile with a property its value does not convert to. The reference
//! is a value of the element, and follows the resource through the resource observable of the
//! control, so a change of the theme variant reaches the property with no component running
//! again.

use crate::builder::IntoValue;
use crate::conversions::brush_of;
use ferroui_base::controls::{ResourceConverter, ResourceHostRef, ResourceKey, ThemeResourceKey};
use ferroui_base::data::core::UntypedObservableBindingExpression;
use ferroui_base::data::{BindingBase, BindingExpressionBase, BindingPriority};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::{IObservable, ObservableExt};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref, StyledElement, StyledValue, UnsetValueType};
use std::any::{Any, TypeId};
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
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a resource key for a property of type `{T}`",
    label = "the value of this key does not convert to the property",
    note = "use a key of the `keys` module of a theme whose value fits the property, or the text of a key"
)]
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

fn is_unset(value: &Option<BoxedValue>) -> bool {
    value.as_ref().is_some_and(|value| value.is::<UnsetValueType>())
}

/// A resource as a binding: of an element of the builders, or of a setter of a theme.
pub(crate) struct ResourceBinding {
    key: ResourceKey,
    dynamic: bool,
    convert: ResourceValueConverter,
}

impl ResourceBinding {
    pub(crate) fn new(key: ResourceKey, dynamic: bool, convert: ResourceValueConverter) -> Self {
        Self { key, dynamic, convert }
    }
}

/// The binding of `key` for a property with a value of type `T`.
pub(crate) fn resource_binding<T, K: ResourceKeyFor<T>>(key: K, dynamic: bool) -> ResourceBinding {
    ResourceBinding::new(key.resource_key(), dynamic, K::converter())
}

impl BindingBase for ResourceBinding {
    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn create_instance(
        &self,
        target: &FerroObject,
        _target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let convert = self.convert;
        let converter: ResourceConverter = Rc::new(move |value: Option<BoxedValue>| {
            if is_unset(&value) {
                return value;
            }
            convert(&value).or_else(|| Some(FerroProperty::unset_value()))
        });
        let element = target.to_ref().cast::<StyledElement>().expect("a resource is bound on a styled element");
        let observable = ResourceHostRef::from(&element).resource_observable(self.key.clone(), Some(converter));
        let observable: Rc<dyn IObservable<Option<BoxedValue>>> =
            if self.dynamic { observable } else { observable.where_(|value| !is_unset(value)).take(1) };
        UntypedObservableBindingExpression::new(observable, BindingPriority::LocalValue)
    }
}
