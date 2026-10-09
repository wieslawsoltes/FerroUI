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
