use crate::templates::IDataTemplate;
use ferroui_base::{FerroObject, FerroPropertyRegistry};
use std::rc::Rc;

const MEMBERS: &[&str] = &[
    "ItemTemplate",
    "SelectionBoxItemTemplate",
    "MenuItemTemplate",
];

/// Reads the effective data-template properties registered on this control.
pub fn data_template_slots(
    object: &FerroObject,
) -> impl Iterator<Item = (&'static str, Option<Rc<dyn IDataTemplate>>)> + '_ {
    MEMBERS.iter().filter_map(move |&member| {
        let property =
            FerroPropertyRegistry::instance().find_registered(object.get_type(), member)?;
        let value = object.get_value_untyped(property);
        Some((
            member,
            value
                .downcast_ref::<Option<Rc<dyn IDataTemplate>>>()?
                .clone(),
        ))
    })
}
