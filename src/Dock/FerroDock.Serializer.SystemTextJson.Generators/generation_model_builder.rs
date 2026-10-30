// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::*;
use std::{cmp::Ordering, collections::HashSet, fmt::Write};

fn ordinal(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}
fn sort_names(names: &mut Vec<String>) {
    names.sort_by(|a, b| ordinal(a, b));
    names.dedup();
}
struct Symbols<'a>(&'a Compilation);
impl Symbols<'_> {
    fn ty(&self, id: TypeId) -> Result<&TypeSymbol, CompilationError> {
        self.0
            .types
            .get(id)
            .ok_or_else(|| CompilationError(format!("Missing type symbol {id}")))
    }
    fn property(&self, id: PropertyId) -> Result<&PropertySymbol, CompilationError> {
        self.0
            .properties
            .get(id)
            .ok_or_else(|| CompilationError(format!("Missing property symbol {id}")))
    }
    fn named(kind: TypeKind) -> bool {
        !matches!(
            kind,
            TypeKind::Array | TypeKind::Pointer | TypeKind::TypeParameter | TypeKind::OtherNonNamed
        )
    }
    fn accessible(&self, id: TypeId) -> Result<bool, CompilationError> {
        let mut current = Some(id);
        let mut seen = HashSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(CompilationError("Cyclic containing type".into()));
            }
            let ty = self.ty(id)?;
            let accessible = match ty.accessibility {
                Accessibility::Public => true,
                Accessibility::Internal | Accessibility::ProtectedOrInternal => {
                    let id = ty
                        .assembly
                        .ok_or_else(|| CompilationError("Missing containing assembly".into()))?;
                    self.0
                        .assemblies
                        .get(id)
                        .ok_or_else(|| CompilationError(format!("Missing assembly {id}")))?
                        .grants_internal_access
                }
                _ => false,
            };
            if !accessible {
                return Ok(false);
            }
            current = ty.containing_type;
        }
        Ok(true)
    }
    fn open(&self, id: TypeId) -> Result<bool, CompilationError> {
        let mut current = Some(id);
        let mut seen = HashSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(CompilationError("Cyclic containing type".into()));
            }
            let ty = self.ty(id)?;
            if ty.is_unbound_generic {
                return Ok(true);
            }
            for id in &ty.type_arguments {
                if self.ty(*id)?.kind == TypeKind::TypeParameter {
                    return Ok(true);
                }
            }
            current = ty.containing_type;
        }
        Ok(false)
    }
    fn valid_registration(&self, id: TypeId) -> Result<bool, CompilationError> {
        let ty = self.ty(id)?;
        Ok(Self::named(ty.kind)
            && !ty.is_abstract
            && !matches!(ty.kind, TypeKind::Interface | TypeKind::Delegate)
            && !self.open(id)?
            && self.accessible(id)?)
    }
    fn assignable(&self, id: TypeId, target: TypeId) -> Result<bool, CompilationError> {
        if id == target {
            return Ok(true);
        }
        let ty = self.ty(id)?;
        if !Self::named(ty.kind) {
            return Ok(false);
        }
        if self.ty(target)?.kind == TypeKind::Interface {
            return Ok(ty.all_interfaces.contains(&target));
        }
        let mut current = ty.base_type;
        let mut seen = HashSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(CompilationError("Cyclic base type".into()));
            }
            if id == target {
                return Ok(true);
            }
            current = self.ty(id)?.base_type;
        }
        Ok(false)
    }
    fn contracts(&self, id: TypeId, required: &[TypeId; 8]) -> Result<[bool; 6], CompilationError> {
        let mut result = [false; 6];
        for i in 0..6 {
            result[i] = self.assignable(id, required[i])?;
        }
        Ok(result)
    }
    fn all_properties(&self, id: TypeId) -> Result<Vec<PropertyId>, CompilationError> {
        let mut result = Vec::new();
        let mut names = HashSet::new();
        if self.ty(id)?.kind == TypeKind::Interface {
            self.interface_properties(id, &mut names, &mut result)?;
            return Ok(result);
        }
        let mut current = Some(id);
        let mut seen = HashSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(CompilationError("Cyclic base type".into()));
            }
            self.declared_properties(id, &mut names, &mut result)?;
            current = self.ty(id)?.base_type;
        }
        for interface in &self.ty(id)?.all_interfaces {
            self.interface_properties(*interface, &mut names, &mut result)?;
        }
        Ok(result)
    }
    fn declared_properties(
        &self,
        id: TypeId,
        names: &mut HashSet<String>,
        result: &mut Vec<PropertyId>,
    ) -> Result<(), CompilationError> {
        for id in &self.ty(id)?.properties {
            let p = self.property(*id)?;
            if p.is_public && !p.is_static && names.insert(p.name.clone()) {
                result.push(*id);
            }
        }
        Ok(())
    }
    fn interface_properties(
        &self,
        id: TypeId,
        names: &mut HashSet<String>,
        result: &mut Vec<PropertyId>,
    ) -> Result<(), CompilationError> {
        for parent in self.ty(id)?.all_interfaces.iter().rev() {
            self.declared_properties(*parent, names, result)?;
        }
        self.declared_properties(id, names, result)
    }
    fn ignored(&self, id: TypeId, command: TypeId) -> Result<Vec<String>, CompilationError> {
        let mut names = Vec::new();
        for property in self.all_properties(id)? {
            let p = self.property(property)?;
            let ty = self.ty(p.type_id)?;
            let is_command = p.type_id == command
                || (Self::named(ty.kind) && ty.all_interfaces.contains(&command));
            let mut current = Some(property);
            let mut seen = HashSet::new();
            let mut ignored = false;
            while let Some(id) = current {
                if !seen.insert(id) {
                    return Err(CompilationError("Cyclic property override".into()));
                }
                let p = self.property(id)?;
                if p.ignore_data_member {
                    ignored = true;
                    break;
                }
                current = p.overridden_property;
            }
            if is_command || ignored {
                names.push(p.name.clone());
            }
        }
        sort_names(&mut names);
        Ok(names)
    }
    fn containing_assembly(&self, id: TypeId) -> Result<Option<usize>, CompilationError> {
        let mut current = id;
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(current) {
                return Err(CompilationError("Cyclic element type".into()));
            }
            let t = self.ty(current)?;
            if matches!(t.kind, TypeKind::Array | TypeKind::Pointer) {
                current = t
                    .element_type
                    .ok_or_else(|| CompilationError("Missing element type".into()))?;
            } else {
                return Ok(t.assembly);
            }
        }
    }
    fn qualified_name(
        &self,
        id: TypeId,
        active: &mut HashSet<TypeId>,
    ) -> Result<String, CompilationError> {
        let mut out = self.name(id, active)?;
        if let Some(id) = self.containing_assembly(id)? {
            let a = self
                .0
                .assemblies
                .get(id)
                .ok_or_else(|| CompilationError(format!("Missing assembly {id}")))?;
            out.push_str(", ");
            out.push_str(&a.name);
            out.push_str(", Version=");
            out.push_str(&a.version);
            out.push_str(", Culture=");
            out.push_str(
                a.culture
                    .as_deref()
                    .filter(|v| !v.is_empty())
                    .unwrap_or("neutral"),
            );
            out.push_str(", PublicKeyToken=");
            if a.public_key_token.is_empty() {
                out.push_str("null");
            } else {
                for byte in &a.public_key_token {
                    write!(out, "{byte:02x}").expect("String formatter");
                }
            }
        }
        Ok(out)
    }
    fn name(&self, id: TypeId, active: &mut HashSet<TypeId>) -> Result<String, CompilationError> {
        if !active.insert(id) {
            return Err(CompilationError("Cyclic type arguments".into()));
        }
        let t = self.ty(id)?;
        let result = match t.kind {
            TypeKind::Array => {
                if t.array_rank == 0 {
                    return Err(CompilationError("Invalid array rank".into()));
                }
                let mut out = self.name(
                    t.element_type
                        .ok_or_else(|| CompilationError("Missing array element".into()))?,
                    active,
                )?;
                out.push('[');
                for _ in 1..t.array_rank {
                    out.push(',');
                }
                out.push(']');
                out
            }
            TypeKind::Pointer => format!(
                "{}*",
                self.name(
                    t.element_type
                        .ok_or_else(|| CompilationError("Missing pointer element".into()))?,
                    active
                )?
            ),
            TypeKind::TypeParameter | TypeKind::OtherNonNamed => t.display_name.clone(),
            _ => {
                let effective = t.tuple_underlying_type.unwrap_or(id);
                let t = self.ty(effective)?;
                let mut chain = Vec::new();
                let mut seen = HashSet::new();
                let mut current = Some(effective);
                while let Some(id) = current {
                    if !seen.insert(id) {
                        return Err(CompilationError("Cyclic containing type".into()));
                    }
                    chain.push(id);
                    current = self.ty(id)?.containing_type;
                }
                chain.reverse();
                let mut out = String::new();
                if !t.namespace.is_empty() {
                    out.push_str(&t.namespace);
                    out.push('.');
                }
                let mut arguments = Vec::new();
                for (i, id) in chain.iter().enumerate() {
                    let segment = self.ty(*id)?;
                    if i > 0 {
                        out.push('+');
                    }
                    out.push_str(&segment.metadata_name);
                    let start = segment.type_arguments.len().saturating_sub(segment.arity);
                    arguments.extend_from_slice(&segment.type_arguments[start..]);
                }
                if !arguments.is_empty() {
                    out.push('[');
                    for (i, id) in arguments.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        out.push('[');
                        out.push_str(&self.qualified_name(*id, active)?);
                        out.push(']');
                    }
                    out.push(']');
                }
                out
            }
        };
        active.remove(&id);
        Ok(result)
    }
}
/// Computes the original runtime full name from semantic named/array/pointer/tuple facts.
pub fn full_name(compilation: &Compilation, id: TypeId) -> Result<String, CompilationError> {
    Symbols(compilation).name(id, &mut HashSet::new())
}
pub(crate) fn assignable(
    compilation: &Compilation,
    id: TypeId,
    target: TypeId,
) -> Result<bool, CompilationError> {
    Symbols(compilation).assignable(id, target)
}
fn invalid(r: &Registration) -> Diagnostic {
    Diagnostic{id:"DSTJ001",message:format!("Type '{}' is not a supported Dock JSON source generation registration. Register a closed, non-abstract named type.",r.display_name),location:r.location.clone()}
}

/// Builds the source semantic generation plan. Compiler syntax scanning and source emission
/// are separate stages; this pass neither reflects runtime fields nor creates model objects.
pub fn build(c: &Compilation) -> Result<GenerationModel, CompilationError> {
    let s = Symbols(c);
    let mut diagnostics = Vec::new();
    let mut registered = Vec::new();
    for r in &c.registrations {
        if r.valid_type_constant
            && r.type_id.is_some()
            && s.valid_registration(r.type_id.expect("checked registration"))?
        {
            registered.push(r);
        } else {
            diagnostics.push(invalid(r));
        }
    }
    if !c.activated {
        for r in registered {
            diagnostics.push(Diagnostic{id:"DSTJ002",message:"Add [assembly: DockJsonSourceGeneration] before using DockJsonSerializableAttribute".into(),location:r.location.clone()});
        }
        return Ok(GenerationModel::empty(diagnostics));
    }
    let missing: Vec<_> = c
        .required_symbols
        .iter()
        .enumerate()
        .filter(|(_, id)| id.is_none())
        .map(|(i, _)| METADATA_NAMES[i])
        .collect();
    if !missing.is_empty() {
        diagnostics.push(Diagnostic{id:"DSTJ004",message:format!("Dock JSON source generation requires references to Dock model contracts. Missing: {}.",missing.join(", ")),location:None});
        return Ok(GenerationModel::empty(diagnostics));
    }
    let required = c
        .required_symbols
        .map(|id| id.expect("validated required symbol"));
    for id in required {
        s.ty(id)?;
    }
    let mut filtered = Vec::new();
    for r in registered {
        let id = r.type_id.expect("validated registration");
        let t = s.ty(id)?;
        if s.contracts(id, &required)?.iter().any(|v| *v)
            || (t.kind == TypeKind::Class && !t.special)
        {
            filtered.push(r);
        } else {
            let mut r = r.clone();
            r.display_name = t.display_name.clone();
            diagnostics.push(invalid(&r));
        }
    }
    let mut serializable = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |id: TypeId,
                   location: Option<String>,
                   is_object_payload: bool|
     -> Result<(), CompilationError> {
        if seen.insert(id) {
            let t = s.ty(id)?;
            serializable.push(SerializableTypeModel {
                type_id: id,
                type_expression: t.expression.clone(),
                display_name: t.display_name.clone(),
                discriminator_key: full_name(c, id)?,
                ignored_members: s.ignored(id, required[6])?,
                location,
                contracts: s.contracts(id, &required)?,
                is_object_payload,
            });
        }
        Ok(())
    };
    for id in &c.source_types {
        let t = s.ty(*id)?;
        if t.kind == TypeKind::Class
            && !t.is_abstract
            && !s.open(*id)?
            && s.accessible(*id)?
            && s.contracts(*id, &required)?.iter().any(|v| *v)
        {
            add(*id, t.location.clone(), false)?;
        }
    }
    for r in filtered {
        let id = r.type_id.expect("validated registration");
        let payload = !s.contracts(id, &required)?.iter().any(|v| *v);
        add(id, r.location.clone(), payload)?;
    }
    serializable.sort_by(|a, b| ordinal(&a.type_expression, &b.type_expression));
    let mut ignored = Vec::new();
    for t in &serializable {
        if !t.ignored_members.is_empty() {
            ignored.push(IgnoredMembersModel {
                type_expression: t.type_expression.clone(),
                member_names: t.ignored_members.clone(),
            });
        }
    }
    for (i, id) in required[..6].iter().enumerate() {
        let mut names = s.ignored(*id, required[6])?;
        for t in &serializable {
            if t.contracts[i] {
                names.extend(t.ignored_members.clone());
            }
        }
        sort_names(&mut names);
        if !names.is_empty() {
            ignored.push(IgnoredMembersModel {
                type_expression: s.ty(*id)?.expression.clone(),
                member_names: names,
            });
        }
    }
    ignored.sort_by(|a, b| ordinal(&a.type_expression, &b.type_expression));
    let kinds = [
        DockBaseKind::Dockable,
        DockBaseKind::Dock,
        DockBaseKind::RootDock,
        DockBaseKind::DockWindow,
        DockBaseKind::DocumentTemplate,
        DockBaseKind::ToolTemplate,
        DockBaseKind::Object,
    ];
    let names = [
        "IDockable",
        "IDock",
        "IRootDock",
        "IDockWindow",
        "IDocumentTemplate",
        "IToolTemplate",
        "object",
    ];
    let mut polymorphisms = Vec::new();
    for (i, kind) in kinds.into_iter().enumerate() {
        let mut types: Vec<_> = serializable
            .iter()
            .filter(|t| {
                if i == 6 {
                    t.is_object_payload
                } else {
                    t.contracts[i]
                }
            })
            .map(|t| DerivedTypeModel {
                type_id: t.type_id,
                type_expression: t.type_expression.clone(),
                display_name: t.display_name.clone(),
                discriminator_key: t.discriminator_key.clone(),
                location: t.location.clone(),
            })
            .collect();
        types.sort_by(|a, b| {
            ordinal(&a.discriminator_key, &b.discriminator_key)
                .then_with(|| ordinal(&a.type_expression, &b.type_expression))
        });
        let mut collisions = HashSet::new();
        let mut index = 0;
        while index < types.len() {
            let start = index;
            index += 1;
            while index < types.len()
                && types[index].discriminator_key == types[start].discriminator_key
            {
                index += 1;
            }
            if index - start > 1 {
                let key = types[start].discriminator_key.clone();
                collisions.insert(key.clone());
                diagnostics.push(Diagnostic {
                    id: "DSTJ003",
                    message: format!(
                        "Types '{}' share the discriminator '{}' for '{}'",
                        types[start..index]
                            .iter()
                            .map(|t| t.display_name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                        key,
                        names[i]
                    ),
                    location: types[start..index].iter().find_map(|t| t.location.clone()),
                });
            }
        }
        types.retain(|t| !collisions.contains(&t.discriminator_key));
        let handling = match kind {
            DockBaseKind::RootDock
            | DockBaseKind::DocumentTemplate
            | DockBaseKind::ToolTemplate => UnknownDerivedTypeHandling::FallBackToNearestAncestor,
            DockBaseKind::Object => UnknownDerivedTypeHandling::FailSerialization,
            _ => UnknownDerivedTypeHandling::FallBackToBaseType,
        };
        polymorphisms.push(PolymorphismModel {
            kind,
            base_type_expression: if i == 6 {
                c.context_roots.object.clone()
            } else {
                s.ty(required[i])?.expression.clone()
            },
            base_display_name: names[i],
            unknown_derived_type_handling: handling,
            ignore_unrecognized_type_discriminators: i != 6,
            derived_types: types,
        });
    }
    let mut contexts = vec![
        c.context_roots.object.clone(),
        c.context_roots.dockable_list.clone(),
        c.context_roots.window_list.clone(),
    ];
    for id in &required[..6] {
        contexts.push(s.ty(*id)?.expression.clone());
    }
    contexts.extend(serializable.iter().map(|t| t.type_expression.clone()));
    sort_names(&mut contexts);
    let base = "DockSerializerGeneratedJsonContext";
    let mut context = base.to_owned();
    let mut suffix = 0;
    let occupied: HashSet<_> = c.occupied_type_names.iter().collect();
    while occupied.contains(&context) {
        suffix += 1;
        context = format!("{base}_{suffix}");
    }
    Ok(GenerationModel {
        should_generate: true,
        diagnostics,
        context_type_name: context,
        context_types: contexts,
        serializable_types: serializable,
        polymorphisms,
        ignored_members: ignored,
    })
}
