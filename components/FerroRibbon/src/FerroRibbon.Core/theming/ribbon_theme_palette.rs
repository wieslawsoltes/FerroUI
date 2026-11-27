use super::RibbonColor;
use crate::RibbonString;
use std::rc::Rc;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RibbonThemePalette {
    pub name: Option<RibbonString>,
    pub accent: RibbonColor,
    pub accent_dark: RibbonColor,
}
ferroui_system::ferro_thread_local! {static PRESETS:Rc<[Rc<RibbonThemePalette>]>=super::palette_data::THEMES.iter().map(|&(_,name,a,b)|RibbonThemePalette::new(Some(name.into()),RibbonColor::parse(Some(&a.into())).expect("source theme color"),RibbonColor::parse(Some(&b.into())).expect("source theme color"))).collect::<Vec<_>>().into();}
impl RibbonThemePalette {
    pub fn new(
        name: Option<RibbonString>,
        accent: RibbonColor,
        accent_dark: RibbonColor,
    ) -> Rc<Self> {
        Rc::new(Self {
            name,
            accent,
            accent_dark,
        })
    }
    pub fn presets() -> Rc<[Rc<Self>]> {
        PRESETS.with(Clone::clone)
    }
    pub fn word() -> Rc<Self> {
        Self::presets()[0].clone()
    }
    pub fn excel() -> Rc<Self> {
        Self::presets()[1].clone()
    }
    pub fn power_point() -> Rc<Self> {
        Self::presets()[2].clone()
    }
    pub fn outlook() -> Rc<Self> {
        Self::presets()[3].clone()
    }
    pub fn one_note() -> Rc<Self> {
        Self::presets()[4].clone()
    }
    pub fn access() -> Rc<Self> {
        Self::presets()[5].clone()
    }
    pub fn visio() -> Rc<Self> {
        Self::presets()[6].clone()
    }
    pub fn project() -> Rc<Self> {
        Self::presets()[7].clone()
    }
    pub fn publisher() -> Rc<Self> {
        Self::presets()[8].clone()
    }
    pub fn teams() -> Rc<Self> {
        Self::presets()[9].clone()
    }
    pub fn graphite() -> Rc<Self> {
        Self::presets()[10].clone()
    }
    pub fn cad() -> Rc<Self> {
        Self::presets()[11].clone()
    }
    pub fn from_accent(accent: RibbonColor) -> Rc<Self> {
        Self::from_accent_named(accent, Some("Custom".into()))
    }
    pub fn from_accent_named(accent: RibbonColor, name: Option<RibbonString>) -> Rc<Self> {
        Self::new(
            name,
            accent,
            if accent.lightness() < 0.55 {
                accent.lighten(0.45)
            } else {
                accent
            },
        )
    }
    pub fn get_accent(&self, dark: bool) -> RibbonColor {
        if dark { self.accent_dark } else { self.accent }
    }
    pub fn get_accent_subtle(&self, dark: bool) -> RibbonColor {
        if dark {
            self.accent_dark.with_alpha(0x33)
        } else {
            self.accent.lighten(0.86)
        }
    }
    pub fn get_accent_subtle_strong(&self, dark: bool) -> RibbonColor {
        if dark {
            self.accent_dark.with_alpha(0x55)
        } else {
            self.accent.lighten(0.74)
        }
    }
    pub fn get_on_accent(&self, dark: bool) -> RibbonColor {
        if self.get_accent(dark).prefers_dark_foreground() {
            RibbonColor::BLACK
        } else {
            RibbonColor::WHITE
        }
    }
}
macro_rules! named_enum {
    ($name:ident,$first:ident,$second:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub struct $name(i32);
        ferroui_system::styled_value!($name);
        #[allow(non_upper_case_globals)]
        impl $name {
            pub const $first: Self = Self(0);
            pub const $second: Self = Self(1);
            pub const fn from_raw(value: i32) -> Self {
                Self(value)
            }
            pub const fn raw(self) -> i32 {
                self.0
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.0 {
                    0 => f.write_str(stringify!($first)),
                    1 => f.write_str(stringify!($second)),
                    other => write!(f, "{other}"),
                }
            }
        }
    };
}
named_enum!(RibbonChromeStyle, Colorful, Neutral);
named_enum!(RibbonThemeStyle, Office, Cad);
