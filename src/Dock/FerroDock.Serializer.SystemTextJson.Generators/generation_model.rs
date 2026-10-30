// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::TypeId;
/// Original error diagnostic emitted during semantic model construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable source code DSTJ001 through DSTJ004.
    pub id: &'static str,
    /// Source diagnostic message.
    pub message: String,
    /// Original compiler source location.
    pub location: Option<String>,
}
/// Polymorphic family, retaining the source factory order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockBaseKind {
    /// IDockable family.
    Dockable,
    /// IDock family.
    Dock,
    /// IRootDock family.
    RootDock,
    /// IDockWindow family.
    DockWindow,
    /// IDocumentTemplate family.
    DocumentTemplate,
    /// IToolTemplate family.
    ToolTemplate,
    /// Explicit ordinary object payloads.
    Object,
}
/// Original policy for unregistered runtime subclasses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownDerivedTypeHandling {
    /// Use the declared base metadata.
    FallBackToBaseType,
    /// Use the nearest registered ancestor.
    FallBackToNearestAncestor,
    /// Reject an unregistered runtime type.
    FailSerialization,
}
/// One selected class or explicitly registered payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerializableTypeModel {
    /// Original compiler symbol; no runtime class is created.
    pub type_id: TypeId,
    /// Fully qualified target expression.
    pub type_expression: String,
    /// Diagnostic display name.
    pub display_name: String,
    /// Runtime full-name discriminator, including qualified generic arguments.
    pub discriminator_key: String,
    /// Ignored names after inherited property selection.
    pub ignored_members: Vec<String>,
    /// Declaration or registration location used for diagnostics.
    pub location: Option<String>,
    /// Assignability to the six original docking contracts, in metadata-name order.
    pub contracts: [bool; 6],
    /// Explicit non-docking class registration.
    pub is_object_payload: bool,
}
/// A filtered derived-type registration for an emitted polymorphic family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedTypeModel {
    /// Compiler symbol.
    pub type_id: TypeId,
    /// Target expression.
    pub type_expression: String,
    /// Display name used by collision diagnostics.
    pub display_name: String,
    /// Logical source discriminator.
    pub discriminator_key: String,
    /// Original candidate location.
    pub location: Option<String>,
}
/// One source polymorphism options factory plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolymorphismModel {
    /// Family.
    pub kind: DockBaseKind,
    /// Target expression for the base contract.
    pub base_type_expression: String,
    /// Base name used in diagnostic text.
    pub base_display_name: &'static str,
    /// Source unknown-derived policy.
    pub unknown_derived_type_handling: UnknownDerivedTypeHandling,
    /// Source unknown-discriminator policy.
    pub ignore_unrecognized_type_discriminators: bool,
    /// Sorted types with every collision group removed.
    pub derived_types: Vec<DerivedTypeModel>,
}
/// Concrete or interface member exclusion table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnoredMembersModel {
    /// Target type expression.
    pub type_expression: String,
    /// Ordinally sorted unique property names.
    pub member_names: Vec<String>,
}
/// Semantic output consumed by context/codec/resolver emission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationModel {
    /// Source generation continues after registration/collision diagnostics, but not absent activation/contracts.
    pub should_generate: bool,
    /// Errors in source order.
    pub diagnostics: Vec<Diagnostic>,
    /// Unique context name; empty when generation is disabled.
    pub context_type_name: String,
    /// Ordinally sorted distinct context expressions.
    pub context_types: Vec<String>,
    /// Selected source and registered types.
    pub serializable_types: Vec<SerializableTypeModel>,
    /// The six docking families and object payload family.
    pub polymorphisms: Vec<PolymorphismModel>,
    /// Ignored member tables including interface unions.
    pub ignored_members: Vec<IgnoredMembersModel>,
}
impl GenerationModel {
    pub(crate) fn empty(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            should_generate: false,
            diagnostics,
            context_type_name: String::new(),
            context_types: Vec::new(),
            serializable_types: Vec::new(),
            polymorphisms: Vec::new(),
            ignored_members: Vec::new(),
        }
    }
}
/// Malformed compiler-adapter input, distinct from source registration diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilationError(pub String);
impl std::fmt::Display for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CompilationError {}
