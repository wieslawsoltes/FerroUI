// Test-only reader for public compiler facts emitted by the original reference.
use super::*;
use serde_json::Value;
pub(crate) fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap().to_owned()
}
fn flag(v: &Value, key: &str) -> bool {
    v[key].as_bool().unwrap()
}
pub(crate) fn id(v: &Value) -> Option<usize> {
    v.as_u64().map(|v| v as usize)
}
fn ids(v: &Value, key: &str) -> Vec<usize> {
    v[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| id(v).unwrap())
        .collect()
}
fn optional(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().map(str::to_owned)
}
pub(crate) fn compilation(v: &Value) -> Compilation {
    let types = v["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| TypeSymbol {
            expression: text(t, "expression"),
            display_name: text(t, "display_name"),
            metadata_name: text(t, "metadata_name"),
            namespace: text(t, "ns"),
            kind: match t["kind"].as_str().unwrap() {
                "Class" => TypeKind::Class,
                "Struct" => TypeKind::Struct,
                "Enum" => TypeKind::Enum,
                "Interface" => TypeKind::Interface,
                "Delegate" => TypeKind::Delegate,
                "Array" => TypeKind::Array,
                "Pointer" => TypeKind::Pointer,
                "TypeParameter" => TypeKind::TypeParameter,
                "Error" | "Unknown" => TypeKind::OtherNamed,
                "Dynamic" | "FunctionPointer" => TypeKind::OtherNonNamed,
                v => panic!("source type kind {v}"),
            },
            special: flag(t, "special"),
            is_abstract: flag(t, "is_abstract"),
            accessibility: match t["accessibility"].as_str().unwrap() {
                "Public" => Accessibility::Public,
                "Internal" => Accessibility::Internal,
                "ProtectedOrInternal" => Accessibility::ProtectedOrInternal,
                "Private" => Accessibility::Private,
                "Protected" => Accessibility::Protected,
                "ProtectedAndInternal" => Accessibility::ProtectedAndInternal,
                "NotApplicable" => Accessibility::Other,
                v => panic!("source accessibility {v}"),
            },
            assembly: id(&t["assembly"]),
            containing_type: id(&t["containing_type"]),
            base_type: id(&t["base_type"]),
            all_interfaces: ids(t, "all_interfaces"),
            type_arguments: ids(t, "type_arguments"),
            arity: id(&t["arity"]).unwrap(),
            is_unbound_generic: flag(t, "is_unbound_generic"),
            tuple_underlying_type: id(&t["tuple_underlying_type"]),
            element_type: id(&t["element_type"]),
            array_rank: id(&t["array_rank"]).unwrap(),
            properties: ids(t, "properties"),
            location: optional(t, "location"),
        })
        .collect();
    Compilation {
        activated: flag(v, "activated"),
        registrations: v["registrations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| Registration {
                type_id: id(&r["type_id"]),
                valid_type_constant: flag(r, "valid_type_constant"),
                display_name: text(r, "display_name"),
                location: optional(r, "location"),
            })
            .collect(),
        source_types: ids(v, "source_types"),
        required_symbols: v["required_symbols"]
            .as_array()
            .unwrap()
            .iter()
            .map(id)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        occupied_type_names: v["occupied_type_names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect(),
        assemblies: v["assemblies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| AssemblySymbol {
                name: text(a, "name"),
                version: text(a, "version"),
                culture: optional(a, "culture"),
                public_key_token: ids(a, "public_key_token")
                    .into_iter()
                    .map(|v| u8::try_from(v).unwrap())
                    .collect(),
                grants_internal_access: flag(a, "grants_internal_access"),
            })
            .collect(),
        types,
        properties: v["properties"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| PropertySymbol {
                name: text(p, "name"),
                type_id: id(&p["type_id"]).unwrap(),
                is_public: flag(p, "is_public"),
                is_static: flag(p, "is_static"),
                ignore_data_member: flag(p, "ignore_data_member"),
                overridden_property: id(&p["overridden_property"]),
            })
            .collect(),
        context_roots: ContextRoots {
            object: text(&v["context_roots"], "obj"),
            dockable_list: text(&v["context_roots"], "dockable_list"),
            window_list: text(&v["context_roots"], "window_list"),
        },
    }
}
