//! The composition renderer.
//!
//! # Architecture and threading
//!
//! As upstream, there are two compositors. The [`Compositor`] lives on the
//! UI thread together with the composition objects; the
//! [`server::ServerCompositor`] owns the server-side counterparts of those
//! objects and renders them. They exchange data in one direction only,
//! through batches: at commit the UI-thread compositor serializes the
//! changes of its objects into a [`transport::CommittedBatch`] and enqueues
//! it; at the start of each frame the server compositor applies the queued
//! batches.
//!
//! The object model of the UI thread is `Rc`-based and thread-affine, so
//! the two sides are kept apart by construction rather than by locks:
//!
//! * UI-thread objects never hold their server counterpart. They hold a
//!   [`server::ServerObjectId`], a plain value. A server object is created
//!   by the server compositor itself, from a factory carried by the batch
//!   ([`Compositor::create_server_object`]); changes and disposal refer to
//!   it by id. Server objects therefore need not be `Send`: they are
//!   created, mutated and dropped on the render thread only.
//! * The batch queue ([`server::BatchQueue`]) is the only crossing point.
//!   What may cross is exactly what a [`transport::BatchObject`] can hold
//!   plus plain values.
//! * The few values the UI thread reads back (the clock, the readback
//!   indices, batch completion) are `Send + Sync` types shared through
//!   `Arc`.
//!
//! Today the server compositor runs on the thread of its compositor in
//! every configuration: the render loop may tick on any thread (its tasks
//! are `Send + Sync`), and a tick that arrives elsewhere is marshalled to
//! the compositor's dispatcher. Running the server on a dedicated render
//! thread is blocked by one thing outside this module: the platform
//! resource handles carried by batches (`Rc<dyn IGeometryImpl>`,
//! `Rc<dyn IBitmapImpl>`, `Rc<dyn IGlyphRunImpl>`, immutable brushes and
//! pens) are `Rc`-based and so are the render contracts the server draws
//! with. Once those are shareable across threads, the remaining steps are
//! local: require `Send` on the `Job`, `Create` and `Value` payloads of
//! `BatchObject`, make the batch queue a mutex-protected queue, and
//! construct the server compositor on the render thread. No `unsafe` is
//! used anywhere in the composition renderer.
//!
//! The browser target has no threads; it uses the same single-thread mode
//! with a UI-thread render timer.

pub mod animations;
pub mod brushes;
pub mod drawing;
pub mod expressions;
pub mod generated;
pub mod hit_testing;
pub mod server;
pub mod transport;

mod composition_cache_mode;
mod composition_custom_visual;
mod composition_custom_visual_handler;
mod composition_draw_list_visual;
mod composition_drawing_surface;
mod composition_external_memory;
mod composition_interop;
mod composition_experimental_acrylic_visual;
mod composition_gradient_stop;
mod composition_object;
mod composition_options;
mod composition_property_set;
mod composition_solid_color_visual;
mod composition_surface;
mod composition_surface_visual;
mod composition_target;
mod compositing_renderer;
mod compositor_factories;
mod container_visual;
mod element_composition_preview;
mod visual;
mod visual_collection;
mod composition_transparency_level;
mod compositor;
mod enums;
mod i_composition_object_host;
mod i_composition_target_debug_events;
mod i_compositor_serializable;
mod matrix_utils;

pub use composition_cache_mode::{CompositionBitmapCache, CompositionCacheMode};
pub use composition_custom_visual::CompositionCustomVisual;
pub use composition_custom_visual_handler::{CompositionCustomVisualHandler, ICompositionCustomVisualHandler};
pub use composition_drawing_surface::CompositionDrawingSurface;
pub use composition_external_memory::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionGpuImportedObject, ICompositionGpuInterop,
    ICompositionImportableSharedGpuContextImage, ICompositionImportableSharedGpuContextObject,
    ICompositionImportableSharedGpuContextSemaphore, ICompositionImportedGpuImage, ICompositionImportedGpuSemaphore,
};
pub use composition_interop::{
    CompositionGpuImportedObjectBase, CompositionImportedGpuImage, CompositionImportedGpuSemaphore, CompositionInterop,
};
pub use composition_draw_list_visual::{CompositionDrawListVisual, ICompositionDrawListVisualExtension};
pub use brushes::{
    CompositionBrush, CompositionConicGradientBrush, CompositionGradientBrush, CompositionLinearGradientBrush,
    CompositionRadialGradientBrush, CompositionSolidColorBrush,
};
pub use composition_experimental_acrylic_visual::CompositionExperimentalAcrylicVisual;
pub use composition_gradient_stop::CompositionGradientStop;
pub use composition_object::{AsCompositionObject, CompositionObject, ICompositionObjectAnimations};
pub use composition_options::CompositionOptions;
pub use composition_property_set::{CompositionGetValueStatus, CompositionPropertySet};
pub use composition_solid_color_visual::CompositionSolidColorVisual;
pub use composition_surface::CompositionSurface;
pub use composition_surface_visual::CompositionSurfaceVisual;
pub use composition_target::CompositionTarget;
pub use compositing_renderer::CompositingRenderer;
pub use container_visual::CompositionContainerVisual;
pub use element_composition_preview::ElementComposition;
pub use server::RenderSurfaces;
pub use visual::CompositionVisual;
pub use visual_collection::CompositionVisualCollection;
pub use composition_transparency_level::CompositionTransparencyLevel;
pub use compositor::{CompositionVisualSnapshotError, Compositor, ICompositorScheduler, ServerJobTask};
pub use enums::{CompositionBlendMode, CompositionGradientExtendMode, CompositionStretch, CompositionTileMode};
pub use i_composition_object_host::{ICompositionObject, ICompositionObjectHost, PendingAnimations};
pub use i_composition_target_debug_events::ICompositionTargetDebugEvents;
pub use i_compositor_serializable::ICompositorSerializable;
pub use matrix_utils::MatrixUtils;
// The key frame animation classes are generated into the namespace of the
// compositor upstream.
pub use animations::{
    BooleanKeyFrameAnimation, ColorKeyFrameAnimation, DoubleKeyFrameAnimation, QuaternionKeyFrameAnimation,
    RelativePointKeyFrameAnimation, RelativeScalarKeyFrameAnimation, ScalarKeyFrameAnimation, Vector2KeyFrameAnimation,
    Vector3DKeyFrameAnimation, Vector3KeyFrameAnimation, Vector4KeyFrameAnimation, VectorKeyFrameAnimation,
};

#[cfg(test)]
mod compositor_tests;
#[cfg(test)]
mod composition_drawing_surface_tests;
#[cfg(test)]
pub(crate) mod test_compositor;
