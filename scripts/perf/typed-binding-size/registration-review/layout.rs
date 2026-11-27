use ferroui_base::metadata::*;
use std::mem::{offset_of as off, size_of};
macro_rules! layout {
    ($t:ident, $($field:ident),+) => {{
        println!("{} size {}", stringify!($t), size_of::<$t>());
        $(println!("{} {} {}", stringify!($t), stringify!($field), off!($t, $field));)+
    }}
}
fn main() {
    layout!(
        MarkupType,
        name,
        properties,
        static_properties,
        methods,
        fields,
        events,
        constructors,
        indexers,
        enum_members,
        type_info,
        handles
    );
    layout!(
        MarkupProperty,
        name,
        get,
        set,
        typed_path_element,
        type_,
        value_markup,
        attributes
    );
    layout!(MarkupEnumMember, name, value, rust_variant);
    layout!(
        MarkupMethod,
        name,
        invoke,
        parameters,
        return_type,
        is_static
    );
    layout!(MarkupConstructor, invoke);
    layout!(MarkupField, name, get);
    layout!(MarkupEvent, name, add);
}
