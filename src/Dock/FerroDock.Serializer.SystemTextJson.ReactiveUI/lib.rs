// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Compiled JSON metadata for the original ReactiveUI docking models.
//!
//! The catalog calls original constructors/accessors and preserves their shared identities.
//! It adds no UI dependency or replacement docking model. Application-specific subclasses
//! can compose their own compiled descriptors with [`model_type_metadata`].
mod catalog;
mod model_registration;
mod properties;
pub use catalog::model_type_metadata;
use ferrodock_model::core::SerializationResult;
use ferrodock_serializer_system_text_json::JsonTypeInfo;
use ferrodock_serializer_system_text_json::{
    DockSerializer, GeneratedJsonTypeInfoResolver, JsonListType,
};
pub use model_registration::DockJsonModelRegistration;
use std::rc::Rc;

/// Composes the standard catalog with explicitly registered application subclasses.
/// Duplicate model types, duplicate logical names and empty names return an argument error.
/// Constructors are stored as callbacks and are not invoked while building the catalog.
pub fn model_type_metadata_with_models(
    models: Vec<DockJsonModelRegistration>,
) -> SerializationResult<Vec<Rc<JsonTypeInfo>>> {
    model_registration::validate(&models)?;
    Ok(catalog::compiled_model_metadata(models))
}

/// Creates an observable-list serializer retaining the registered application classes.
/// Each class needs its concrete descriptor and original polymorphic registrations;
/// the typed registration helpers supply both over the same actual model instance.
pub fn create_serializer_with_models(
    models: Vec<DockJsonModelRegistration>,
) -> SerializationResult<Rc<DockSerializer>> {
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(
        model_type_metadata_with_models(models)?,
    )))
}

/// Creates a serializer for the standard ReactiveUI docking models and observable lists.
///
/// ```
/// use ferrodock_model::{controls::IRootDock, core::{DockSerializationError, IDockSerializer}};
/// use ferrodock_model_reactiveui::controls::RootDock;
/// use std::rc::Rc;
/// let serializer = ferrodock_serializer_system_text_json_reactiveui::create_serializer()?;
/// let layout: Rc<dyn IRootDock> = RootDock::new();
/// let json = serializer.serialize(layout)?;
/// let restored = serializer.deserialize::<Rc<dyn IRootDock>>(Some(&json))?.unwrap();
/// assert_eq!(serializer.serialize(restored)?, json);
/// # Ok::<(), DockSerializationError>(())
/// ```
pub fn create_serializer() -> SerializationResult<Rc<DockSerializer>> {
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(
        model_type_metadata(),
    )))
}

/// Creates the standard model serializer with an explicit original list creator.
/// A missing list type follows the serializer's original argument guard.
pub fn create_serializer_with_list_type(
    list_type: Option<JsonListType>,
) -> SerializationResult<Rc<DockSerializer>> {
    DockSerializer::with_list_type_and_resolver(
        list_type,
        Some(GeneratedJsonTypeInfoResolver::new(model_type_metadata())),
    )
}

#[cfg(test)]
mod registration_tests;
#[cfg(test)]
mod tests;
