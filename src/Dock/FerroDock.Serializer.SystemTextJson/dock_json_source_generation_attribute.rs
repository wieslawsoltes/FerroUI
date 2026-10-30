// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use std::rc::Rc;

/// Activates source-generated JSON serialization for the declaring assembly.
pub struct DockJsonSourceGenerationAttribute {
    _private: (),
}
impl DockJsonSourceGenerationAttribute {
    /// Creates the source activation attribute.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { _private: () })
    }
}

/// Declares the crate's source-generation activation marker.
///
/// Invoke at crate scope and pass the source to the build-time generator adapter.
/// The fixed marker name rejects repeated activation in the same scope. The
/// declaration retains a typed attribute constructor without creating runtime state.
///
/// ```compile_fail
/// ferrodock_serializer_system_text_json::dock_json_source_generation!();
/// ferrodock_serializer_system_text_json::dock_json_source_generation!();
/// ```
#[macro_export]
macro_rules! dock_json_source_generation {
    () => {
        const _DOCK_JSON_SOURCE_GENERATION_ATTRIBUTE: fn() -> ::std::rc::Rc<
            $crate::DockJsonSourceGenerationAttribute,
        > = $crate::DockJsonSourceGenerationAttribute::new;
    };
}
