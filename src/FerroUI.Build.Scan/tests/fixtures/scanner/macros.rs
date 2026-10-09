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

/// Declares the metadata of a typed list: a macro the crate exports, which a crate built
/// on this one invokes. The scan of that crate expands the invocation with the rules the
/// model of this crate has, and `$crate` is the path of this crate there.
#[macro_export]
macro_rules! typed_list {
    ($visibility:vis $carrier:ident : $item:ty) => {
        $visibility struct $carrier;

        ::ferroui_base::ferro_markup_type!(class $carrier as "List`1" {
            namespace: "Fixture.Collections",
            handles: [Vec<$item>, Option<Vec<$item>>],
            this: Vec<$item>,
            generic: "List`1" [$item],
            properties: [
                Count: i32 { get: |list: &Vec<$item>| list.len() as i32 },
                Dock: Option<$crate::media::Dock> { get: |_: &Vec<$item>| None },
            ],
        });
    };
}
