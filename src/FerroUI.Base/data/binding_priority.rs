/// The priority of a binding or of a value set on a property.
///
/// Lower values take precedence over higher ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum BindingPriority {
    /// A value that comes from an animation.
    Animation = -1,
    /// A local value.
    LocalValue = 0,
    /// A triggered style value: a style whose selector has conditions such as
    /// pseudo-classes or property matches.
    StyleTrigger = 1,
    /// A template binding or a value set by a template.
    Template = 2,
    /// A style value.
    Style = 3,
    /// The value is inherited from an ancestor element.
    Inherited = 4,
    /// The value is uninitialized.
    Unset = i32::MAX,
}

impl Default for BindingPriority {
    fn default() -> Self {
        BindingPriority::LocalValue
    }
}
