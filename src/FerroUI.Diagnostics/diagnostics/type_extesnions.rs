//! Port of `Diagnostics/TypeExtesnions.cs` (the misspelt name is upstream's).
//!
//! A `System.Type` of the original is a [`ValueType`] here (any type: the value type of a
//! property, the type of a value) or a [`TypeInfo`] (a class of the object model); see
//! `docs/porting/xaml.md`, decision 16. The name of a [`ValueType`] is read from its dotted full
//! name ([`ValueTypes::type_full_name`]), which states what the original states about a type: the
//! name of the runtime library type a primitive mirrors, the full name of a class or of a type
//! with markup metadata, ``System.Nullable`1[..]`` for the nullable form of a value type, and the
//! Rust type name for a type nothing is known about.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::TypeInfo;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;

ferroui_base::ferro_thread_local! {
    // C# `ConditionalWeakTable<Type, string> s_getTypeNameCache`: the types of the port are never
    // unloaded, so the names are kept by the identity of the type.
    static GET_TYPE_NAME_CACHE: RefCell<HashMap<TypeId, String>> = RefCell::new(HashMap::new());
}

/// Extension methods of types.
pub trait TypeExtesnions {
    /// The short name of the type as DevTools shows it: `Double`, `Int32?` for a nullable value
    /// type, `List<String>` for a generic type.
    fn get_type_name(&self) -> String;
}

impl TypeExtesnions for ValueType {
    fn get_type_name(&self) -> String {
        if let Some(name) = GET_TYPE_NAME_CACHE.with(|cache| cache.borrow().get(&self.id()).cloned()) {
            return name;
        }

        let name = match ValueTypes::nullable_inner(*self) {
            // `Nullable.GetUnderlyingType(type) is Type nullable`: the nullable form of a value type.
            Some(nullable) if !is_reference_type(nullable) => {
                format!("{}?", get_type_name_of_full_name(&ValueTypes::type_full_name(nullable)))
            }
            // A nullable reference is the reference type itself upstream (`IBrush?` is `IBrush`).
            Some(reference) => reference.get_type_name(),
            None => get_type_name_of_full_name(&ValueTypes::type_full_name(*self)),
        };
        GET_TYPE_NAME_CACHE.with(|cache| {
            cache.borrow_mut().insert(self.id(), name.clone());
        });
        name
    }
}

impl TypeExtesnions for TypeInfo {
    fn get_type_name(&self) -> String {
        // A class of the object model is neither nullable nor generic: its name is `type.Name`.
        self.name().to_string()
    }
}

/// Whether `type_` is a reference type of the original (a class, an interface, text or any
/// value), whose nullable form is the type itself, rather than a value type.
fn is_reference_type(type_: ValueType) -> bool {
    type_.is_object()
        || type_.is_string()
        || ValueTypes::class_of(type_).is_some()
        || ValueTypes::is_reference(type_.id())
        || type_.name().starts_with("alloc::rc::Rc<")
}

/// The name [`TypeExtesnions::get_type_name`] gives the type with the dotted full name
/// `full_name`:
///
/// * ``System.Nullable`1[T]`` (`Nullable.GetUnderlyingType(type) is Type nullable`) is the
///   name of `T` followed by `?`;
/// * a generic type (``Name`N[A,B]``, `type.IsGenericType`) is its name without the arity,
///   followed by the names of its arguments in angle brackets (`Name<A,B>`);
/// * any other type is its name without the namespace (`type.Name`).
///
/// A Rust type name (a type nothing is known about) keeps the last segment of each path, and
/// `Option<T>` is shown as the nullable form of `T`.
#[doc(hidden)]
pub fn get_type_name_of_full_name(full_name: &str) -> String {
    let full_name = unbracket(full_name.trim());

    if let Some(underlying) = full_name.strip_prefix("System.Nullable`1[").and_then(|rest| rest.strip_suffix(']')) {
        return format!("{}?", simple_name(unbracket(underlying)));
    }

    if let Some(tick) = full_name.find('`') {
        let definition = simple_name(&full_name[..tick]);
        let arguments = full_name[tick..]
            .find('[')
            .map(|open| tick + open)
            .filter(|_| full_name.ends_with(']'))
            .map(|open| &full_name[open + 1..full_name.len() - 1]);
        return match arguments {
            Some(arguments) => {
                let arguments: Vec<String> =
                    split_top_level(arguments).into_iter().map(get_type_name_of_full_name).collect();
                format!("{definition}<{}>", arguments.join(","))
            }
            None => definition.to_string(),
        };
    }

    if full_name.contains("::") || full_name.contains('<') {
        return rust_type_name(full_name);
    }

    simple_name(full_name).to_string()
}

/// `type.Name`: the name without its namespace (and without the type that declares a nested
/// type).
fn simple_name(full_name: &str) -> &str {
    let start = full_name.rfind(['.', '+']).map_or(0, |index| index + 1);
    &full_name[start..]
}

/// An assembly-qualified argument of a generic type (`[System.Int32, System.Private.CoreLib]`)
/// is the name of its type.
fn unbracket(name: &str) -> &str {
    match name.strip_prefix('[').and_then(|inner| inner.strip_suffix(']')) {
        Some(inner) => inner.split(',').next().unwrap_or(inner).trim(),
        None => name,
    }
}

/// The arguments of a generic type, separated by the commas that are not inside the brackets of
/// an argument.
fn split_top_level(arguments: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (index, c) in arguments.char_indices() {
        match c {
            '[' | '<' | '(' => depth += 1,
            ']' | '>' | ')' => depth -= 1,
            ',' if depth == 0 => {
                result.push(arguments[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    let last = arguments[start..].trim();
    if !last.is_empty() {
        result.push(last);
    }
    result
}

/// The short form of a Rust type name: each path is reduced to its last segment
/// (`alloc::rc::Rc<dyn ferroui_base::media::IBrush>` is `Rc<dyn IBrush>`), the arguments are
/// separated by `,` as upstream joins them, and `Option<T>` is `T?`.
fn rust_type_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut path = String::new();

    fn flush(path: &mut String, result: &mut String) {
        if !path.is_empty() {
            let last = path.rsplit("::").next().unwrap_or(path);
            result.push_str(last);
            path.clear();
        }
    }

    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_alphanumeric() || c == '_' || c == ':' {
            path.push(c);
            continue;
        }
        flush(&mut path, &mut result);
        if c == ',' {
            result.push(',');
            while chars.peek() == Some(&' ') {
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    flush(&mut path, &mut result);

    match result.strip_prefix("Option<").and_then(|inner| inner.strip_suffix('>')) {
        Some(inner) if split_top_level(inner).len() == 1 => format!("{inner}?"),
        _ => result,
    }
}
