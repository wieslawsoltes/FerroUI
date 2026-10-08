/// Defines a specific tab/page (subview) within the [`ColorView`](crate::ColorView).
///
/// This is indexed to match the default control template ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ColorViewTab {
    /// The color spectrum subview with a box/ring spectrum and sliders.
    Spectrum = 0,

    /// The color palette subview with a grid of selectable colors.
    Palette = 1,

    /// The components subview with sliders and numeric input boxes.
    Components = 2,
}
