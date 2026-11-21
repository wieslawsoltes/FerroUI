use crate::{
    ObservableObject, ObservableObjectImpl,
    model::RibbonIcon,
    text::{RibbonString, whitespace},
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::RefCell, rc::Rc};

pub struct RibbonScreenTip {
    base: ObservableObject,
    title: RefCell<Option<RibbonString>>,
    description: RefCell<Option<RibbonString>>,
    shortcut: RefCell<Option<RibbonString>>,
    help_text: RefCell<Option<RibbonString>>,
    disabled_reason: RefCell<Option<RibbonString>>,
    image: RefCell<Option<Rc<RibbonIcon>>>,
    extended_description: RefCell<Option<RibbonString>>,
    extended_image: RefCell<Option<Rc<RibbonIcon>>>,
}
macro_rules! property {
    ($get:ident,$set:ident,$ty:ty,$name:literal) => {
        pub fn $get(&self) -> $ty {
            self.$get.borrow().clone()
        }
        pub fn $set(&self, value: $ty) {
            ObservableObject::set_property(self, &self.$get, value, $name);
        }
    };
}
impl RibbonScreenTip {
    pub fn with_title(title: Option<RibbonString>) -> Rc<Self> {
        Self::with_text(title, None, None)
    }
    pub fn with_description(
        title: Option<RibbonString>,
        description: Option<RibbonString>,
    ) -> Rc<Self> {
        Self::with_text(title, description, None)
    }
    pub fn new() -> Rc<Self> {
        Self::with_text(None, None, None)
    }
    pub fn with_text(
        title: Option<RibbonString>,
        description: Option<RibbonString>,
        shortcut: Option<RibbonString>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: ObservableObject::construct(),
            title: RefCell::new(title),
            description: RefCell::new(description),
            shortcut: RefCell::new(shortcut),
            help_text: RefCell::default(),
            disabled_reason: RefCell::default(),
            image: RefCell::default(),
            extended_description: RefCell::default(),
            extended_image: RefCell::default(),
        })
    }
    property!(title, set_title, Option<RibbonString>, "Title");
    property!(
        description,
        set_description,
        Option<RibbonString>,
        "Description"
    );
    property!(shortcut, set_shortcut, Option<RibbonString>, "Shortcut");
    property!(help_text, set_help_text, Option<RibbonString>, "HelpText");
    property!(
        disabled_reason,
        set_disabled_reason,
        Option<RibbonString>,
        "DisabledReason"
    );
    property!(image, set_image, Option<Rc<RibbonIcon>>, "Image");
    property!(
        extended_description,
        set_extended_description,
        Option<RibbonString>,
        "ExtendedDescription"
    );
    property!(
        extended_image,
        set_extended_image,
        Option<Rc<RibbonIcon>>,
        "ExtendedImage"
    );
    pub fn format_title(
        title: Option<&RibbonString>,
        shortcut: Option<&RibbonString>,
    ) -> RibbonString {
        let Some(shortcut) = shortcut.filter(|s| !s.as_utf16().iter().all(|&c| whitespace(c)))
        else {
            return title.cloned().unwrap_or_default();
        };
        let mut units = title.map_or_else(Vec::new, |s| s.as_utf16().to_vec());
        units.extend([0x20, 0x28]);
        units.extend_from_slice(shortcut.as_utf16());
        units.push(0x29);
        RibbonString::from_utf16(units)
    }
}
impl INotifyPropertyChanged for RibbonScreenTip {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonScreenTip {}
impl PartialEq for RibbonScreenTip {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
