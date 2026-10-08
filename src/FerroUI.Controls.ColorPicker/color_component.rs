/// Defines a specific component within a color model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ColorComponent {
    /// Represents the alpha component.
    Alpha = 0,

    /// Represents the first color component which is Red when RGB or Hue when HSV.
    Component1 = 1,

    /// Represents the second color component which is Green when RGB or Saturation when HSV.
    Component2 = 2,

    /// Represents the third color component which is Blue when RGB or Value when HSV.
    Component3 = 3,
}
