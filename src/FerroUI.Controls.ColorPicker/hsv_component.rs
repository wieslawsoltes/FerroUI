// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

/// Defines a specific component in the HSV color model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum HsvComponent {
    /// The Alpha component.
    ///
    /// Also see: [`HsvColor::a`](ferroui_base::media::HsvColor::a)
    Alpha = 0,

    /// The Hue component.
    ///
    /// Also see: [`HsvColor::h`](ferroui_base::media::HsvColor::h)
    Hue = 1,

    /// The Saturation component.
    ///
    /// Also see: [`HsvColor::s`](ferroui_base::media::HsvColor::s)
    Saturation = 2,

    /// The Value component.
    ///
    /// Also see: [`HsvColor::v`](ferroui_base::media::HsvColor::v)
    Value = 3,
}
