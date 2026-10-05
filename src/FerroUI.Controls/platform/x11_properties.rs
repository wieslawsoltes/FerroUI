use super::{IX11OptionsToplevelImplFeature, X11NetWmWindowType};
use crate::{TopLevel, Window};
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_base::{
    ferro_properties, ferro_static_type, AttachedProperty, FerroProperty, FerroPropertyChangedEventArgs,
};
use std::rc::Rc;

/// Set of X11 specific attached properties that allow deeper customization
/// of the application per platform.
pub struct X11Properties;

ferro_static_type!(X11Properties);

ferro_properties! {
    impl X11Properties {
        /// Defines the `NetWmWindowType` attached property: the window type
        /// hint of the window.
        pub fn net_wm_window_type_property() -> AttachedProperty<X11NetWmWindowType> {
            FerroProperty::register_attached::<X11Properties, Window, _>("NetWmWindowType", X11NetWmWindowType::default())
        }

        /// Defines the `WmClass` attached property: the window class of
        /// the window.
        pub fn wm_class_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<X11Properties, Window, _>("WmClass", None)
        }
    }
}

impl X11Properties {
    fn static_constructor() {
        Self::net_wm_window_type_property().changed().add_class_handler::<TopLevel>(Self::on_net_wm_window_type_changed);
        Self::wm_class_property().changed().add_class_handler::<TopLevel>(Self::on_wm_class_changed);
    }

    pub fn set_net_wm_window_type(obj: &Window, value: X11NetWmWindowType) {
        obj.set_value(Self::net_wm_window_type_property(), value)
    }

    pub fn get_net_wm_window_type(obj: &Window) -> X11NetWmWindowType {
        obj.get_value(Self::net_wm_window_type_property())
    }

    pub fn set_wm_class(obj: &Window, value: Option<String>) {
        obj.set_value(Self::wm_class_property(), value)
    }

    pub fn get_wm_class(obj: &Window) -> Option<String> {
        obj.get_value(Self::wm_class_property())
    }

    fn try_get_feature(sender: &TopLevel) -> Option<Rc<dyn IX11OptionsToplevelImplFeature>> {
        let platform_impl = sender.platform_impl()?;
        let provider: &dyn IOptionalFeatureProvider = &*platform_impl;
        provider.try_get::<dyn IX11OptionsToplevelImplFeature>()
    }

    fn on_wm_class_changed(sender: &TopLevel, e: &FerroPropertyChangedEventArgs<'_>) {
        if let Some(feature) = Self::try_get_feature(sender) {
            feature.set_wm_class(e.get_new_value::<Option<String>>().as_deref());
        }
    }

    fn on_net_wm_window_type_changed(sender: &TopLevel, e: &FerroPropertyChangedEventArgs<'_>) {
        if let Some(feature) = Self::try_get_feature(sender) {
            feature.set_net_wm_window_type(e.get_new_value::<X11NetWmWindowType>());
        }
    }
}
