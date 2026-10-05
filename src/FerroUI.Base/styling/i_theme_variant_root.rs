/// Marks an object that can be the root of theme variant resolution: at a
/// root, the default theme variant resolves to the variant of the platform
/// instead of being inherited.
pub trait IThemeVariantRoot {
    fn is_theme_variant_root(&self) -> bool;
}
