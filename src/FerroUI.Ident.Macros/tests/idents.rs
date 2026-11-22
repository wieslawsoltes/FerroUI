//! The forms of `ferro_idents!` that the macros of FerroUI use.

#![allow(non_camel_case_types, non_snake_case)]

use ferroui_ident_macros::ferro_idents;

macro_rules! vtable_of {
    ($name:ident) => {
        ferro_idents! {
            #[derive(Default)]
            struct [<$name VTable>] {
                slots: u32,
            }

            fn [<new_ $name:camel _table>]() -> [<$name VTable>] {
                [<$name VTable>] { slots: 3 }
            }
        }
    };
}

vtable_of!(button);

#[test]
fn joins_type_and_function_names() {
    assert_eq!(new_Button_table().slots, 3);
    assert_eq!(buttonVTable::default().slots, 0);
}

macro_rules! property {
    ($field:ident) => {
        ferro_idents! {
            fn [<get_ $field>]() -> &'static str {
                ::core::stringify!([<$field:camel>])
            }
        }
    };
}

property!(search_term);
property!(r#type);

#[test]
fn camel_modifier_and_raw_identifiers() {
    assert_eq!(get_search_term(), "SearchTerm");
    assert_eq!(get_type(), "Type");
}

struct Holder;

impl Holder {
    fn set_value(&self, value: u32) -> u32 {
        value + 1
    }
}

macro_rules! call_setter {
    ($source:expr, $last:ident, $value:expr) => {
        ferro_idents! { $source.[<set_ $last>]($value) }
    };
}

#[test]
fn joins_in_expression_position() {
    assert_eq!(call_setter!(Holder, value, 41), 42);
}

mod nested {
    pub struct Point;

    impl Point {
        pub const ORIGIN: u32 = 7;

        pub fn make() -> u32 {
            Self::ORIGIN
        }
    }
}

// A `ty` or `path` fragment reaches the macro in an invisible group: as a plain path it must
// still work before a `::`, in a type position and as a bound.
macro_rules! through_fragments {
    ($name:ident, $type_:ty, $path:path) => {
        ferro_idents! {
            fn [<$name _from_type>]() -> u32 {
                <$type_>::make()
            }

            fn [<$name _from_path>]() -> u32 {
                $path::ORIGIN
            }

            fn [<$name _size>]<T: ::core::marker::Sized>(value: $type_, _other: T) -> usize {
                let _ = value;
                ::core::mem::size_of::<$type_>()
            }
        }
    };
}

through_fragments!(point, nested::Point, nested::Point);

#[test]
fn fragments_keep_working() {
    assert_eq!(point_from_type(), 7);
    assert_eq!(point_from_path(), 7);
    assert_eq!(point_size(nested::Point, 0u8), 0);
}

macro_rules! documented {
    ($(#[$meta:meta])* $name:ident) => {
        ferro_idents! {
            $(#[$meta])*
            #[doc = concat!("Generated for `", stringify!($name), "`.")]
            pub fn [<documented_ $name>]() -> bool {
                true
            }
        }
    };
}

documented!(
    /// A doc comment passed through a `meta` fragment.
    #[inline]
    thing
);

#[test]
fn attributes_pass_through() {
    assert!(documented_thing());
}

// A bracket that is not a join stays an array or an index.
#[test]
fn other_brackets_are_left_alone() {
    let values: [u32; 2] = ferro_idents! { [<u32 as Default>::default(), 5] };
    assert_eq!(values, [0, 5]);
}
