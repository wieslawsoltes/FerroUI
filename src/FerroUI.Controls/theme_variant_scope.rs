use crate::{ControlImpl, Decorator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};

/// Decorator control that isolates a subtree of controls with a locally
/// defined [`ThemeVariant`].
#[repr(C)]
pub struct ThemeVariantScope {
    base: Decorator,
}

ferro_class!(ThemeVariantScope: Decorator);
ferro_class_info!(ThemeVariantScope { new: ThemeVariantScope::new });
ferro_impl_classes!(
    ThemeVariantScope: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl ThemeVariantScope {
    /// Defines the `ActualThemeVariant` property: the property of
    /// [`ThemeVariant`], which every styled element owns.
    pub fn actual_theme_variant_property() -> &'static StyledProperty<Option<ThemeVariant>> {
        ThemeVariant::actual_theme_variant_property()
    }

    /// Defines the `RequestedThemeVariant` property: the property of
    /// [`ThemeVariant`], which every styled element owns.
    pub fn requested_theme_variant_property() -> &'static StyledProperty<Option<ThemeVariant>> {
        ThemeVariant::requested_theme_variant_property()
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Decorator::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The UI theme variant that is used by the control (and its child
    /// elements) for resource determination. The UI theme specified with it
    /// can override the app-level theme variant.
    ///
    /// Setting it to the default theme variant applies the actual theme
    /// variant of the parent on the current scope.
    pub fn requested_theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(Self::requested_theme_variant_property())
    }

    pub fn set_requested_theme_variant(&self, value: Option<ThemeVariant>) {
        self.set_value(Self::requested_theme_variant_property(), value)
    }
}
