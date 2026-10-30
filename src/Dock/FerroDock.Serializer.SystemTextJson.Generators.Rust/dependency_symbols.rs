// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_serializer_system_text_json_generators::{Compilation, CompilationError, TypeId};

/// Resolved import identities supplied to a native dependency-binding transaction.
#[derive(Debug)]
pub struct RustSemanticImportIds {
    /// Source table ID to imported/native ID, absent for facts outside the selected closure.
    pub source_types: Vec<Option<TypeId>>,
    /// Fresh native IDs, in the compiler host's actual native declaration order.
    pub native_types: Vec<TypeId>,
}

fn error(message: &str) -> CompilationError {
    CompilationError(message.into())
}

/// Imports a dependency closure and binds native replacements without importing their schemas.
///
/// `native_sources` identifies external symbols resolved to native declarations, in native
/// declaration order. Their headers and members are never inspected or copied. Native IDs
/// are allocated after the selected foreign types, independently of old source table slots.
/// The host supplies every additional foreign root needed by the native declarations;
/// required contracts are included automatically. All foreign type/property/assembly links
/// are remapped, including generic/array/tuple cycles and override links within the closure.
/// An override into a declaration outside that closure needs explicit compiler rebinding
/// and is rejected, rather than retaining an excluded native property's old schema.
///
/// `bind` receives a private working graph and its import map. It must construct the native
/// declarations in the reserved order, for example with `bind_semantic_declarations`.
/// Foreign facts may refer forward to those native IDs during construction; no dummy type
/// headers are inserted. The result is returned only after the final table size and every
/// symbol link validate. Errors never mutate the source. Activation, registrations, source
/// discovery and occupied names start empty and remain explicit host decisions. Context root
/// expressions are the caller-supplied dependency input, without target-language guessing.
pub fn import_semantic_dependencies(
    source: &Compilation,
    roots: &[TypeId],
    native_sources: &[TypeId],
    bind: impl FnOnce(&mut Compilation, &RustSemanticImportIds) -> Result<(), CompilationError>,
) -> Result<Compilation, CompilationError> {
    let mut native_index = vec![None; source.types.len()];
    for (index, id) in native_sources.iter().copied().enumerate() {
        let slot = native_index
            .get_mut(id)
            .ok_or_else(|| error("Missing native source symbol identity"))?;
        if slot.replace(index).is_some() {
            return Err(error("Duplicate native source symbol identity"));
        }
    }
    let mut selected_types = vec![false; source.types.len()];
    let mut selected_properties = vec![false; source.properties.len()];
    let mut pending: Vec<_> = roots
        .iter()
        .copied()
        .chain(source.required_symbols.into_iter().flatten())
        .collect();
    while let Some(id) = pending.pop() {
        let selected = selected_types
            .get_mut(id)
            .ok_or_else(|| error("Missing dependency type symbol"))?;
        if native_index[id].is_some() || *selected {
            continue;
        }
        *selected = true;
        let t = &source.types[id];
        pending.extend(t.all_interfaces.iter().chain(&t.type_arguments).copied());
        pending.extend(
            [
                t.base_type,
                t.containing_type,
                t.element_type,
                t.tuple_underlying_type,
            ]
            .into_iter()
            .flatten(),
        );
        for id in &t.properties {
            let selected = selected_properties
                .get_mut(*id)
                .ok_or_else(|| error("Missing dependency property symbol"))?;
            if !*selected {
                *selected = true;
                pending.push(source.properties[*id].type_id);
            }
        }
    }
    let mut types = vec![None; source.types.len()];
    for (new, old) in selected_types
        .iter()
        .enumerate()
        .filter(|(_, v)| **v)
        .map(|(i, _)| i)
        .enumerate()
    {
        types[old] = Some(new);
    }
    let first = selected_types.iter().filter(|v| **v).count();
    let end = first
        .checked_add(native_sources.len())
        .ok_or_else(|| error("Dependency type table capacity overflow"))?;
    let native_types: Vec<_> = (first..end).collect();
    for (old, index) in native_index.iter().enumerate() {
        if let Some(index) = index {
            types[old] = Some(native_types[*index]);
        }
    }
    let mut properties = vec![None; source.properties.len()];
    for (new, old) in selected_properties
        .iter()
        .enumerate()
        .filter(|(_, v)| **v)
        .map(|(i, _)| i)
        .enumerate()
    {
        properties[old] = Some(new);
    }
    let mut assemblies = vec![None; source.assemblies.len()];
    for (_, t) in source
        .types
        .iter()
        .enumerate()
        .filter(|(i, _)| selected_types[*i])
    {
        if let Some(id) = t.assembly {
            *assemblies
                .get_mut(id)
                .ok_or_else(|| error("Missing dependency assembly identity"))? = Some(0);
        }
    }
    for (new, slot) in assemblies.iter_mut().filter(|id| id.is_some()).enumerate() {
        *slot = Some(new);
    }
    let type_id = |id| {
        types
            .get(id)
            .copied()
            .flatten()
            .ok_or_else(|| error("Unbound dependency type reference"))
    };
    let property_id = |id| {
        properties
            .get(id)
            .copied()
            .flatten()
            .ok_or_else(|| error("Dependency override requires explicit property rebinding"))
    };
    let mut imported_types = Vec::with_capacity(end);
    for (_, t) in source
        .types
        .iter()
        .enumerate()
        .filter(|(i, _)| selected_types[*i])
    {
        let mut t = t.clone();
        t.assembly = t
            .assembly
            .map(|id| assemblies[id].expect("selected assembly"));
        t.base_type = t.base_type.map(type_id).transpose()?;
        t.containing_type = t.containing_type.map(type_id).transpose()?;
        t.element_type = t.element_type.map(type_id).transpose()?;
        t.tuple_underlying_type = t.tuple_underlying_type.map(type_id).transpose()?;
        for id in t.all_interfaces.iter_mut().chain(&mut t.type_arguments) {
            *id = type_id(*id)?;
        }
        for id in &mut t.properties {
            *id = property_id(*id)?;
        }
        imported_types.push(t);
    }
    let mut imported_properties =
        Vec::with_capacity(selected_properties.iter().filter(|v| **v).count());
    for (_, p) in source
        .properties
        .iter()
        .enumerate()
        .filter(|(i, _)| selected_properties[*i])
    {
        let mut p = p.clone();
        p.type_id = type_id(p.type_id)?;
        p.overridden_property = p.overridden_property.map(property_id).transpose()?;
        imported_properties.push(p);
    }
    let mut required = source.required_symbols;
    for id in required.iter_mut().flatten() {
        *id = type_id(*id)?;
    }
    let mut working = Compilation {
        activated: false,
        registrations: Vec::new(),
        source_types: Vec::new(),
        occupied_type_names: Vec::new(),
        required_symbols: required,
        assemblies: source
            .assemblies
            .iter()
            .zip(&assemblies)
            .filter_map(|(a, id)| id.map(|_| a.clone()))
            .collect(),
        types: imported_types,
        properties: imported_properties,
        context_roots: source.context_roots.clone(),
    };
    let ids = RustSemanticImportIds {
        source_types: types,
        native_types,
    };
    bind(&mut working, &ids)?;
    if working.types.len() != end {
        return Err(error(
            "Native dependency declarations did not fill the allocated type table",
        ));
    }
    validate_links(&working)?;
    Ok(working)
}

fn validate_links(c: &Compilation) -> Result<(), CompilationError> {
    let type_exists = |id| {
        if id < c.types.len() {
            Ok(())
        } else {
            Err(error("Unbound final dependency type reference"))
        }
    };
    for id in c
        .required_symbols
        .into_iter()
        .flatten()
        .chain(c.source_types.iter().copied())
        .chain(c.registrations.iter().filter_map(|r| r.type_id))
    {
        type_exists(id)?;
    }
    for t in &c.types {
        if t.assembly.is_some_and(|id| id >= c.assemblies.len()) {
            return Err(error("Unbound final dependency assembly reference"));
        }
        for id in t
            .all_interfaces
            .iter()
            .chain(&t.type_arguments)
            .copied()
            .chain(
                [
                    t.base_type,
                    t.containing_type,
                    t.element_type,
                    t.tuple_underlying_type,
                ]
                .into_iter()
                .flatten(),
            )
        {
            type_exists(id)?;
        }
        if t.properties.iter().any(|id| *id >= c.properties.len()) {
            return Err(error("Unbound final dependency property reference"));
        }
    }
    for p in &c.properties {
        type_exists(p.type_id)?;
        if p.overridden_property
            .is_some_and(|id| id >= c.properties.len())
        {
            return Err(error("Unbound final dependency override reference"));
        }
    }
    Ok(())
}
