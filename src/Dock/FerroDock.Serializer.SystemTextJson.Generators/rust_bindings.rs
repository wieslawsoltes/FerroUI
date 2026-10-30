// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{DockBaseKind, TypeId};
/// Resolved source-property accessors in the target compiler's Rust syntax.
#[derive(Clone, Debug)]
pub struct RustPropertyBinding {
    /// Effective JSON property name, after source naming attributes.
    pub name: String,
    /// Exact Rust property value type, including nullable/shared handles.
    pub value_type: String,
    /// Original getter expression, absent for a set-only property.
    pub getter: Option<String>,
    /// Original setter expression, absent for a get-only property.
    pub setter: Option<String>,
    /// The getter returns SerializationResult rather than the property value directly.
    pub getter_fallible: bool,
    /// The setter returns SerializationResult rather than unit directly.
    pub setter_fallible: bool,
    /// Explicit custom metadata factory expression; takes precedence over object conversion.
    pub converter: Option<String>,
    /// Source property type is System.Object and receives the generated payload converter.
    pub object_payload: bool,
}
/// One extra compiled reference conversion for an original shared handle.
#[derive(Clone, Debug)]
pub struct RustReferenceCast {
    /// Rust target handle type.
    pub target: String,
    /// Closure converting a borrowed source handle to the same target instance.
    pub convert: String,
}
/// Codec binding emitted by the syntax/metadata adapter for a selected source type.
#[derive(Clone, Debug)]
pub struct RustTypeBinding {
    /// Semantic symbol index.
    pub symbol: TypeId,
    /// Original class/value handle type in Rust.
    pub value_type: String,
    /// Class constructor expression; absent when no supported constructor exists.
    pub constructor: Option<String>,
    /// Reference identity callback for class metadata.
    pub identity: String,
    /// Resolved properties in source serialization order.
    pub properties: Vec<RustPropertyBinding>,
    /// Custom codec factory for source value types or type-level converters.
    pub custom_codec: Option<String>,
    /// Additional reference conversions not supplied by polymorphic families.
    pub reference_casts: Vec<RustReferenceCast>,
}
/// Casts for one original derived class within a declared family.
#[derive(Clone, Debug)]
pub struct RustDerivedBinding {
    /// Derived class symbol index.
    pub symbol: TypeId,
    /// Checked class/view cast from a borrowed base handle, preserving the model instance.
    pub downcast: String,
    /// Upcast from an owned derived handle, preserving the model instance.
    pub upcast: String,
}
/// Declared family schema and the compiled runtime class/cast facts it uses.
#[derive(Clone, Debug)]
pub struct RustFamilyBinding {
    /// One of the six docking families; object payloads are generated separately.
    pub kind: DockBaseKind,
    /// Base interface or class handle type.
    pub value_type: String,
    /// Runtime handle TypeId callback used for exact/unknown derived dispatch.
    pub runtime_type: Option<String>,
    /// Identity callback for base fallback metadata.
    pub identity: Option<String>,
    /// Actual declared base getters/setters in source order.
    pub properties: Vec<RustPropertyBinding>,
    /// Typed casts for the family's registered classes.
    pub derived: Vec<RustDerivedBinding>,
}
/// Target compiler bindings consumed by Rust context/resolver emission.
#[derive(Clone, Debug)]
pub struct RustBindings {
    /// Absolute Rust path to the serializer runtime crate.
    pub runtime_crate: String,
    /// Absolute Rust path to the UI-independent docking model crate.
    pub model_crate: String,
    /// Selected concrete source type bindings.
    pub types: Vec<RustTypeBinding>,
    /// Six original family bindings.
    pub families: Vec<RustFamilyBinding>,
    /// Compiled factories for the context closure: nullable/list/value/custom support types.
    pub additional_codecs: Vec<String>,
    /// Explicit compiled metadata collections needed by the selected model fields.
    pub additional_metadata_collections: Vec<String>,
}
