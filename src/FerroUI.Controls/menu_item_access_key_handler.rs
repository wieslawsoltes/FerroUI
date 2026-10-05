use ferroui_base::input::{AccessKeyHandler, IAccessKeyHandler, IMainMenu, InputElement, TextInputEventArgs};
use ferroui_base::Ref;
use std::rc::{Rc, Weak};

/// Handles access keys within a menu item.
///
/// It is the access key handler of a window with the handlers attached to
/// the owner replaced: instead of the Alt key handling of a window, it
/// processes text input in the popup of the menu item.
pub(crate) struct MenuItemAccessKeyHandler {
    this: Weak<MenuItemAccessKeyHandler>,
    base: Rc<AccessKeyHandler>,
}

impl MenuItemAccessKeyHandler {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), base: AccessKeyHandler::new() })
    }

    /// Handles a key being pressed in the menu.
    fn on_text_input(&self, e: &TextInputEventArgs) {
        let Some(key) = e.text.clone().filter(|text| !text.trim().is_empty()) else { return };

        let registration = self
            .base
            .registrations()
            .into_iter()
            .find(|(registered, _)| registered.to_uppercase() == key.to_uppercase());

        let Some((_, element)) = registration else { return };

        e.set_handled(self.base.process_key(Some(&key), Some(&element)));
    }
}

impl IAccessKeyHandler for MenuItemAccessKeyHandler {
    fn main_menu(&self) -> Option<Rc<dyn IMainMenu>> {
        self.base.main_menu()
    }

    fn set_main_menu(&self, value: Option<Rc<dyn IMainMenu>>) {
        self.base.set_main_menu(value);
    }

    fn set_owner(&self, owner: &Ref<InputElement>) {
        self.base.set_owner_without_handlers(owner);

        let weak = self.this.clone();
        owner.add_handler(InputElement::text_input_event(), move |_, e: &TextInputEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.on_text_input(e);
            }
        });
    }

    fn register(&self, access_key: &str, element: &Ref<InputElement>) {
        self.base.register(access_key, element);
    }

    fn unregister(&self, element: &InputElement) {
        self.base.unregister(element);
    }
}
