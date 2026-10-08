/// Defines the model used to represent colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ColorModel {
    /// Color is represented by hue, saturation, value and alpha components.
    Hsva = 0,

    /// Color is represented by red, green, blue and alpha components.
    Rgba = 1,
}
