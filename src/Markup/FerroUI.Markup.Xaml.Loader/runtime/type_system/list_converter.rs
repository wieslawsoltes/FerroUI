//! ``FerroUI.Collections.FerroListConverter`1``: the type converter that
//! creates a `FerroList<T>` from its text, synthesized for every element
//! type (metadata declares no generic types with members).
//!
//! As the converter of the managed original: the text is split at commas
//! (entries are neither trimmed nor dropped) and every entry is converted to
//! the element type with the conversions of `TypeUtilities.TryConvert` that
//! apply to text: the text itself for a string or an untyped element, the
//! member of that exact name for an enumeration, the invariant text form for
//! the primitive types of the runtime library. Anything else is the invalid
//! cast of the original (`Could not convert '<entry>' to <type>.`).

use std::rc::Rc;

use ferroui_base::metadata::{MarkupInvokeError, MarkupValue};
use ferroui_base::BoxedValue;
use xamlx::type_system::{IXamlMember, IXamlType};

use super::runtime_type::{RuntimeInvoker, RuntimeMembers, RuntimeType};
use super::runtime_type_system::{MemberBuilder, RuntimeTypeSystem};

pub(crate) const DEFINITION: &str = "FerroUI.Collections.FerroListConverter`1";
pub(crate) const LIST_DEFINITION: &str = "FerroUI.Collections.FerroList`1";

/// An instance of the converter: the element type it converts to.
#[derive(Clone)]
pub struct RuntimeListConverter {
    element_type: Rc<dyn IXamlType>,
}

impl RuntimeListConverter {
    pub fn element_type(&self) -> &Rc<dyn IXamlType> {
        &self.element_type
    }
}

impl PartialEq for RuntimeListConverter {
    fn eq(&self, other: &Self) -> bool {
        self.element_type.equals(&*other.element_type)
    }
}

fn failed(message: String) -> MarkupInvokeError {
    MarkupInvokeError::Failed(message)
}

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

/// `TypeUtilities.TryConvert(typeof(T), text, culture, out value)` for a text
/// value. `None` if the text cannot be converted.
pub(crate) fn convert_text(element_type: &Rc<dyn IXamlType>, text: &str) -> Option<MarkupValue> {
    if element_type.is("System", "String") || element_type.is("System", "Object") {
        return Some(boxed(text.to_string()));
    }
    if element_type.is_enum() {
        // `Enum.IsDefined(type, text)`: the exact name of a member.
        let markup = element_type.as_any().downcast_ref::<RuntimeType>().and_then(RuntimeType::markup)?;
        let member = markup.enum_members.iter().find(|member| member.name == text)?;
        return Some(Some((member.get)()));
    }
    if element_type.namespace().as_deref() != Some("System") {
        return None;
    }
    // `Convert.ChangeType(text, type, culture)`: white space around a number is allowed.
    let number = text.trim();
    macro_rules! parse {
        ($type_:ty) => {
            number.parse::<$type_>().ok().map(boxed)
        };
    }
    match element_type.name().as_str() {
        "Boolean" => match number.to_ascii_lowercase().as_str() {
            "true" => Some(boxed(true)),
            "false" => Some(boxed(false)),
            _ => None,
        },
        "Char" => {
            let mut characters = text.chars();
            match (characters.next(), characters.next()) {
                (Some(character), None) => Some(boxed(character)),
                _ => None,
            }
        }
        "SByte" => parse!(i8),
        "Byte" => parse!(u8),
        "Int16" => parse!(i16),
        "UInt16" => parse!(u16),
        "Int32" => parse!(i32),
        "UInt32" => parse!(u32),
        "Int64" => parse!(i64),
        "UInt64" => parse!(u64),
        "Single" => parse!(f32),
        "Double" => parse!(f64),
        _ => None,
    }
}

/// `ConvertFrom(context, culture, value)`: null for a value that is not
/// text, otherwise a new list with the converted entries.
fn convert_from(
    system: &RuntimeTypeSystem,
    element_type: &Rc<dyn IXamlType>,
    value: &MarkupValue,
) -> Result<MarkupValue, MarkupInvokeError> {
    let Some(text) = value.as_ref().and_then(|v| v.downcast_ref::<String>()) else {
        return Ok(None);
    };
    use xamlx::type_system::IXamlTypeSystem;
    let list_type = system
        .find_type(LIST_DEFINITION)
        .and_then(|definition| definition.make_generic_type(std::slice::from_ref(element_type)).ok())
        .ok_or_else(|| {
            failed(format!("FerroList<{}> has no markup metadata: the list cannot be created", element_type.full_name()))
        })?;
    let runtime = list_type.as_any().downcast_ref::<RuntimeType>();
    let constructor = runtime
        .and_then(|t| t.runtime_constructors().into_iter().find(|c| xamlx::type_system::IXamlConstructor::parameters(&**c).is_empty()))
        .ok_or_else(|| failed(format!("{} has no parameterless constructor", list_type.full_name())))?;
    let add = runtime
        .and_then(|t| {
            t.runtime_methods().into_iter().find(|m| {
                m.name() == "Add" && !xamlx::type_system::IXamlMethod::is_static(&**m) && xamlx::type_system::IXamlMethod::parameters(&**m).len() == 1
            })
        })
        .ok_or_else(|| failed(format!("{} has no Add method", list_type.full_name())))?;
    let list = constructor.invoke(&[])?;
    for entry in text.split(',') {
        let converted = convert_text(element_type, entry)
            .ok_or_else(|| failed(format!("Could not convert '{entry}' to {}.", element_type.full_name())))?;
        add.invoke(&[list.clone(), converted])?;
    }
    Ok(list)
}

/// The members of the instantiation of the converter for `element_type`.
pub(crate) fn members(builder: &mut MemberBuilder, element_type: Rc<dyn IXamlType>) -> RuntimeMembers {
    builder.base("System.ComponentModel.TypeConverter");
    let constructed = element_type.clone();
    builder.constructor(
        Vec::new(),
        RuntimeInvoker::Dynamic(Rc::new(move |_| Ok(boxed(RuntimeListConverter { element_type: constructed.clone() })))),
    );
    let system = Rc::downgrade(&builder.system);
    let object = builder.t("System.Object");
    builder.method(
        "ConvertFrom",
        false,
        object.clone(),
        vec![
            builder.t("System.ComponentModel.ITypeDescriptorContext"),
            builder.t("System.Globalization.CultureInfo"),
            object,
        ],
        RuntimeInvoker::Dynamic(Rc::new(move |arguments: &[MarkupValue]| {
            // The instance, the context, the culture, the value.
            let value = arguments.get(3).ok_or(MarkupInvokeError::ArgumentCount { expected: 4, actual: arguments.len() })?;
            let system = system.upgrade().ok_or_else(|| failed("The type system has been dropped".to_string()))?;
            convert_from(&system, &element_type, value)
        })),
    );
    std::mem::take(&mut builder.members)
}
