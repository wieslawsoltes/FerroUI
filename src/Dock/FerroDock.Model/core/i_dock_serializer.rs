// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::SerializationResult;
/// Source System.IO.Stream, retaining shared identity, capabilities and explicit disposal.
pub use crate::io::Stream as DockStream;
use std::rc::Rc;

/// Generic docking serializer contract. Implementations retain the requested Rust type.
/// Native generic dispatch is static; consumers such as DockWorkspaceManager carry
/// the serializer type rather than replacing serialization with a layout-only API.
pub trait IDockSerializer: 'static {
    /// Serializes the supplied value, retaining its declared type.
    fn serialize<T: 'static>(&self, value: T) -> SerializationResult<String>;
    /// Deserializes the requested type; a source null result differs from a failure.
    fn deserialize<T: 'static>(&self, text: Option<&str>) -> SerializationResult<Option<T>>;
    /// Loads the requested type from the original nullable stream reference.
    /// Concrete serializers retain the source reader/disposal and failure behavior.
    fn load<T: 'static>(
        &self,
        stream: Option<Rc<dyn DockStream>>,
    ) -> SerializationResult<Option<T>>;
    /// Saves the supplied value to the original nullable stream reference.
    /// Concrete serializers may close that same stream, as their source implementation does.
    fn save<T: 'static>(
        &self,
        stream: Option<Rc<dyn DockStream>>,
        value: T,
    ) -> SerializationResult<()>;
}
