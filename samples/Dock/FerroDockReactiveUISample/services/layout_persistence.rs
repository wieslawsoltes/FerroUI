// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Framework-independent layout serialization and initialization boundaries.
use ferrodock_model::{controls::IRootDock, core::SerializationResult};
use std::rc::Rc;

/// Serializes the original layout graph without view or file-selection dependencies.
pub trait ILayoutSerialization {
    /// Writes ordinary JSON, retaining a null layout distinctly from a failure.
    fn serialize(&self, layout: Option<Rc<dyn IRootDock>>) -> SerializationResult<String>;
    /// Reads original model instances; malformed JSON leaves the current layout untouched.
    fn deserialize(&self, json: &str) -> SerializationResult<Option<Rc<dyn IRootDock>>>;
    /// Saves through the standard serializer's stream API, including its encoding preamble.
    fn save_bytes(&self, layout: Option<Rc<dyn IRootDock>>) -> SerializationResult<Vec<u8>>;
    /// Loads through the standard serializer's encoding-aware stream API.
    fn load_bytes(&self, bytes: Vec<u8>) -> SerializationResult<Option<Rc<dyn IRootDock>>>;
}

/// Reconnects a loaded graph to the application's actual factory and live locators.
pub trait ILayoutInitialization {
    /// Initializes the restored graph through the original factory, or clears live
    /// layout locators when the serialized layout is null.
    fn initialize(&self, layout: Option<Rc<dyn IRootDock>>);
}
