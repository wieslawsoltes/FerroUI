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
//! resource handles carried by batches (`std::sync::Arc<dyn IGeometryImpl>`,
//! `std::sync::Arc<crate::platform::SharedBitmapImpl>`, `std::sync::Arc<dyn IGlyphRunImpl>`, immutable brushes and
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
pub mod effects;
pub mod expressions;
pub mod generated;
pub mod hit_testing;
pub mod interactions;
pub mod server;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod transport;

mod composition_batch;
mod composition_cache_mode;
mod composition_clip;
mod composition_custom_visual;
mod composition_custom_visual_handler;
mod composition_draw_list_visual;
mod composition_drawing_surface;
mod composition_external_memory;
mod composition_interop;
mod composition_experimental_acrylic_visual;
mod composition_geometry;
mod composition_light;
mod composition_gradient_stop;
mod composition_object;
mod composition_object_impls;
mod composition_options;
mod composition_property_set;
mod composition_shape;
mod composition_shape_factories;
mod composition_solid_color_visual;
mod composition_surface;
mod composition_surface_visual;
mod composition_target;
mod composition_view_box;
mod composition_visual_surface;
mod composition_winui_factories;
mod composition_visual_winui;
mod composition_winui_enums;
mod composition_winui_layer_factories;
mod layer_visual;
mod redirect_visual;
mod compositing_renderer;
mod compositor_factories;
mod container_visual;
mod drop_shadow;
mod i_composition_surface;
mod loaded_image_surface;
mod shape_visual;
mod sprite_visual;
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
mod render_interface_feature;

pub use composition_batch::{
    AnimationCompletion, BatchCompletionGuard, CompositionBatchCompletedEventArgs, CompositionBatchTypes,
    CompositionCommitBatch, CompositionScopedBatch,
};
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
    IServerGpuImportedObject, ServerGpuImportedObjectBase, ServerImportedGpuImage,
    ServerImportedGpuSemaphore,
};
pub use composition_draw_list_visual::{CompositionDrawListVisual, ICompositionDrawListVisualExtension};
pub use brushes::{
    CompositionBrush, CompositionConicGradientBrush, CompositionGradientBrush, CompositionLinearGradientBrush,
    CompositionRadialGradientBrush, CompositionSolidColorBrush, CompositionSurfaceBrush,
};
pub use composition_clip::{CompositionClip, GeometricClip, InsetClip, RectangleClip};
pub use composition_geometry::{
    CompositionEllipseGeometry, CompositionGeometry, CompositionLineGeometry, CompositionPath, CompositionPathGeometry,
    CompositionRectangleGeometry, CompositionRoundedRectangleGeometry,
};
pub use composition_shape::{
    CompositionContainerShape, CompositionShape, CompositionShapeCollection, CompositionSpriteShape, CompositionStrokeCap,
    CompositionStrokeLineJoin,
};
pub use composition_view_box::CompositionViewBox;
pub use shape_visual::ShapeVisual;
pub use brushes::{CompositionMaskBrush, CompositionNineGridBrush};
// The WinUI effect brushes (phase 6); the effect descriptions and the
// factory are in `effects`.
pub use brushes::{CompositionBackdropBrush, CompositionEffectBrush};
pub use effects::{
    CompositionEffectFactory, CompositionEffectFactoryError, CompositionEffectFactoryLoadStatus,
    CompositionEffectSourceParameter,
};
pub use composition_winui_enums::{
    CompositionBackfaceVisibility, CompositionBorderMode, CompositionCompositeMode, CompositionMappingMode,
};
pub use layer_visual::LayerVisual;
pub use redirect_visual::RedirectVisual;
pub use composition_visual_surface::CompositionVisualSurface;
pub use drop_shadow::DropShadow;
pub use composition_light::{
    AmbientLight, CompositionLight, DistantLight, PointLight, SpotLight, VisualUnorderedCollection,
};
pub use i_composition_surface::ICompositionSurface;
pub use loaded_image_surface::{LoadedImageSourceLoadStatus, LoadedImageSurface};
pub use sprite_visual::SpriteVisual;
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
pub use compositor::{
    CompositionVisualSnapshotError, Compositor, ICompositorScheduler, LocalFuture, ServerJobError, ServerJobTask,
};
pub use enums::{
    CompositionBitmapInterpolationMode, CompositionBlendMode, CompositionDropShadowSourcePolicy,
    CompositionGradientExtendMode, CompositionStretch, CompositionTileMode,
};
pub use i_composition_object_host::{ICompositionObject, ICompositionObjectHost, PendingAnimations};
pub use i_composition_target_debug_events::ICompositionTargetDebugEvents;
pub use i_compositor_serializable::ICompositorSerializable;
pub use matrix_utils::MatrixUtils;
pub use render_interface_feature::RenderInterfaceFeature;
// The key frame animation classes are generated into the namespace of the
// compositor upstream.
pub use animations::{
    BooleanKeyFrameAnimation, ColorKeyFrameAnimation, DoubleKeyFrameAnimation, QuaternionKeyFrameAnimation,
    RelativePointKeyFrameAnimation, RelativeScalarKeyFrameAnimation, ScalarKeyFrameAnimation, Vector2KeyFrameAnimation,
    Vector3DKeyFrameAnimation, Vector3KeyFrameAnimation, Vector4KeyFrameAnimation, VectorKeyFrameAnimation,
};
// The WinUI animation types of `Microsoft.UI.Composition` that are not in the
// schema: the natural motion animations, the animation controller and the
// colour space of colour animations.
pub use animations::{
    AnimationController, AnimationControllerProgressBehavior, CompositionColorSpace, NaturalMotionAnimation,
    ScalarNaturalMotionAnimation, SpringScalarNaturalMotionAnimation, SpringVector2NaturalMotionAnimation,
    SpringVector3NaturalMotionAnimation, Vector2NaturalMotionAnimation, Vector3NaturalMotionAnimation,
};

#[cfg(test)]
mod composition_property_set_tests;
#[cfg(test)]
mod composition_batch_tests;
#[cfg(test)]
mod compositor_tests;
#[cfg(test)]
mod render_thread_tests;
#[cfg(test)]
mod composition_drawing_surface_tests;
#[cfg(test)]
pub(crate) mod test_compositor;
