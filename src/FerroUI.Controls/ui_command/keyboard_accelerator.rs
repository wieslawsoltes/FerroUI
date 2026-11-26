//! WinUI's `KeyboardAccelerator` as the description of a key gesture of an `XamlUICommand`
//! (`Microsoft.UI.Xaml.Input.KeyboardAccelerator` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};

/// A key combination of a command (WinUI's `KeyboardAccelerator`): its [`key`](Self::key) with
/// its [`modifiers`](Self::modifiers).
///
/// FerroUI's controls are invoked by their `HotKey`: an [`XamlUICommand`](super::XamlUICommand)
/// gives the gesture of its first enabled accelerator to the sources it is set on. An accelerator
/// is not set on an element directly (WinUI's `UIElement.KeyboardAccelerators` is the element's
/// `HotKey`, docs/porting/winui-controls.md, section 10.16).
#[repr(C)]
pub struct KeyboardAccelerator {
    base: FerroObject,
}

ferro_class!(KeyboardAccelerator: FerroObject);
ferro_class_info!(KeyboardAccelerator {
    new: KeyboardAccelerator::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(KeyboardAccelerator: FerroObjectImpl);

ferro_properties! { impl KeyboardAccelerator {
    /// Defines the `Key` property. Default [`Key::None`].
    pub fn key_property() -> StyledProperty<Key> {
        FerroProperty::register::<KeyboardAccelerator, _>("Key", Key::None)
    }

    /// Defines the `Modifiers` property. Default none.
    pub fn modifiers_property() -> StyledProperty<KeyModifiers> {
        FerroProperty::register::<KeyboardAccelerator, _>("Modifiers", KeyModifiers::NONE)
    }

    /// Defines the `IsEnabled` property. Default `true`.
    pub fn is_enabled_property() -> StyledProperty<bool> {
        FerroProperty::register::<KeyboardAccelerator, _>("IsEnabled", true)
    }
} }

impl KeyboardAccelerator {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates an accelerator of a key and its modifiers.
    pub fn with_key(key: Key, modifiers: KeyModifiers) -> Ref<Self> {
        let accelerator = Self::new();
        accelerator.set_key(key);
        accelerator.set_modifiers(modifiers);
        accelerator
    }

    pub fn key(&self) -> Key {
        self.get_value(Self::key_property())
    }

    pub fn set_key(&self, value: Key) {
        self.set_value(Self::key_property(), value);
    }

    pub fn modifiers(&self) -> KeyModifiers {
        self.get_value(Self::modifiers_property())
    }

    pub fn set_modifiers(&self, value: KeyModifiers) {
        self.set_value(Self::modifiers_property(), value);
    }

    pub fn is_enabled(&self) -> bool {
        self.get_value(Self::is_enabled_property())
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.set_value(Self::is_enabled_property(), value);
    }

    /// The key gesture of the accelerator; `None` while its key is [`Key::None`].
    pub fn key_gesture(&self) -> Option<KeyGesture> {
        let key = self.key();
        (key != Key::None).then(|| KeyGesture::new(key, self.modifiers()))
    }
}
