use crate::data::BindingPriority;

/// The kind of source a value frame comes from, used to order frames of the
/// same binding priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrameType {
    Style = 0,
    TemplatedParentTheme = 1,
    Theme = 2,
}

/// The total ordering of value frames: binding priority first, then frame
/// type. Lower values take precedence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FramePriority(i8);

impl FramePriority {
    pub fn new(priority: BindingPriority, type_: FrameType) -> Self {
        debug_assert!(priority != BindingPriority::LocalValue);
        let p = priority as i32;
        let p = if p > 0 { p } else { p + 1 };
        FramePriority((p * 3 + type_ as i32) as i8)
    }

    pub fn to_binding_priority(self) -> BindingPriority {
        match self.0 / 3 {
            0 => BindingPriority::Animation,
            1 => BindingPriority::StyleTrigger,
            2 => BindingPriority::Template,
            _ => BindingPriority::Style,
        }
    }

    pub fn is_type(self, type_: FrameType) -> bool {
        (self.0 % 3) as i32 == type_ as i32
    }
}
