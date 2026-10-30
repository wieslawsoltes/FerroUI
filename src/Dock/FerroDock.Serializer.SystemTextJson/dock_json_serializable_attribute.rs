// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use std::{any::TypeId, rc::Rc};

/// Registers an additional type for source-generated JSON serialization.
pub struct DockJsonSerializableAttribute {
    type_: Option<TypeId>,
}
impl DockJsonSerializableAttribute {
    /// Retains the supplied type identity, including the source runtime's null value.
    pub fn new(type_: Option<TypeId>) -> Rc<Self> {
        Rc::new(Self { type_ })
    }
    /// Gets the type supplied to the constructor.
    pub fn type_(&self) -> Option<TypeId> {
        self.type_
    }
}

/// Registers a logical Rust type in the declaring crate's generated JSON context.
///
/// Repeated registrations are permitted and deduplicated by the semantic generator.
/// The Rust compiler checks the named type; build-time discovery resolves its source
/// symbol. The marker does not install a runtime registry or construct the type.
#[macro_export]
macro_rules! dock_json_serializable {
    ($type:ty) => {
        const _: fn() -> ::std::rc::Rc<$crate::DockJsonSerializableAttribute> =
            || $crate::DockJsonSerializableAttribute::new(Some(::std::any::TypeId::of::<$type>()));
    };
}
