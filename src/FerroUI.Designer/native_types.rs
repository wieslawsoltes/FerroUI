//! Resolve authored XML type identities without creating application objects.
use ferroui_base::TypeInfo;
use ferroui_base::metadata::MarkupAssembly;

/// Resolve the same registered XML namespace mappings used by the runtime loader.
/// A short-name search is insufficient: an application's Button must not make the
/// framework Button ambiguous, and libraries can declare their own XML namespaces.
/// Native classes need not declare their own MarkupType to inherit registered properties.
pub(crate) fn registered_xml_type(uri: &str, name: &str) -> Option<&'static TypeInfo> {
    if let Some(namespace) = uri
        .strip_prefix("using:")
        .or_else(|| uri.strip_prefix("clr-namespace:"))
    {
        let mut parts = namespace.split(';');
        let type_ = TypeInfo::find(parts.next()?, name)?;
        for part in parts {
            let assembly = part.strip_prefix("assembly=")?;
            if MarkupAssembly::of_module(type_.module_path())?.name != assembly {
                return None;
            }
        }
        return Some(type_);
    }
    let mut resolved: Option<&'static TypeInfo> = None;
    for definition in MarkupAssembly::registered_assemblies()
        .into_iter()
        .flat_map(|assembly| assembly.xmlns_definitions)
        .filter(|definition| definition.xml_namespace == uri)
    {
        if let Some(candidate) = TypeInfo::find(definition.namespace, name) {
            if resolved.is_some_and(|previous| !std::ptr::eq(previous, candidate)) {
                return None;
            }
            resolved = Some(candidate);
        }
    }
    resolved
}
