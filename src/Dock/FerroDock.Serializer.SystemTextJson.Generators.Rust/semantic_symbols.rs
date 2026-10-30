// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{RustDeclarationKind, RustSourceDeclaration};
use ferrodock_serializer_system_text_json_generators::{
    Accessibility, Compilation, CompilationError, PropertyId, PropertySymbol, TypeId, TypeKind,
    TypeSymbol,
};
use ferroui_build_scan::model::{AttributeModel, RustType, TypeKind as MetadataKind, TypeModel};
use std::collections::HashSet;

/// One actual source property override resolved by the target compiler.
#[derive(Clone, Debug)]
pub struct RustPropertyOverride {
    /// Original declared property name, before JSON renaming.
    pub name: String,
    /// Property on the original base-class chain.
    pub overridden_property: PropertyId,
}
/// Logical facts absent from the shared declaration model and native struct syntax.
#[derive(Clone, Debug)]
pub struct RustSemanticTypeOptions {
    /// Logical source metadata name, independent of a markup alias.
    pub metadata_name: String,
    /// Logical source namespace used by the original full-name algorithm.
    pub namespace: String,
    /// Assembly identity already registered in the compiler's table.
    pub assembly: Option<usize>,
    /// Accessibility resolved for the context's actual emission scope.
    pub accessibility: Accessibility,
    /// Explicit source class abstraction; never inferred from a missing constructor.
    pub is_abstract: bool,
    /// Original containing type for logical nested source classes.
    pub containing_type: Option<TypeId>,
    /// Resolved original base class, including the logical object base when applicable.
    pub base_type: Option<TypeId>,
    /// Complete compiler-ordered interface closure, retaining original precedence.
    pub all_interfaces: Vec<TypeId>,
    /// Actual overrides; a hidden property has no override entry.
    pub property_overrides: Vec<RustPropertyOverride>,
}

/// One native declaration to append to a semantic compilation in a single transaction.
pub struct RustSemanticDeclaration<'a> {
    /// Ordinary or already instantiated closed generic shared metadata.
    pub metadata: &'a TypeModel,
    /// Actual Rust definition supplying provenance and parameter order.
    pub source: &'a RustSourceDeclaration,
    /// Resolved logical identity, accessibility and inheritance facts.
    pub options: &'a RustSemanticTypeOptions,
}

/// Allocates and binds a batch of native types without preexisting application symbols.
///
/// IDs are contiguous in declaration order and supplied to `resolve` in that same order.
/// The resolver explicitly maps both dependency types and native nullable/shared handles;
/// no handle erasure or name guessing is performed. Property references and closed generic
/// arguments may point forward within this batch, including mutually referring classes.
/// Base classes, containing types and interface closures must precede their dependents;
/// override links must name existing original property rows.
///
/// A private working compilation contains only fully constructed type headers. Generic
/// argument closure is checked after all headers are present. Any error leaves the entire
/// input unchanged, including property rows. No placeholder symbols, runtime objects,
/// registrations or discovery entries are published. Source traversal/discovery order
/// remains the compiler host's explicit responsibility.
pub fn bind_semantic_declarations(
    c: &mut Compilation,
    declarations: &[RustSemanticDeclaration<'_>],
    resolve: impl Fn(&RustType, &[TypeId]) -> Option<TypeId>,
) -> Result<Vec<TypeId>, CompilationError> {
    let first = c.types.len();
    let end = first
        .checked_add(declarations.len())
        .ok_or_else(|| error("Semantic type table capacity overflow"))?;
    let ids: Vec<_> = (first..end).collect();
    let mut names: HashSet<_> = c.types.iter().map(|t| t.expression.as_str()).collect();
    for d in declarations {
        if !names.insert(d.metadata.rust_path.text.as_str()) {
            return Err(error("Duplicate native semantic type identity"));
        }
    }
    let mut working = c.clone();
    for (d, id) in declarations.iter().zip(&ids) {
        let resolve = |ty: &RustType| resolve(ty, &ids);
        let arguments = if d.source.parameters.is_empty() {
            None
        } else {
            Some(generic_arguments(
                d.metadata, d.source, d.options, &resolve,
            )?)
        };
        bind_semantic_type(
            &mut working,
            (*id, end),
            d.metadata,
            d.source,
            d.options,
            arguments,
            resolve,
        )?;
    }
    for id in &ids {
        validate_closed_arguments(&working, &working.types[*id].type_arguments, *id)?;
    }
    *c = working;
    Ok(ids)
}

fn error(message: impl Into<String>) -> CompilationError {
    CompilationError(message.into())
}
fn reference(
    type_: &RustType,
    type_count: usize,
    resolve: &impl Fn(&RustType) -> Option<TypeId>,
) -> Result<TypeId, CompilationError> {
    if !type_.is_resolved() {
        return Err(error(format!("Unresolved semantic type {}", type_.text)));
    }
    let id = resolve(type_)
        .ok_or_else(|| error(format!("Missing semantic type binding for {}", type_.text)))?;
    if id >= type_count {
        return Err(error(format!("Missing semantic target type {id}")));
    }
    Ok(id)
}
fn ignore_data_member(attributes: &[AttributeModel]) -> bool {
    attributes.iter().any(|a| {
        matches!(
            a.name.as_str(),
            "IgnoreDataMember"
                | "IgnoreDataMemberAttribute"
                | "System.Runtime.Serialization.IgnoreDataMemberAttribute"
        )
    })
}
fn inherited_property(
    c: &Compilation,
    mut base: Option<TypeId>,
    property: PropertyId,
) -> Result<bool, CompilationError> {
    let mut seen = HashSet::new();
    while let Some(id) = base {
        if !seen.insert(id) {
            return Err(error("Cyclic semantic base-class chain"));
        }
        let ty = c
            .types
            .get(id)
            .ok_or_else(|| error("Missing semantic base class"))?;
        if ty.properties.contains(&property) {
            return Ok(true);
        }
        base = ty.base_type;
    }
    Ok(false)
}

/// Constructs or replaces a non-generic logical type from actual declaration metadata.
///
/// The shared declaration supplies its logical kind and declared public instance/static
/// properties. Type identity is explicit: a markup alias need not be a serialization name.
/// Actual Rust syntax supplies provenance and confirms the
/// representation. Compiler-provided abstraction/access/base/interface/override facts
/// remain explicit. Property types must resolve through the compiler's real type table;
/// nullable/shared handles are not stripped or guessed by this function.
/// A nonflags Int32 enum represented by a struct requires an actual single-field
/// transparent declaration and an explicit typed factory in its shared metadata.
///
/// `symbol` may name an existing entry or the next entry in the type table. Property
/// symbols are appended and the type commits only after all checks succeed, including
/// self references. Conditional, generic, registered-property and indexer schemas need
/// their complete target semantics and return errors here. No runtime models are created.
pub fn bind_declared_semantic_type(
    c: &mut Compilation,
    symbol: TypeId,
    declaration: &TypeModel,
    source: &RustSourceDeclaration,
    options: &RustSemanticTypeOptions,
    resolve: impl Fn(&RustType) -> Option<TypeId>,
) -> Result<(), CompilationError> {
    let type_count = c.types.len().max(symbol.saturating_add(1));
    bind_semantic_type(
        c,
        (symbol, type_count),
        declaration,
        source,
        options,
        None,
        resolve,
    )
}

/// Constructs a closed, non-nested generic semantic type from instantiated native metadata.
/// Type parameters and declaration order come from the actual Rust definition. The
/// shared metadata and concrete Rust type must agree; all logical argument symbols
/// must already be closed compiler facts. Source identities and inheritance remain
/// explicit as for ordinary declarations. No missing arguments are fabricated.
pub fn bind_closed_generic_semantic_type(
    c: &mut Compilation,
    symbol: TypeId,
    declaration: &TypeModel,
    source: &RustSourceDeclaration,
    options: &RustSemanticTypeOptions,
    resolve: impl Fn(&RustType) -> Option<TypeId>,
) -> Result<(), CompilationError> {
    let ids = generic_arguments(declaration, source, options, &resolve)?;
    validate_closed_arguments(c, &ids, symbol)?;
    let type_count = c.types.len().max(symbol.saturating_add(1));
    bind_semantic_type(
        c,
        (symbol, type_count),
        declaration,
        source,
        options,
        Some(ids),
        resolve,
    )
}

fn generic_arguments(
    declaration: &TypeModel,
    source: &RustSourceDeclaration,
    options: &RustSemanticTypeOptions,
    resolve: &impl Fn(&RustType) -> Option<TypeId>,
) -> Result<Vec<TypeId>, CompilationError> {
    use crate::{generic_metadata::named_arguments, RustGenericParameterKind};
    use quote::ToTokens;
    if options.containing_type.is_some()
        || source.parameters.is_empty()
        || source
            .parameters
            .iter()
            .any(|p| p.kind != RustGenericParameterKind::Type)
    {
        return Err(error(
            "Closed generic symbols require non-nested source type parameters",
        ));
    }
    let (definition, arguments) = named_arguments(&declaration.rust_path)?;
    let generic = declaration
        .generic
        .as_ref()
        .ok_or_else(|| error("Missing instantiated generic metadata"))?;
    let arity = source.parameters.len();
    if definition != source.path.text.trim_start_matches("::")
        || arguments.len() != arity
        || generic.arguments.len() != arity
        || generic.definition.is_empty()
    {
        return Err(error(
            "Closed generic metadata differs from source definition/arity",
        ));
    }
    if options
        .metadata_name
        .rsplit_once('`')
        .and_then(|(_, n)| n.parse::<usize>().ok())
        != Some(arity)
    {
        return Err(error("Logical generic metadata name has incorrect arity"));
    }
    let mut ids = Vec::with_capacity(arity);
    for (argument, metadata) in arguments.iter().zip(&generic.arguments) {
        crate::generic_metadata::concrete_argument(argument)?;
        let parsed = syn::parse_str::<syn::Type>(&metadata.text)
            .map_err(|_| error("Invalid generic argument metadata"))?;
        if argument.to_token_stream().to_string() != parsed.to_token_stream().to_string()
            || !metadata.is_resolved()
        {
            return Err(error(
                "Generic argument metadata differs from actual Rust type",
            ));
        }
        ids.push(resolve(metadata).ok_or_else(|| error("Missing generic argument type binding"))?);
    }
    Ok(ids)
}

fn validate_closed_arguments(
    c: &Compilation,
    arguments: &[TypeId],
    symbol: TypeId,
) -> Result<(), CompilationError> {
    fn closed(
        c: &Compilation,
        id: TypeId,
        constructed: TypeId,
        active: &mut HashSet<TypeId>,
    ) -> Result<(), CompilationError> {
        if id == constructed || !active.insert(id) {
            return Err(error("Cyclic constructed generic argument"));
        }
        let ty = c
            .types
            .get(id)
            .ok_or_else(|| error("Missing generic argument symbol"))?;
        if ty.kind == TypeKind::TypeParameter || ty.is_unbound_generic {
            return Err(error("Generic argument is not closed"));
        }
        for id in &ty.type_arguments {
            closed(c, *id, constructed, active)?;
        }
        for id in [
            ty.containing_type,
            ty.element_type,
            ty.tuple_underlying_type,
        ]
        .into_iter()
        .flatten()
        {
            closed(c, id, constructed, active)?;
        }
        active.remove(&id);
        Ok(())
    }
    for id in arguments {
        closed(c, *id, symbol, &mut HashSet::new())?;
    }
    Ok(())
}

fn bind_semantic_type(
    c: &mut Compilation,
    slot: (TypeId, usize),
    declaration: &TypeModel,
    source: &RustSourceDeclaration,
    options: &RustSemanticTypeOptions,
    generic_arguments: Option<Vec<TypeId>>,
    resolve: impl Fn(&RustType) -> Option<TypeId>,
) -> Result<(), CompilationError> {
    let (symbol, type_count) = slot;
    if symbol > c.types.len() {
        return Err(error("Semantic type slot is not allocated"));
    }
    if options.metadata_name.is_empty() {
        return Err(error("Missing logical source type identity"));
    }
    if !declaration.rust_path.is_resolved()
        || (generic_arguments.is_none()
            && crate::source_declarations::metadata_definition_path(declaration)
                .is_none_or(|path| path != source.path))
    {
        return Err(error(
            "Shared declaration and actual Rust definition differ",
        ));
    }
    if !source.cfg.is_empty() || !declaration.cfg.is_empty() {
        return Err(error("Semantic declaration requires target cfg selection"));
    }
    if generic_arguments.is_none()
        && (!source.parameters.is_empty() || declaration.generic.is_some())
    {
        return Err(error(
            "Generic semantic declarations require resolved substitutions",
        ));
    }
    if !declaration.registered.is_empty() || !declaration.indexers.is_empty() {
        return Err(error(
            "Registered properties/indexers require resolved source member facts",
        ));
    }
    let kind = match (declaration.kind, source.kind) {
        (MetadataKind::Class, RustDeclarationKind::Struct) => TypeKind::Class,
        (MetadataKind::Struct, RustDeclarationKind::Struct) => TypeKind::Struct,
        (MetadataKind::Enum, RustDeclarationKind::Enum) => TypeKind::Enum,
        (MetadataKind::Enum, RustDeclarationKind::Struct) if declaration.is_flags => TypeKind::Enum,
        (MetadataKind::Enum, RustDeclarationKind::Struct)
            if source
                .transparent_field
                .as_ref()
                .is_some_and(|field| field.is_resolved() && field.text == "i32")
                && declaration
                    .enum_i32_factory
                    .as_ref()
                    .is_some_and(|name| syn::parse_str::<syn::Ident>(name).is_ok()) =>
        {
            TypeKind::Enum
        }
        (MetadataKind::Interface, RustDeclarationKind::Trait) => TypeKind::Interface,
        _ => {
            return Err(error(
                "Logical metadata kind has no matching Rust representation",
            ))
        }
    };
    if kind != TypeKind::Class && options.is_abstract != (kind == TypeKind::Interface) {
        return Err(error(
            "Abstraction differs from logical value/interface kind",
        ));
    }
    if kind == TypeKind::Interface && options.base_type.is_some() {
        return Err(error(
            "Interface inheritance belongs in the resolved interface closure",
        ));
    }
    if options.assembly.is_some_and(|id| id >= c.assemblies.len()) {
        return Err(error("Missing semantic assembly identity"));
    }
    if let Some(id) = options.containing_type {
        if id == symbol || id >= c.types.len() {
            return Err(error("Invalid semantic containing type"));
        }
    }
    let mut seen_containers = HashSet::new();
    let mut container = options.containing_type;
    while let Some(id) = container {
        if id == symbol || !seen_containers.insert(id) {
            return Err(error("Cyclic semantic containing type"));
        }
        container = c
            .types
            .get(id)
            .ok_or_else(|| error("Missing semantic containing type"))?
            .containing_type;
    }
    if let Some(id) = options.base_type {
        if id == symbol || c.types.get(id).is_none_or(|t| t.kind != TypeKind::Class) {
            return Err(error("Invalid semantic base class"));
        }
    }
    if let Some(base) = declaration
        .base
        .as_ref()
        .filter(|_| kind != TypeKind::Interface)
    {
        if Some(reference(base, type_count, &resolve)?) != options.base_type {
            return Err(error("Declared base differs from resolved source base"));
        }
    }
    let mut seen_bases = HashSet::new();
    let mut base = options.base_type;
    while let Some(id) = base {
        if id == symbol || !seen_bases.insert(id) {
            return Err(error("Cyclic semantic base-class chain"));
        }
        let ty = c
            .types
            .get(id)
            .ok_or_else(|| error("Missing semantic base class"))?;
        if ty.kind != TypeKind::Class {
            return Err(error("Invalid semantic base-class chain"));
        }
        base = ty.base_type;
    }
    let mut interfaces = HashSet::new();
    for id in &options.all_interfaces {
        if !interfaces.insert(*id)
            || *id == symbol
            || c.types
                .get(*id)
                .is_none_or(|t| t.kind != TypeKind::Interface)
        {
            return Err(error("Invalid or duplicate semantic interface"));
        }
    }
    for interface in &declaration.interfaces {
        if !interfaces.contains(&reference(interface, type_count, &resolve)?) {
            return Err(error(
                "Declared interface is absent from resolved source closure",
            ));
        }
    }
    if let Some(base) = declaration
        .base
        .as_ref()
        .filter(|_| kind == TypeKind::Interface)
    {
        if !interfaces.contains(&reference(base, type_count, &resolve)?) {
            return Err(error(
                "Declared base interface is absent from resolved source closure",
            ));
        }
    }
    if options.all_interfaces.iter().any(|id| {
        c.types[*id]
            .all_interfaces
            .iter()
            .any(|i| !interfaces.contains(i))
    }) {
        return Err(error(
            "Resolved interface closure omits an inherited interface",
        ));
    }
    if let Some(id) = options.base_type {
        if c.types[id]
            .all_interfaces
            .iter()
            .any(|i| !interfaces.contains(i))
        {
            return Err(error("Resolved interface closure omits a base interface"));
        }
    }
    let mut overrides = HashSet::new();
    for p in &options.property_overrides {
        if !overrides.insert(&p.name) {
            return Err(error("Duplicate semantic property override"));
        }
        let parent = c
            .properties
            .get(p.overridden_property)
            .ok_or_else(|| error("Missing overridden property"))?;
        if parent.name != p.name
            || !inherited_property(c, options.base_type, p.overridden_property)?
        {
            return Err(error("Override does not name an original base property"));
        }
    }
    let mut properties =
        Vec::with_capacity(declaration.properties.len() + declaration.static_properties.len());
    let mut names = HashSet::new();
    for (fields, is_static) in [
        (&declaration.properties, false),
        (&declaration.static_properties, true),
    ] {
        for p in fields {
            if p.name.is_empty() || !p.parameters.is_empty() || !names.insert(p.name.clone()) {
                return Err(error(
                    "Ambiguous or unsupported semantic property declaration",
                ));
            }
            let parent = options
                .property_overrides
                .iter()
                .find(|o| o.name == p.name)
                .map(|o| o.overridden_property);
            if parent.is_some_and(|id| c.properties[id].is_static != is_static) {
                return Err(error("Override changes static property semantics"));
            }
            let ignored = ignore_data_member(&p.attributes)
                || declaration
                    .property_attributes
                    .iter()
                    .any(|(name, attrs)| name == &p.name && ignore_data_member(attrs));
            properties.push(PropertySymbol {
                name: p.name.clone(),
                type_id: reference(&p.value_type, type_count, &resolve)?,
                is_public: true,
                is_static,
                ignore_data_member: ignored,
                overridden_property: parent,
            });
        }
    }
    if options
        .property_overrides
        .iter()
        .any(|o| !names.contains(&o.name))
    {
        return Err(error("Override has no declared source property"));
    }
    let first = c.properties.len();
    let ty = TypeSymbol {
        expression: declaration.rust_path.text.clone(),
        display_name: if options.namespace.is_empty() {
            options.metadata_name.clone()
        } else {
            format!("{}.{}", options.namespace, options.metadata_name)
        },
        metadata_name: options.metadata_name.clone(),
        namespace: options.namespace.clone(),
        kind,
        special: false,
        is_abstract: options.is_abstract,
        accessibility: options.accessibility,
        assembly: options.assembly,
        containing_type: options.containing_type,
        base_type: options.base_type,
        all_interfaces: options.all_interfaces.clone(),
        arity: generic_arguments.as_ref().map_or(0, Vec::len),
        type_arguments: generic_arguments.unwrap_or_default(),
        is_unbound_generic: false,
        tuple_underlying_type: None,
        element_type: None,
        array_rank: 0,
        properties: (first..first + properties.len()).collect(),
        location: Some(source.location.clone()),
    };
    c.properties.extend(properties);
    if symbol == c.types.len() {
        c.types.push(ty);
    } else {
        c.types[symbol] = ty;
    }
    Ok(())
}
