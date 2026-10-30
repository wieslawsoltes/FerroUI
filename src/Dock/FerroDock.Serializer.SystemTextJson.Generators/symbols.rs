// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
/// Stable index into the compilation's type-symbol table.
pub type TypeId = usize;
/// Stable index into the compilation's property-symbol table.
pub type PropertyId = usize;
/// Compiler type kinds read by the original generator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    /// Logical reference class.
    Class,
    /// Value type.
    Struct,
    /// Enumeration value type.
    Enum,
    /// Interface contract.
    Interface,
    /// Callable delegate type.
    Delegate,
    /// Array with an element and rank.
    Array,
    /// Pointer-shaped compiler type; no runtime pointer is created.
    Pointer,
    /// Open generic parameter.
    TypeParameter,
    /// Other named compiler symbol.
    OtherNamed,
    /// A dynamic, function-pointer or other non-named compiler symbol.
    OtherNonNamed,
}
/// Declared accessibility; enclosing types are checked independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accessibility {
    /// Accessible from any consumer.
    Public,
    /// Accessible to the declaring or friend assembly.
    Internal,
    /// Protected or declaring/friend assembly access.
    ProtectedOrInternal,
    /// Private to the declaring scope.
    Private,
    /// Protected subclass access.
    Protected,
    /// Protected and assembly access.
    ProtectedAndInternal,
    /// Not applicable or otherwise unavailable.
    Other,
}
/// Assembly identity and the compiler's friend-assembly decision for the current consumer.
#[derive(Clone, Debug)]
pub struct AssemblySymbol {
    /// Simple assembly name.
    pub name: String,
    /// Four-component assembly version.
    pub version: String,
    /// Empty or absent means neutral culture.
    pub culture: Option<String>,
    /// Original public-key token bytes.
    pub public_key_token: Vec<u8>,
    /// Includes access to the current compilation's own internal types.
    pub grants_internal_access: bool,
}
/// Immutable compiler facts for a type, including named and generic argument types.
#[derive(Clone, Debug)]
pub struct TypeSymbol {
    /// Target-language fully qualified expression, supplied by the compiler adapter.
    pub expression: String,
    /// Diagnostic display name.
    pub display_name: String,
    /// Metadata simple name, including generic arity.
    pub metadata_name: String,
    /// Dotted namespace, empty for the global namespace.
    pub namespace: String,
    /// Compiler kind; Rust reference-class adapters supply their logical class kind.
    pub kind: TypeKind,
    /// Builtin CLR special types are not object-payload registrations.
    pub special: bool,
    /// Abstract classes cannot be registered or discovered.
    pub is_abstract: bool,
    /// Declared type accessibility.
    pub accessibility: Accessibility,
    /// Containing assembly, if the compiler supplies one.
    pub assembly: Option<usize>,
    /// Containing type for a nested declaration.
    pub containing_type: Option<TypeId>,
    /// Original base class.
    pub base_type: Option<TypeId>,
    /// Compiler-ordered complete interface closure.
    pub all_interfaces: Vec<TypeId>,
    /// Actual type arguments as reported for this named segment.
    pub type_arguments: Vec<TypeId>,
    /// Number of type arguments owned by this segment.
    pub arity: usize,
    /// An unbound generic registration.
    pub is_unbound_generic: bool,
    /// Underlying named tuple type, preserving its generic arguments.
    pub tuple_underlying_type: Option<TypeId>,
    /// Element for array/pointer kinds.
    pub element_type: Option<TypeId>,
    /// Array rank, positive for array kinds.
    pub array_rank: usize,
    /// Public and nonpublic declared property symbols, in compiler order.
    pub properties: Vec<PropertyId>,
    /// Source declaration location; absent for metadata-only symbols.
    pub location: Option<String>,
}
/// Property facts used for inheritance, hiding and ignore rules.
#[derive(Clone, Debug)]
pub struct PropertySymbol {
    /// Original public property name.
    pub name: String,
    /// Declared property type.
    pub type_id: TypeId,
    /// Public declared accessibility, irrespective of individual accessor visibility.
    pub is_public: bool,
    /// Static properties are excluded.
    pub is_static: bool,
    /// The exact source IgnoreDataMember attribute was present on this declaration.
    pub ignore_data_member: bool,
    /// Original overridden property; a hidden property has no such link.
    pub overridden_property: Option<PropertyId>,
}
/// One registration attribute before semantic filtering.
#[derive(Clone, Debug)]
pub struct Registration {
    /// Attribute argument type, if present.
    pub type_id: Option<TypeId>,
    /// Exactly one constructor argument of compiler kind Type.
    pub valid_type_constant: bool,
    /// Source diagnostic representation, including unsupported arguments.
    pub display_name: String,
    /// Attribute source location.
    pub location: Option<String>,
}
/// Type-expression seeds whose spelling belongs to the target compiler adapter.
#[derive(Clone, Debug)]
pub struct ContextRoots {
    /// The original System.Object counterpart.
    pub object: String,
    /// IList of dockable handles.
    pub dockable_list: String,
    /// IList of window handles.
    pub window_list: String,
}
/// Required symbols in original missing-symbol diagnostic order.
pub const METADATA_NAMES: [&str; 8] = [
    "Dock.Model.Core.IDockable",
    "Dock.Model.Core.IDock",
    "Dock.Model.Controls.IRootDock",
    "Dock.Model.Core.IDockWindow",
    "Dock.Model.Controls.IDocumentTemplate",
    "Dock.Model.Controls.IToolTemplate",
    "System.Windows.Input.ICommand",
    "System.Runtime.Serialization.IgnoreDataMemberAttribute",
];
/// Complete semantic input for one incremental compilation snapshot.
#[derive(Clone, Debug)]
pub struct Compilation {
    /// Whether the assembly activation attribute is present.
    pub activated: bool,
    /// Attribute registrations, in declaration order.
    pub registrations: Vec<Registration>,
    /// Consumer's named types in compiler namespace/nesting traversal order.
    pub source_types: Vec<TypeId>,
    /// Required symbols; absence is reported as DSTJ004.
    pub required_symbols: [Option<TypeId>; 8],
    /// Names occupied by types in the compiler's GetSymbolsWithName search scope.
    pub occupied_type_names: Vec<String>,
    /// Assembly identity table.
    pub assemblies: Vec<AssemblySymbol>,
    /// All referenced type facts needed by these semantic passes.
    pub types: Vec<TypeSymbol>,
    /// Properties and override links needed by candidate/interface hierarchies.
    pub properties: Vec<PropertySymbol>,
    /// Target-language context seed expressions.
    pub context_roots: ContextRoots,
}
