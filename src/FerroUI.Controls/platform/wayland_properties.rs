use super::IWaylandOptionsToplevelImplFeature;
use crate::{TopLevel, Window};
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_base::{
    ferro_properties, ferro_static_type, AttachedProperty, FerroProperty, FerroPropertyChangedEventArgs,
};
use std::rc::Rc;

/// Set of Wayland specific properties and events that allow deeper customization of the
/// application per platform.
pub struct WaylandProperties;

ferro_static_type!(WaylandProperties);

ferro_properties! {
    impl WaylandProperties {
        /// Defines the `AppId` attached property.
        ///
        /// Specifies the application ID (`app_id`) sent to the Wayland compositor via
        /// `xdg_toplevel` for window management, application identification, and window
        /// grouping.
        ///
        /// In Wayland environments, client windows do not directly provide their own window
        /// icons. Instead, compositors, docks, and taskbars resolve the application icon,
        /// launcher identity, and notifications by matching the `AppId` against the filename of
        /// the application's installed `.desktop` file (for example, using `org.example.myapp`
        /// to match `org.example.myapp.desktop`).
        pub fn app_id_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<WaylandProperties, Window, _>("AppId", None)
        }
    }
}

impl WaylandProperties {
    fn static_constructor() {
        Self::app_id_property().changed().add_class_handler::<TopLevel>(Self::on_app_id_changed).detach();
    }

    /// Sets the value of the `AppId` attached property of the specified window.
    pub fn set_app_id(obj: &Window, value: Option<String>) {
        obj.set_value(Self::app_id_property(), value)
    }

    /// Gets the value of the `AppId` attached property of the specified window.
    pub fn get_app_id(obj: &Window) -> Option<String> {
        obj.get_value(Self::app_id_property())
    }

    fn try_get_feature(sender: &TopLevel) -> Option<Rc<dyn IWaylandOptionsToplevelImplFeature>> {
        let platform_impl = sender.platform_impl()?;
        let provider: &dyn IOptionalFeatureProvider = &*platform_impl;
        provider.try_get::<dyn IWaylandOptionsToplevelImplFeature>()
    }

    fn on_app_id_changed(sender: &TopLevel, e: &FerroPropertyChangedEventArgs<'_>) {
        if let Some(feature) = Self::try_get_feature(sender) {
            feature.set_app_id(e.get_new_value::<Option<String>>().as_deref());
        }
    }
}
