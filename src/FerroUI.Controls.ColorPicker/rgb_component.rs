/// Defines a specific component in the RGB color model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RgbComponent {
    /// The Alpha component.
    ///
    /// Also see: [`Color::a`](ferroui_base::media::Color::a)
    Alpha = 0,

    /// The Red component.
    ///
    /// Also see: [`Color::r`](ferroui_base::media::Color::r)
    Red = 1,

    /// The Green component.
    ///
    /// Also see: [`Color::g`](ferroui_base::media::Color::g)
    Green = 2,

    /// The Blue component.
    ///
    /// Also see: [`Color::b`](ferroui_base::media::Color::b)
    Blue = 3,
}
