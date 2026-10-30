// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source JSON serializer and typed metadata, retaining the original generic contract.
mod core_model_metadata;
mod dock_json_serializable_attribute;
mod dock_json_source_generation_attribute;
mod dock_serializer;
mod enum_metadata;
mod json_value;
mod metadata;
mod number_metadata;
mod object_payload_converter;
pub use core_model_metadata::core_model_type_metadata;
pub use dock_json_serializable_attribute::DockJsonSerializableAttribute;
pub use dock_json_source_generation_attribute::DockJsonSourceGenerationAttribute;
pub use dock_serializer::DockSerializer;
use ferrodock_model::core::DockSerializationError;
pub use json_value::{JsonNumber, JsonValue};
pub use metadata::{
    GeneratedJsonTypeInfoResolver, IJsonTypeInfoResolver, JsonDerivedType, JsonListType,
    JsonPropertyInfo, JsonReadContext, JsonSerializerOptions, JsonTypeInfo,
    JsonUnknownDerivedTypeHandling, JsonValuePropertyInfo, JsonWriteContext,
};
pub use object_payload_converter::JsonObjectPayloadType;
/// JSON, metadata or argument failure from the source serialization pipeline.
#[derive(Debug)]
pub struct JsonError {
    /// Source failure category.
    pub kind: JsonErrorKind,
    /// Failure detail.
    pub message: String,
}
/// Categories retaining distinct parse, metadata, reference and argument failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonErrorKind {
    /// Invalid JSON or a value incompatible with its declared type.
    InvalidJson,
    /// Requested type has no generated metadata.
    MissingMetadata,
    /// Invalid or missing reference-preservation metadata.
    InvalidReference,
    /// A source argument was null.
    ArgumentNull,
    /// A stream does not support the required operation.
    Argument,
}
impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for JsonError {}
fn error(kind: JsonErrorKind, message: impl Into<String>) -> DockSerializationError {
    DockSerializationError::new(JsonError {
        kind,
        message: message.into(),
    })
}
#[cfg(test)]
mod accessor_failure_tests;
#[cfg(test)]
mod attribute_tests;
#[cfg(test)]
mod core_model_metadata_tests;
#[cfg(test)]
mod core_value_metadata_tests;
#[cfg(test)]
mod dock_enum_metadata_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod window_metadata_tests;

#[cfg(test)]
mod model_metadata_tests;
#[cfg(test)]
mod number_metadata_tests;
#[cfg(test)]
mod object_payload_converter_tests;
#[cfg(test)]
mod polymorphic_metadata_tests;
#[cfg(test)]
mod polymorphic_policy_tests;
