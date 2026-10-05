use crate::helpers::ComResultExt;
use crate::interop::*;
use ferroui_base::{ferro_property, AttachedProperty, FerroProperty, StaticType, TypeInfo};
use ferroui_controls::platform::INativeApplicationCommands;
use ferroui_controls::NativeMenu;
use ferroui_microcom::ComPtr;

/// The application-level commands of the macOS application menu.
pub struct MacOSNativeMenuCommands {
    commands: ComPtr<IFrnApplicationCommands>,
}

impl StaticType for MacOSNativeMenuCommands {
    const TYPE: &'static TypeInfo = {
        static TYPE: TypeInfo = TypeInfo::new("MacOSNativeMenuCommands", None);
        &TYPE
    };
}

impl MacOSNativeMenuCommands {
    ferro_property!(
        /// Marks a menu as the Services submenu of the application menu:
        /// the system fills it with the available services.
        pub fn is_services_submenu_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<MacOSNativeMenuCommands, NativeMenu, _>("IsServicesSubmenu", false)
        }
    );

    pub(crate) fn new(commands: ComPtr<IFrnApplicationCommands>) -> Self {
        Self { commands }
    }
}

impl INativeApplicationCommands for MacOSNativeMenuCommands {
    fn show_app(&self) {
        self.commands.unhide_app().check();
    }

    fn hide_app(&self) {
        self.commands.hide_app().check();
    }

    fn show_all(&self) {
        self.commands.show_all().check();
    }

    fn hide_others(&self) {
        self.commands.hide_others().check();
    }
}
