//! Macros of the fixture that declare through the declaration macros.

use ferroui_base::animation::TransitionBase;

/// Declares a transition class: one rule of an identifier and a type, which the scanner
/// expands.
macro_rules! transition_class {
    ($(#[$meta:meta])* $name:ident : $value:ty) => {
        $(#[$meta])*
        #[repr(C)]
        pub struct $name {
            base: TransitionBase,
        }

        $crate::ferro_class!($name: TransitionBase);
        $crate::ferro_class_info!($name { new: $name::new });
    };
}

/// Declares several classes: a repetition, which the scanner does not expand.
macro_rules! many_classes {
    ($($name:ident),*) => {
        $(ferroui_base::ferro_class!($name: TransitionBase);)*
    };
}

transition_class!(
    /// Transitions of numbers.
    DoubleTransition: f64
);

many_classes!(First, Second);
