/// Describes how to interpret the alpha component of a pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AlphaFormat {
    /// All pixels have their alpha premultiplied in their color components.
    #[default]
    Premul,
    /// All pixels have their color components stored without any regard to
    /// the alpha, e.g. they are unpremultiplied.
    Unpremul,
    /// All pixels are stored as opaque.
    Opaque,
}
