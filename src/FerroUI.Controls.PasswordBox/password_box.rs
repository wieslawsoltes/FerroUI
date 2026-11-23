//! WinUI's `PasswordBox` on `TextBox`: the password is the text of the box, which the box masks.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ControlImpl, TextBox, TextBoxImpl};
use std::cell::Cell;
use std::rc::Rc;

/// The style class of the text box themes that shows the button that reveals the password.
const REVEAL_BUTTON_CLASS: &str = "revealPasswordButton";

/// How the password can be shown (WinUI `PasswordRevealMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PasswordRevealMode {
    /// A button in the box shows the password.
    #[default]
    Peek = 0,
    /// The password is always masked, and the box has no button.
    Hidden = 1,
    /// The password is shown.
    Visible = 2,
}

/// A text box for a password.
#[repr(C)]
pub struct PasswordBox {
    base: TextBox,
    /// Set while the box copies between `Text` and `Password`.
    syncing: Cell<bool>,
    password_changed: Rc<HandlerList<dyn Fn()>>,
}

ferro_class!(PasswordBox: TextBox);
ferroui_base::ferro_class_info!(PasswordBox { new: PasswordBox::new });
ferro_impl_classes!(
    PasswordBox: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TextBoxImpl
);

impl StyledElementImpl for PasswordBox {
    // The themes of the text box are the themes of the password box.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TextBox as StaticType>::TYPE
    }
}

impl FerroObjectImpl for PasswordBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_reveal_mode();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == TextBox::text_property().as_property() {
            if !this.syncing.replace(true) {
                let text = this.text().unwrap_or_default();
                this.set_current_value(Self::password_property(), text);
                this.syncing.set(false);
                this.raise_password_changed();
            }
        } else if property == Self::password_property().as_property() {
            if !this.syncing.replace(true) {
                let password = this.password();
                this.set_text(Some(&password));
                this.syncing.set(false);
                this.raise_password_changed();
            }
        } else if property == Self::password_reveal_mode_property().as_property() {
            this.update_reveal_mode();
        }
    }
}

ferro_properties! {
    impl PasswordBox {
        /// Defines the `Password` property.
        pub fn password_property() -> StyledProperty<String> {
            FerroProperty::register::<PasswordBox, _>("Password", String::new())
        }

        /// Defines the `PasswordRevealMode` property.
        pub fn password_reveal_mode_property() -> StyledProperty<PasswordRevealMode> {
            FerroProperty::register::<PasswordBox, _>("PasswordRevealMode", PasswordRevealMode::Peek)
        }
    }
}

impl PasswordBox {
    fn static_constructor() {
        // WinUI's default `PasswordChar`.
        TextBox::password_char_property().override_default_value::<PasswordBox>('\u{25CF}');
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TextBox::construct(),
            syncing: Cell::new(false),
            password_changed: Rc::new(HandlerList::new()),
        }
    }

    /// Creates an empty password box.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The password.
    pub fn password(&self) -> String {
        self.get_value(Self::password_property())
    }

    pub fn set_password(&self, value: &str) {
        self.set_value(Self::password_property(), value.to_owned());
    }

    /// How the password can be shown. The default is `Peek`.
    pub fn password_reveal_mode(&self) -> PasswordRevealMode {
        self.get_value(Self::password_reveal_mode_property())
    }

    pub fn set_password_reveal_mode(&self, value: PasswordRevealMode) {
        self.set_value(Self::password_reveal_mode_property(), value);
    }

    /// Occurs when the password has changed.
    pub fn password_changed(&self, handler: impl Fn() + 'static) -> Subscription {
        let handlers = self.password_changed.clone();
        let token = handlers.add(Rc::new(handler));
        Subscription::new(Disposable::create(move || {
            handlers.remove(token);
        }))
    }

    fn raise_password_changed(&self) {
        for (_, handler) in self.password_changed.snapshot().iter() {
            handler();
        }
    }

    fn update_reveal_mode(&self) {
        let mode = self.password_reveal_mode();
        let classes = self.classes();
        if mode == PasswordRevealMode::Peek {
            if !classes.contains(REVEAL_BUTTON_CLASS) {
                classes.add(REVEAL_BUTTON_CLASS);
            }
        } else {
            classes.remove(REVEAL_BUTTON_CLASS);
        }
        // Peek starts masked; its button sets `RevealPassword` while it is checked.
        self.set_reveal_password(mode == PasswordRevealMode::Visible);
    }
}
