//! The short name of a type (`Type.Name`).

/// The name of a type without its module path, as `Type.Name` gives it: `ferroui_splat::NullLogger`
/// gives `NullLogger`, `alloc::vec::Vec<i32>` gives `Vec<i32>`.
pub fn short_type_name(type_name: &str) -> &str {
    // The path ends before the first generic argument list.
    let path_end = type_name.find('<').unwrap_or(type_name.len());
    match type_name[..path_end].rfind("::") {
        Some(index) => &type_name[index + 2..],
        None => type_name,
    }
}
