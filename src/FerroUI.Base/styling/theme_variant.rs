use crate::controls::ResourceKey;
use crate::platform::{IPlatformSettings, PlatformThemeVariant};
use crate::{FerroLocator, LocatorExtensions, TypeInfo};
use crate::{ferro_property, FerroObject, FerroProperty, StyledElement, StyledProperty, StyledPropertyOptions};
use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str::FromStr;

struct Inner {
    key: ResourceKey,
    inherit_variant: Option<ThemeVariant>,
}

/// Specifies a UI theme variant that should be used for the control and
/// application types.
///
/// Theme variants are compared by their key.
#[derive(Clone)]
pub struct ThemeVariant(Rc<Inner>);

thread_local! {
    static DEFAULT: ThemeVariant = ThemeVariant::from_key("Default".into());
    static LIGHT: ThemeVariant = ThemeVariant::from_key("Light".into());
    static DARK: ThemeVariant = ThemeVariant::from_key("Dark".into());
}

thread_local! {
    /// The classes, other than styled elements, whose objects are theme
    /// variant roots.
    static ROOT_TYPES: RefCell<Vec<&'static TypeInfo>> = const { RefCell::new(Vec::new()) };
}

impl ThemeVariant {
    /// Creates a theme variant.
    ///
    /// `key` is the key of the custom theme variant. `inherit_variant` is the
    /// variant that this variant inherits from: if set, a resource that is
    /// missing from the theme dictionary of the current variant is looked up
    /// in the dictionary of the inherited one. Inheriting the default variant
    /// is not supported.
    pub fn new(key: impl Into<ResourceKey>, inherit_variant: Option<ThemeVariant>) -> Self {
        if inherit_variant.as_ref().is_some_and(|v| *v == Self::default()) {
            panic!("Inheriting default theme variant is not supported.");
        }
        ThemeVariant(Rc::new(Inner { key: key.into(), inherit_variant }))
    }

    fn from_key(key: ResourceKey) -> Self {
        ThemeVariant(Rc::new(Inner { key, inherit_variant: None }))
    }

    /// Use the default theme variant: inherit the variant from the parent,
    /// and at the root use the variant of the platform.
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> ThemeVariant {
        DEFAULT.with(Clone::clone)
    }

    /// Use the Light theme variant.
    pub fn light() -> ThemeVariant {
        LIGHT.with(Clone::clone)
    }

    /// Use the Dark theme variant.
    pub fn dark() -> ThemeVariant {
        DARK.with(Clone::clone)
    }

    ferro_property!(for StyledElement;
        /// Defines the ActualThemeVariant property: the theme variant in
        /// effect for an element, inherited down the tree. It is `None`
        /// until a root or an element with a requested variant sets it.
        pub fn actual_theme_variant_property() -> StyledProperty<Option<ThemeVariant>> {
            FerroProperty::register_with::<StyledElement, _>(
                "ActualThemeVariant",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
    );

    ferro_property!(for StyledElement;
        /// Defines the RequestedThemeVariant property: the theme variant
        /// requested for an element and its descendants.
        pub fn requested_theme_variant_property() -> StyledProperty<Option<ThemeVariant>> {
            let property = FerroProperty::register::<StyledElement, _>(
                "RequestedThemeVariant",
                Some(ThemeVariant::default()),
            );
            property.changed().add_class_handler::<FerroObject>(|x, _| ThemeVariant::update_actual_theme_variant(x));
            property
        }
    );

    /// Key of the theme variant by which variants are compared.
    pub fn key(&self) -> &ResourceKey {
        &self.0.key
    }

    /// The theme variant that this variant inherits from.
    pub fn inherit_variant(&self) -> Option<ThemeVariant> {
        self.0.inherit_variant.clone()
    }

    /// Updates the actual theme variant of `target` from its requested
    /// theme variant: a requested variant other than the default becomes the
    /// actual one, otherwise the actual variant is inherited.
    pub fn update_actual_theme_variant(target: &FerroObject) {
        let requested =
            target.get_value(Self::requested_theme_variant_property()).unwrap_or_else(ThemeVariant::default);
        if requested != ThemeVariant::default() {
            target.set_value(Self::actual_theme_variant_property(), Some(requested));
            return;
        }

        let is_root = target.downcast_ref::<StyledElement>().is_some_and(|element| element.is_theme_variant_root())
            || ROOT_TYPES.with_borrow(|types| types.iter().any(|type_| type_.is_assignable_from(target.get_type())));
        if is_root {
            target.set_value(Self::actual_theme_variant_property(), Some(Self::get_platform_theme_variant()));
        } else {
            target.clear_value(Self::actual_theme_variant_property());
        }
    }

    /// Declares that the objects of a class that is not a styled element
    /// (the application class) are roots of theme variant resolution.
    /// Styled elements say so through the `is_theme_variant_root` virtual.
    /// (Upstream: the `IThemeVariantRoot` interface.)
    pub fn register_theme_variant_root_type(type_: &'static TypeInfo) {
        ROOT_TYPES.with_borrow_mut(|types| {
            if !types.iter().any(|t| std::ptr::eq(*t, type_)) {
                types.push(type_);
            }
        });
    }

    /// The theme variant of the platform: the one of the colour values of
    /// the platform settings, light when there are no platform settings.
    fn get_platform_theme_variant() -> ThemeVariant {
        let variant = FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .map(|settings| settings.get_color_values().theme_variant())
            .unwrap_or(PlatformThemeVariant::Light);

        ThemeVariant::from(variant)
    }

    /// The platform theme variant of a theme variant (the original's
    /// explicit conversion to a nullable platform theme variant): the light
    /// and the dark variant convert to their counterparts, another variant
    /// converts as the variant it inherits from, and a variant that inherits
    /// from neither, or no variant, converts to nothing.
    pub fn to_platform_theme_variant(theme_variant: Option<&ThemeVariant>) -> Option<PlatformThemeVariant> {
        let theme_variant = theme_variant?;
        if *theme_variant == ThemeVariant::light() {
            Some(PlatformThemeVariant::Light)
        } else if *theme_variant == ThemeVariant::dark() {
            Some(PlatformThemeVariant::Dark)
        } else if let Some(inherit_variant) = theme_variant.inherit_variant() {
            Self::to_platform_theme_variant(Some(&inherit_variant))
        } else {
            None
        }
    }
}

/// The original's explicit conversion from a platform theme variant.
impl From<PlatformThemeVariant> for ThemeVariant {
    fn from(theme_variant: PlatformThemeVariant) -> Self {
        match theme_variant {
            PlatformThemeVariant::Light => ThemeVariant::light(),
            PlatformThemeVariant::Dark => ThemeVariant::dark(),
        }
    }
}

impl Default for ThemeVariant {
    fn default() -> Self {
        ThemeVariant::default()
    }
}

impl PartialEq for ThemeVariant {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.0.key == other.0.key
    }
}

impl Eq for ThemeVariant {}

impl Hash for ThemeVariant {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.key.hash(state)
    }
}

impl fmt::Display for ThemeVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.key, f)
    }
}

impl fmt::Debug for ThemeVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.key, f)
    }
}

/// The error returned when parsing a theme variant from a string fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeVariantParseError;

impl fmt::Display for ThemeVariantParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Only the built in theme variants (Default, Light, Dark) can be converted from a string.")
    }
}

impl std::error::Error for ThemeVariantParseError {}

impl FromStr for ThemeVariant {
    type Err = ThemeVariantParseError;

    /// Converts the name of a built in theme variant.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Default" => Ok(ThemeVariant::default()),
            "Light" => Ok(ThemeVariant::light()),
            "Dark" => Ok(ThemeVariant::dark()),
            _ => Err(ThemeVariantParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of the conversions.
    use super::*;

    #[test]
    fn platform_theme_variant_converts_to_the_built_in_variants() {
        assert_eq!(ThemeVariant::light(), ThemeVariant::from(PlatformThemeVariant::Light));
        assert_eq!(ThemeVariant::dark(), ThemeVariant::from(PlatformThemeVariant::Dark));
    }

    #[test]
    fn theme_variant_converts_to_the_platform_variant_it_is_or_inherits() {
        assert_eq!(None, ThemeVariant::to_platform_theme_variant(None));
        assert_eq!(None, ThemeVariant::to_platform_theme_variant(Some(&ThemeVariant::default())));
        assert_eq!(
            Some(PlatformThemeVariant::Light),
            ThemeVariant::to_platform_theme_variant(Some(&ThemeVariant::light()))
        );
        assert_eq!(
            Some(PlatformThemeVariant::Dark),
            ThemeVariant::to_platform_theme_variant(Some(&ThemeVariant::dark()))
        );

        let custom = ThemeVariant::new("Custom", None);
        assert_eq!(None, ThemeVariant::to_platform_theme_variant(Some(&custom)));

        let inherited = ThemeVariant::new("Inherited", Some(ThemeVariant::dark()));
        let nested = ThemeVariant::new("Nested", Some(inherited));
        assert_eq!(Some(PlatformThemeVariant::Dark), ThemeVariant::to_platform_theme_variant(Some(&nested)));
    }
}
