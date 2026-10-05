//! A compile-time value parser backed by the run-time type system: text is
//! converted by the parse function the target type declares in its markup
//! metadata, so that the loader needs no knowledge of the types of the
//! controls library.

use std::rc::Rc;

use ferroui_base::metadata::MarkupValue;
use ferroui_base::BoxedValue;
use xamlx::type_system::{IXamlMember, IXamlType};

use crate::compiler_extensions::ast_nodes::{FerroXamlIlGridLength, FerroXamlIlGridUnitType};
use crate::compiler_extensions::{IXamlCompileTimeValueParser, XamlCompileTimeValue};

use super::type_system::{RuntimeMethod, RuntimeType};

/// Parses the compile-time values of framework types with the parse
/// function of their markup metadata. Register it with the
/// `XamlCompileTimeValueParsers` extra of a configuration.
///
/// A parsed value is taken apart through the properties the type projects
/// (`Value` and `GridUnitType` for a grid length). A type that declares a
/// parse function but not these properties is validated only: text the type
/// rejects is an error, accepted text is left to the built-in grammar.
pub struct RuntimeCompileTimeValueParser;

fn property(type_: &RuntimeType, instance: &MarkupValue, name: &str) -> Option<BoxedValue> {
    let getter: Rc<RuntimeMethod> =
        type_.runtime_methods().into_iter().find(|m| m.name() == format!("get_{name}"))?;
    getter.invoke(std::slice::from_ref(instance)).ok().flatten()
}

impl IXamlCompileTimeValueParser for RuntimeCompileTimeValueParser {
    fn try_parse(&self, type_: &Rc<dyn IXamlType>, text: &str) -> Option<Result<XamlCompileTimeValue, String>> {
        let runtime = type_.as_any().downcast_ref::<RuntimeType>()?;
        let parse = runtime.markup()?.parse?;
        let text_value: BoxedValue = Rc::new(text.to_string());
        let parsed = match parse(&[Some(text_value)]) {
            Ok(parsed) => parsed,
            Err(error) => return Some(Err(error.to_string())),
        };
        if !type_.is("FerroUI.Controls", "GridLength") {
            return None;
        }
        Some(Ok(XamlCompileTimeValue::GridLength(grid_length(runtime, &parsed)?)))
    }
}

/// Takes a parsed grid length apart through the `Value` and `GridUnitType`
/// properties its type projects. `None` if the type projects neither.
pub(crate) fn grid_length(type_: &RuntimeType, parsed: &MarkupValue) -> Option<FerroXamlIlGridLength> {
    let value = *property(type_, parsed, "Value")?.downcast_ref::<f64>()?;
    let unit = property(type_, parsed, "GridUnitType")?;
    let system = type_.system.upgrade()?;
    let unit_markup = system.runtime_type_of(&unit).as_any().downcast_ref::<RuntimeType>()?.markup()?;
    let member = unit_markup.enum_members.iter().find(|member| (member.get)().value_eq(&*unit))?;
    let grid_unit_type = match member.value {
        0 => FerroXamlIlGridUnitType::Auto,
        1 => FerroXamlIlGridUnitType::Pixel,
        2 => FerroXamlIlGridUnitType::Star,
        _ => return None,
    };
    Some(FerroXamlIlGridLength { value, grid_unit_type })
}
