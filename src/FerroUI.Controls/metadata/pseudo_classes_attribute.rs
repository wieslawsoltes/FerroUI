/// Declares the pseudoclasses a control sets on itself.
///
/// Classes expose their pseudoclasses through a `pseudo_classes_metadata()`
/// function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PseudoClassesAttribute {
    /// The names of the pseudoclasses, including the leading colon.
    pub pseudo_classes: &'static [&'static str],
}

impl PseudoClassesAttribute {
    pub const fn new(pseudo_classes: &'static [&'static str]) -> Self {
        Self { pseudo_classes }
    }
}
