//! The server (render thread) side of the composition renderer.

pub mod dirty_rects;

mod composition_property;
mod composition_target_overlays;
mod compositor_lock;
mod compositor_pools;
mod drawing_context_proxy;

pub use compositor_lock::{CompositorLock, CompositorLockGuard, LockBound, LockedServerCompositor};
mod server_composition_bitmap_cache;
mod server_composition_cache_mode;
mod server_composition_clip;
mod server_composition_container_visual;
mod server_composition_draw_list_visual;
mod server_composition_drawing_surface;
mod server_composition_geometry;
mod server_composition_light;
mod server_composition_experimental_acrylic_visual;
mod server_composition_gradient_stop;
mod server_composition_property_set;
mod server_composition_shape;
mod server_composition_solid_color_visual;
mod server_composition_surface;
mod server_composition_surface_visual;
mod server_composition_target;
mod server_composition_view_box;
mod server_composition_visual;
mod server_composition_visual_collection;
mod server_composition_visual_surface;
mod server_compositor_animations;
mod server_custom_composition_visual;
mod server_drop_shadow;
mod server_layer_visual;
mod server_redirect_visual;
mod server_loaded_image_surface;
mod server_object_animations;
mod server_shape_visual;
mod server_size_dependant_visual;
mod server_sprite_visual;
mod server_visual_render_context;
mod diagnostic_text_renderer;
mod fps_counter;
mod frame_time_graph;
mod i_server_clock_item;
mod readback_indices;
mod server_composition_simple_brush;
mod server_composition_simple_geometry;
mod server_composition_simple_transform;
mod server_compositor;
mod server_list;
mod server_object;
mod server_property_host;
mod server_render_resource;

pub use composition_property::{props_of, CompositionProperty, CompositionPropertyOf};
pub use diagnostic_text_renderer::{
    DiagnosticGlyphRun, DiagnosticTextRenderer, GlyphTypefaceDiagnosticGlyphRunSource, IDiagnosticGlyphRunSource,
    ShapedText,
};
pub use dirty_rects::{
    DebugEventsDirtyRectCollectorProxy, IDirtyRectCollector, IDirtyRectTracker, MultiDirtyRectTracker,
    RegionDirtyRectTracker, SingleDirtyRectTracker,
};
pub use fps_counter::FpsCounter;
pub use frame_time_graph::FrameTimeGraph;
pub use i_server_clock_item::IServerClockItem;
pub use readback_indices::{ReadbackIndices, ReadbackWriteScope};
pub(crate) use server_composition_simple_brush::server_simple_brush;
pub use server_composition_simple_brush::{
    ServerCompositionSimpleBrush, ServerCompositionSimpleConicGradientBrush, ServerCompositionSimpleGradientBrush,
    ServerCompositionSimpleLinearGradientBrush, ServerCompositionSimpleRadialGradientBrush,
    ServerCompositionSimpleSolidColorBrush, ServerCompositionSimpleTileBrush,
};
pub use server_composition_simple_geometry::ServerCompositionSimpleGeometry;
// The server-side tile brushes live with the composition brushes, as
// upstream; they belong to this namespace.
pub use super::brushes::{ServerCompositionSimpleContentBrush, ServerCompositionSimpleImageBrush};
pub use server_composition_simple_transform::ServerCompositionSimpleTransform;
pub(crate) use server_compositor::OBJECT_END_MAGIC;
pub use server_compositor::{BatchQueue, CompositorClock, ServerCompositor};
pub use server_list::ServerList;
pub use composition_target_overlays::CompositionTargetOverlays;
pub use compositor_pools::{CompositorPools, StackPool};
pub use drawing_context_proxy::CompositorDrawingContextProxy;
pub use server_composition_bitmap_cache::ServerCompositionBitmapCache;
pub use server_composition_cache_mode::ServerCompositionCacheMode;
pub use server_composition_container_visual::ServerCompositionContainerVisual;
pub use server_composition_draw_list_visual::ServerCompositionDrawListVisual;
pub use server_composition_drawing_surface::{DrawingSurfaceUpdateError, ServerCompositionDrawingSurface};
pub use server_composition_experimental_acrylic_visual::ServerCompositionExperimentalAcrylicVisual;
pub use server_composition_gradient_stop::ServerCompositionGradientStop;
pub(crate) use server_composition_property_set::PropertySetChanges;
pub use server_composition_property_set::ServerCompositionPropertySet;
// The server-side composition brushes are in the namespace of the server
// objects upstream.
pub use super::brushes::{
    ServerCompositionConicGradientBrush, ServerCompositionGradientBrush, ServerCompositionLinearGradientBrush,
    ServerCompositionRadialGradientBrush, ServerCompositionSolidColorBrush,
};
pub use server_composition_solid_color_visual::ServerCompositionSolidColorVisual;
pub use server_composition_surface::{IServerCompositionSurface, ServerCompositionSurfaceChanged};
pub use server_composition_surface_visual::ServerCompositionSurfaceVisual;
pub use server_composition_target::{RenderSurfaces, ServerCompositionTarget};
pub use server_composition_visual::{
    IServerVisualContent, ReadbackData, ServerCompositionVisual,
    ServerCompositionVisualCache, TreeWalkerFrame, VisualReadback,
};
pub use server_composition_visual_collection::ServerCompositionVisualCollection;
pub use server_composition_clip::{
    composition_transform, inset_rect, CompositionClipShape, ServerGeometricClip, ServerInsetClip, ServerRectangleClip,
};
pub use server_composition_geometry::{
    platform_geometry_of, server_geometry_of, trim_geometry, IServerCompositionGeometry,
    ServerCompositionEllipseGeometry, ServerCompositionLineGeometry, ServerCompositionPathGeometry,
    ServerCompositionRectangleGeometry, ServerCompositionRoundedRectangleGeometry,
};
pub use server_composition_shape::{
    render_shapes, shape_transform, CompositionShapePen, ServerCompositionContainerShape,
    ServerCompositionShapeCollection, ServerCompositionSpriteShape,
};
pub use server_composition_view_box::{view_box_mapping, ServerCompositionViewBox, ViewBoxMapping};
pub use server_shape_visual::ServerShapeVisual;
pub use server_composition_visual_surface::ServerCompositionVisualSurface;
pub use server_drop_shadow::ServerDropShadow;
pub use server_composition_light::{
    distant_light_incidence, light_map_of, point_light_incidence, spot_light_cone, IServerCompositionLight,
    LightAttenuation, LightColor, LightMap, LightSource, LitContent, ServerAmbientLight, ServerDistantLight,
    ServerLightState, ServerPointLight, ServerSpotLight, ServerVisualUnorderedCollection, LIGHT_MAP_CELL,
    LIGHT_MAP_MAX_CELLS,
};
pub use server_loaded_image_surface::ServerLoadedImageSurface;
pub use server_sprite_visual::ServerSpriteVisual;
pub use server_layer_visual::{drop_shadow_effect, ServerLayerVisual};
pub use server_redirect_visual::ServerRedirectVisual;
// The server-side surface brush lives with the composition brushes.
pub use super::brushes::{paint_composition_brush, surface_brush_content_rect, ServerCompositionSurfaceBrush};
// The server-side nine-grid and mask brushes live with the composition
// brushes.
// The server-side effect and backdrop brushes live with the composition
// brushes.
pub use super::brushes::{ServerCompositionBackdropBrush, ServerCompositionEffectBrush};
pub use super::brushes::{
    nine_grid_rects, NineGridEdges, ServerCompositionMaskBrush, ServerCompositionNineGridBrush, NINE_GRID_CENTER,
};
pub use server_compositor_animations::ServerCompositorAnimations;
pub use server_custom_composition_visual::ServerCompositionCustomVisual;
pub use server_object::{
    impl_animated_server_object, IAnimatedServerObject, IServerObject, ServerExpressionObject, ServerObject, ServerObjectId,
};
pub use server_object_animations::ServerObjectAnimations;
pub use server_size_dependant_visual::ServerSizeDependantVisual;
pub use server_visual_render_context::ServerVisualRenderContext;
pub use server_property_host::{
    read_resource, read_server_object, resolve_resource, AsServerRenderResource, IServerAnimatedPropertyHost,
    IServerPropertyHost, ServerPropertyValue, ServerResourceRef, ServerValueChange,
};
pub use server_render_resource::{
    impl_server_render_resource, impl_simple_server_render_resource, IServerRenderResource,
    IServerRenderResourceHost, IServerRenderResourceObserver, ServerRenderResource, ServerRenderResourceCore,
    SimpleServerRenderResource,
};
