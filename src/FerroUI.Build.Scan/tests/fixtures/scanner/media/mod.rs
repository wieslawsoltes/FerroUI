use ferroui_base::{ferro_markup_enum, ferro_markup_type};
use std::rc::Rc;

mod brush;

pub use brush::{Brush, IBrush};

#[derive(Clone, Copy, PartialEq)]
pub enum Dock {
    Left,
    Bottom = 4,
    Right,
    Top = -1,
}

ferro_markup_enum!(Dock { Left, Bottom, Right, Top });

pub enum Mode {
    DataContext,
    Self_,
}

ferro_markup_enum!(Mode { DataContext, Self = Self_ }, { namespace: "Fixture.Data", attributes: [Flagged] });

bitflags::bitflags! {
    /// How an event travels.
    #[derive(Clone, Copy, PartialEq)]
    pub struct Routes: u32 {
        const DIRECT = 0x01;
        const TUNNEL = 1 << 1;
        const BUBBLE = 1 << 2;
        const BOTH = Self::TUNNEL.bits() | Self::BUBBLE.bits();
        const ODD = compute();
    }
}

ferro_markup_enum!(flags Routes {
    Direct = Routes::DIRECT,
    Tunnel = Routes::TUNNEL,
    Both = Routes::BOTH,
    Odd = Routes::ODD,
});

#[derive(Clone, Copy, PartialEq)]
pub struct Thickness {
    pub left: f64,
}

ferro_markup_type!(struct Thickness {
    handles: [Thickness],
    parse: Thickness::parse,
    constructors: [
        (f64) => Thickness::uniform,
        try (f64, f64) => |left: f64, top: f64| Thickness::checked(left, top),
    ],
    properties: [
        Left: f64 { get: |thickness: &Thickness| thickness.left },
    ],
});

ferro_markup_type!(interface dyn IBrush as "IBrush" {
    handles: [Rc<dyn IBrush>, Option<Rc<dyn IBrush>>],
});
