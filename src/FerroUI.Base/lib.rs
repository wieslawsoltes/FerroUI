//! FerroUI base library: the object model, property system, layout, input,
//! media and rendering contracts shared by every other FerroUI crate.

#[doc(hidden)]
pub use paste::paste as __paste;

pub mod animation;
pub mod collections;
pub mod controls;
pub mod data;
pub mod diagnostics;
pub mod input;
pub mod interactivity;
pub mod layout;
pub mod logging;
pub mod logical_tree;
pub mod media;
pub mod metadata;
pub mod numerics;
pub mod platform;
pub mod reactive;
pub mod rendering;
pub mod styling;
pub mod threading;
pub mod utilities;
pub mod visual_tree;

#[doc(hidden)]
pub mod property_store;

mod class_binding_manager;
mod combined_geometry;
mod corner_radius;
mod direct_property;
mod ferro_internal_exception;
mod ferro_locator;
mod ferro_object;
mod ferro_object_extensions;
mod ferro_property;
mod ferro_property_changed_event_args;
mod ferro_property_metadata;
mod ferro_property_registry;
mod i_data_context_provider;
mod i_named;
mod i_support_initialize;
mod markup_types;
mod matrix;
mod pixel_point;
mod pixel_rect;
mod pixel_size;
mod pixel_vector;
mod point;
mod rect;
mod register_types;
mod rust_paths;
mod relative_point;
mod relative_rect;
mod relative_scalar;
mod render_target_corrupted_exception;
mod render_target_not_ready_exception;
mod rotate_3d_transform;
mod rounded_rect;
mod size;
mod styled_element;
mod styled_element_extensions;
mod styled_property;
mod thickness;
mod type_system;
mod element_ref;
mod vector;
mod vector3d;
mod visual;
mod visual_tree_attachment_event_args;

pub use class_binding_manager::ClassBindingManager;
pub use corner_radius::CornerRadius;
pub use direct_property::{DirectProperty, DirectPropertyBase};
pub use ferro_internal_exception::FerroInternalException;
pub use ferro_locator::{FerroLocator, IFerroDependencyResolver, LocatorExtensions, RegistrationHelper, ServiceHandle};
pub use ferro_object::{FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroObjectVTable};
pub use ferro_object_extensions::{to_binding, FerroObjectExtensions, ObjectPropertyChangedObservable, TypedProperty};
pub use ferro_property::{
    AnyValue, BoxedValue, DoNothingType, FerroProperty, NotifyingCallback, PropertyChangedObservable,
    PropertyValue, StyledPropertyOptions, UnsetValueType,
};
pub use ferro_property_changed_event_args::{FerroPropertyChangedEventArgs, OwnedFerroPropertyChangedEventArgs};
pub use ferro_property_metadata::{
    CoerceValueCallback, DirectPropertyMetadata, FerroPropertyMetadata, StyledPropertyMetadata,
};
pub use ferro_property_registry::{FerroPropertyRegistry, Registrable};
pub use i_data_context_provider::IDataContextProvider;
pub use i_named::INamed;
pub use i_support_initialize::{ISupportInitialize, InitializationError};
pub use matrix::{Matrix, MatrixDecomposed};
pub use pixel_point::PixelPoint;
pub use pixel_rect::PixelRect;
pub use pixel_size::PixelSize;
pub use pixel_vector::PixelVector;
pub use point::Point;
pub use rect::Rect;
pub use metadata::IServiceProvider;
pub use register_types::register_types;
pub use relative_point::{RelativePoint, RelativeUnit};
pub use relative_rect::RelativeRect;
pub use relative_scalar::RelativeScalar;
pub use render_target_corrupted_exception::RenderTargetCorruptedException;
pub use render_target_not_ready_exception::RenderTargetNotReadyException;
pub use rounded_rect::RoundedRect;
pub use size::Size;
pub use styled_element::{StyledElement, StyledElementImpl, StyledElementImplExt, StyledElementVTable};
pub use styled_property::{AttachedProperty, StyledProperty};
pub use thickness::Thickness;
pub use element_ref::ElementRef;
pub use type_system::{
    __register_class, cast_this, instantiate, parent_vtable, ClassDefaults, IntoRef, Nullable, ObjectType, Ref,
    StaticType, Subclassable, TypeInfo, Upcast, WeakRef,
};
pub use vector::Vector;
pub use visual::{Visual, VisualImpl, VisualImplExt, VisualVTable};
pub use visual_tree_attachment_event_args::VisualTreeAttachmentEventArgs;
pub use vector3d::Vector3D;

#[cfg(test)]
mod ferro_object_tests;

#[cfg(test)]
mod tree_tests;
#[cfg(test)]
mod visual_render_tests;
#[cfg(test)]
mod tests;
