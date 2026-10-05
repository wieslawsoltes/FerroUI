
/// Defines how a bar (such as a navigation bar or drawer header) is laid out
/// relative to page content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BarLayoutBehavior {
    /// The bar occupies its own layout space and content is placed below it.
    Inset = 0,

    /// The bar overlays the content area without consuming layout space.
    Overlay = 1,
}
