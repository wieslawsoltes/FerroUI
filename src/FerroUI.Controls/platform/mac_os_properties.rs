use crate::TrayIcon;
use ferroui_base::{ferro_properties, ferro_static_type, AttachedProperty, FerroProperty, FerroPropertyChangedEventArgs};

/// Set of MacOS specific attached properties that allow deeper
/// customization of the application per platform.
pub struct MacOSProperties;

ferro_static_type!(MacOSProperties);

ferro_properties! {
    impl MacOSProperties {
        /// Defines the IsTemplateIcon attached property.
        pub fn is_template_icon_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<MacOSProperties, TrayIcon, _>("IsTemplateIcon", false)
        }
    }
}

impl MacOSProperties {
    fn static_constructor() {
        Self::is_template_icon_property()
            .changed()
            .add_class_handler::<TrayIcon>(Self::tray_icon_is_template_icon_changed);
    }

    /// A Boolean value that determines whether the TrayIcon image
    /// represents a template image.
    pub fn set_is_template_icon(obj: &TrayIcon, value: bool) {
        obj.set_value(Self::is_template_icon_property(), value)
    }

    /// Returns a Boolean value that indicates whether the TrayIcon image is
    /// a template image.
    pub fn get_is_template_icon(obj: &TrayIcon) -> bool {
        obj.get_value(Self::is_template_icon_property())
    }

    fn tray_icon_is_template_icon_changed(tray_icon: &TrayIcon, args: &FerroPropertyChangedEventArgs<'_>) {
        if let Some(impl_) = tray_icon.platform_impl() {
            if let Some(template_impl) = impl_.as_tray_icon_with_is_template_impl() {
                template_impl.set_is_template_icon(args.get_new_value::<bool>());
            }
        }
    }
}
