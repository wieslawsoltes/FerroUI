//! Platform abstraction contracts and their managed default implementations.

pub mod surfaces;

mod alpha_format;
mod i_bitmap_impl;
mod i_drawing_context_impl;
mod i_glyph_run_impl;
mod i_locked_framebuffer;
mod i_optional_feature_provider;
mod i_scoped_resource;
mod i_surface_orientation;
mod platform_graphics_device_adapter_description;
mod surface_orientation;
mod i_platform_behavior_inhibition;
mod i_external_objects_render_interface_context_feature;
mod i_platform_gpu;
mod platform_graphics_external_memory;
mod i_platform_render_interface;
mod i_platform_render_interface_region;
mod i_platform_threading_interface;
mod i_render_target;
mod ltrb_rect;
mod pixel_format;
mod retained_framebuffer;
mod managed_dispatcher_impl;
mod system_navigation_manager_impl;

pub use alpha_format::AlphaFormat;
pub use i_bitmap_impl::{IBitmapImpl, IReadableBitmapImpl, IRenderTargetBitmapImpl, IWriteableBitmapImpl, SharedBitmapImpl};
pub use i_drawing_context_impl::{
    IDrawingContextImpl, IDrawingContextLayerImpl, IDrawingContextLayerWithRenderContextAffinityImpl,
};
pub use i_glyph_run_impl::IGlyphRunImpl;
pub use i_locked_framebuffer::ILockedFramebuffer;
pub use i_optional_feature_provider::IOptionalFeatureProvider;
pub use i_scoped_resource::{IScopedResource, ScopedResource};
pub use i_surface_orientation::ISurfaceOrientation;
pub use platform_graphics_device_adapter_description::PlatformGraphicsDeviceAdapterDescription;
pub use surface_orientation::SurfaceOrientation;
pub use i_platform_behavior_inhibition::IPlatformBehaviorInhibition;
pub use i_external_objects_render_interface_context_feature::{
    IExternalObjectsHandleWrapRenderInterfaceContextFeature, IExternalObjectsRenderInterfaceContextFeature,
    IExternalObjectsWrappedGpuHandle, IPlatformRenderInterfaceImportedImage, IPlatformRenderInterfaceImportedObject,
    IPlatformRenderInterfaceImportedSemaphore,
};
pub use i_platform_gpu::{
    IPlatformGraphics, IPlatformGraphicsContext, IPlatformGraphicsReadyStateFeature, PlatformGraphicsContextLostException,
};
pub use platform_graphics_external_memory::{
    KnownPlatformGraphicsExternalImageHandleTypes, KnownPlatformGraphicsExternalSemaphoreHandleTypes,
    PlatformGraphicsDrmFormat, PlatformGraphicsExternalImageDmaBufProperties, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties, PlatformGraphicsExternalImageVulkanProperties,
};
pub use i_platform_render_interface::{IPlatformRenderInterface, IPlatformRenderInterfaceContext};
pub use i_platform_render_interface_region::IPlatformRenderInterfaceRegion;
pub use i_render_target::{
    IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties, RenderTargetProperties,
    RenderTargetSceneInfo,
};
pub use system_navigation_manager_impl::ISystemNavigationManagerImpl;
pub use ltrb_rect::{LtrbPixelRect, LtrbRect};
#[doc(hidden)]
pub use pixel_format::PixelFormatEnum;
pub use pixel_format::{PixelFormat, PixelFormats};
pub use retained_framebuffer::RetainedFramebuffer;
pub use i_platform_threading_interface::{IPlatformThreadingInterface, PlatformTimerHandle};
#[cfg(not(target_family = "wasm"))]
pub use managed_dispatcher_impl::{IManagedDispatcherInputProvider, ManagedDispatcherImpl};

// --- geometry contracts ---

mod i_geometry_context;
mod i_geometry_impl;
mod i_stream_geometry_context_impl;
mod i_stream_geometry_impl;
mod i_transformed_geometry_impl;
mod path_geometry_context;
mod render_interface_access;

pub use i_geometry_context::IGeometryContext;
pub use i_geometry_impl::IGeometryImpl;
pub use i_stream_geometry_context_impl::IStreamGeometryContextImpl;
pub use i_stream_geometry_impl::IStreamGeometryImpl;
pub use i_transformed_geometry_impl::ITransformedGeometryImpl;
pub use path_geometry_context::PathGeometryContext;
pub use render_interface_access::render_interface;

// --- input contracts ---

mod default_platform_settings;
mod i_cursor_factory;
mod i_cursor_impl;
mod i_mac_os_top_level_platform_handle;
mod i_platform_handle;
mod i_platform_settings;
mod platform_color_values;
mod platform_handle;

pub use default_platform_settings::DefaultPlatformSettings;
pub use i_mac_os_top_level_platform_handle::IMacOSTopLevelPlatformHandle;
pub use i_platform_handle::IPlatformHandle;
pub use platform_color_values::{ColorContrastPreference, PlatformColorValues, PlatformThemeVariant};
pub use platform_handle::PlatformHandle;
pub use i_cursor_factory::ICursorFactory;
pub use i_cursor_impl::ICursorImpl;
pub use i_platform_settings::IPlatformSettings;

// --- effects, materials and assets ---

mod asset_loader;
mod i_asset_loader;
mod i_drawing_context_impl_with_effects;
mod i_drawing_context_with_acrylic_like_support;
mod internal;
mod i_runtime_platform;
mod standard_asset_loader;
mod standard_runtime_platform;
mod standard_runtime_platform_services;

pub use asset_loader::{AssetLoader, ASSET_SCHEME};
pub use i_asset_loader::{AssetAssembly, AssetStream, IAssetLoader};
pub use i_runtime_platform::{FormFactorType, IRuntimePlatform, RuntimePlatformInfo};
pub use standard_runtime_platform::StandardRuntimePlatform;
pub use standard_runtime_platform_services::StandardRuntimePlatformServices;
pub use i_drawing_context_impl_with_effects::IDrawingContextImplWithEffects;
pub use i_drawing_context_with_acrylic_like_support::IDrawingContextWithAcrylicLikeSupport;
pub use internal::{register_assets, register_manifest_resources};
pub use standard_asset_loader::StandardAssetLoader;

// --- font and text contracts ---

mod i_font_manager_impl;
mod i_text_shaper_impl;

pub use i_font_manager_impl::IFontManagerImpl;
pub use i_text_shaper_impl::ITextShaperImpl;

// --- storage contracts ---

pub mod storage;
