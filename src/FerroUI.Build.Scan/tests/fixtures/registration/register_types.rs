//! The type table of the fixture: what its registration function makes known.

use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped};
use ferroui_base::{StaticType, TypeInfo};

const NAMESPACES: &[(&str, &str)] = &[("registration", "Registration")];

pub static ASSEMBLY: MarkupAssembly =
    MarkupAssembly { name: "Registration", crate_name: "registration", xmlns_definitions: &[], xmlns_prefixes: &[], metadata: &[] };

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// The classes the crate registers: `Hidden` is left out, and `Missing` is no class of it.
const TYPES: &[&TypeInfo] = types![crate::panel::Panel, crate::Slot, crate::panel::Deep, crate::Missing];

pub fn register_types() {
    use crate::panel::{PanelCollection, PanelList as Lists, Wrapper};
    use ferroui_base::data::core::ValueTypes;

    TypeInfo::register_all(TYPES);
    // One more handle of the contract, and of the collection.
    let contract = <dyn crate::panel::IPanel as MarkupTyped>::MARKUP;
    MarkupType::register_handle::<Wrapper>(contract);
    MarkupType::register_handle::<Option<Wrapper>>(<PanelCollection as MarkupTyped>::MARKUP);
    // A registration the scanner does not read: the type is computed.
    MarkupType::register_handle::<Wrapper>(lookup());
    // The collection is the list it derives from.
    ValueTypes::register_cast::<PanelCollection, Lists>(|collection| collection.list());
    // A cast whose types are left to inference is not read.
    ValueTypes::register_cast::<Wrapper, _>(unwrap);
}

/// What the crate registers with the untyped value conversions next to its casts.
fn register_value_types() {
    use crate::panel::{Panel, PanelCollection, Wrapper};
    use ferroui_base::data::core::ValueTypes;
    use std::rc::Rc;

    ValueTypes::register_nullable::<PanelCollection>();
    ValueTypes::register_reference::<Wrapper>();
    // Handed to the function that calls it.
    ValueTypes::register_deferred(ValueTypes::register_element_ref::<Panel>);
    // A table of casts written with a macro of the function, by a macro of the crate that
    // is handed its name.
    macro_rules! assignable {
        ($concrete:ty => $handle:ty) => {
            assignable!($concrete => $handle, |value| value.clone());
        };
        ($concrete:ty => $handle:ty, $cast:expr) => {{
            ValueTypes::register_cast::<Rc<$concrete>, $handle>($cast);
            ValueTypes::register_nullable::<$handle>();
        }};
    }
    for_each_cast!(assignable);
    // A macro whose rule repeats a part of its input registers once for each round.
    macro_rules! references {
        ($($type_:ty),* $(,)?) => {
            $(ValueTypes::register_reference::<$type_>();)*
        };
    }
    references![PanelCollection, crate::panel::Deep,];
    // A macro whose rule takes a fragment the scanner does not match is not expanded: what
    // it registers is not read.
    macro_rules! guarded {
        ($type_:ty, $guard:block) => {
            if $guard {
                ValueTypes::register_reference::<$type_>();
            }
        };
    }
    guarded!(Panel, { true });
}

macro_rules! for_each_cast {
    ($apply:ident) => {
        $apply!(Wrapper => Rc<dyn crate::panel::IPanel>);
        $apply!(PanelCollection => Rc<Wrapper>, |collection| collection.wrapper());
    };
}

/// A function written as a closure in the value of a constant.
pub const VALUE_TYPES: fn() = || {
    ferroui_base::data::core::ValueTypes::register_upcast::<crate::panel::Deep, crate::panel::Panel>();
};
