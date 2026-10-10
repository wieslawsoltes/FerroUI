//! The brushes, the visuals and the clip of the blur effects of the
//! Windows.UI.Composition mode.

use super::win_ui_composition_shared::WinUiCompositionShared;
use super::win_ui_effect_base::{
    BlendEffect, ColorSourceEffect, OpacityEffect, SaturationEffect, WinUIEffectBase, WinUIGaussianBlurEffect,
};
use super::Required;
use crate::win32_platform::Win32Platform;
use crate::win_rt::numerics::Vector2;
use crate::win_rt::{
    HStringInterop, ICompositionBrush, ICompositionClip, ICompositionEffectSourceParameterFactory, ICompositionGeometry,
    ICompositionRoundedRectangleGeometry, ICompositor, ICompositor2, ICompositor3, ICompositor5, ICompositor6,
    ICompositorWithBlurredWallpaperBackdropBrush, IGraphicsEffectSource, IVisual, IVisual2, NativeWinRTMethods,
};
use ferroui_microcom::{ComPtr, HResult};

pub(crate) struct WinUiCompositionUtils;

impl WinUiCompositionUtils {
    /// The brush of the mica effect for a tint (a grey level of 0 to 255)
    /// and a luminosity opacity; `None` before Windows 11.
    pub fn create_mica_backdrop_brush(
        compositor: &ComPtr<ICompositor>,
        color: f32,
        opacity: f32,
    ) -> Result<Option<ComPtr<ICompositionBrush>>, HResult> {
        if Win32Platform::windows_version().build < 22000 {
            return Ok(None);
        }

        let back_drop_parameter_factory = NativeWinRTMethods::create_activation_factory::<ICompositionEffectSourceParameterFactory>(
            "Windows.UI.Composition.CompositionEffectSourceParameter",
        )?;

        let tint = vec![color / 255.0, color / 255.0, color / 255.0, 255.0 / 255.0];

        let tint_color_effect = WinUIEffectBase::new(ColorSourceEffect::new(tint.clone()), &[]);
        let tint_color_effect_source = tint_color_effect.cast::<IGraphicsEffectSource>()?;

        let tint_opacity_effect = WinUIEffectBase::new(OpacityEffect::new(1.0), &[&tint_color_effect_source]);
        let tint_opacity_effect_factory = compositor.create_effect_factory(Some(&tint_opacity_effect)).required()?;
        let tint_opacity_effect_brush_effect = tint_opacity_effect_factory.create_brush().required()?;
        let tint_opacity_effect_brush = tint_opacity_effect_brush_effect.cast::<ICompositionBrush>()?;

        let luminosity_color_effect = WinUIEffectBase::new(ColorSourceEffect::new(tint), &[]);
        let luminosity_color_effect_source = luminosity_color_effect.cast::<IGraphicsEffectSource>()?;

        let luminosity_opacity_effect = WinUIEffectBase::new(OpacityEffect::new(opacity), &[&luminosity_color_effect_source]);
        let luminosity_opacity_effect_factory = compositor.create_effect_factory(Some(&luminosity_opacity_effect)).required()?;
        let luminosity_opacity_effect_brush_effect = luminosity_opacity_effect_factory.create_brush().required()?;
        let luminosity_opacity_effect_brush = luminosity_opacity_effect_brush_effect.cast::<ICompositionBrush>()?;

        let compositor_with_blurred_wallpaper_backdrop_brush =
            compositor.cast::<ICompositorWithBlurredWallpaperBackdropBrush>()?;
        let blurred_wallpaper_backdrop_brush =
            compositor_with_blurred_wallpaper_backdrop_brush.try_create_blurred_wallpaper_backdrop_brush()?;
        let mica_backdrop_brush =
            blurred_wallpaper_backdrop_brush.map(|brush| brush.cast::<ICompositionBrush>()).transpose()?;

        let (background_parameter_as_source, background_handle) =
            Self::get_parameter_source("Background", &back_drop_parameter_factory)?;
        let (foreground_parameter_as_source, foreground_handle) =
            Self::get_parameter_source("Foreground", &back_drop_parameter_factory)?;

        let luminosity_blend_effect =
            WinUIEffectBase::new(BlendEffect::new(23), &[&background_parameter_as_source, &foreground_parameter_as_source]);
        let luminosity_blend_effect_factory = compositor.create_effect_factory(Some(&luminosity_blend_effect)).required()?;
        let luminosity_blend_effect_brush = luminosity_blend_effect_factory.create_brush().required()?;
        let luminosity_blend_effect_brush1 = luminosity_blend_effect_brush.cast::<ICompositionBrush>()?;
        luminosity_blend_effect_brush.set_source_parameter(background_handle.handle(), mica_backdrop_brush.as_deref())?;
        luminosity_blend_effect_brush
            .set_source_parameter(foreground_handle.handle(), Some(&luminosity_opacity_effect_brush))?;

        let (background_parameter_as_source1, background_handle1) =
            Self::get_parameter_source("Background", &back_drop_parameter_factory)?;
        let (foreground_parameter_as_source1, foreground_handle1) =
            Self::get_parameter_source("Foreground", &back_drop_parameter_factory)?;

        let color_blend_effect =
            WinUIEffectBase::new(BlendEffect::new(22), &[&background_parameter_as_source1, &foreground_parameter_as_source1]);
        let color_blend_effect_factory = compositor.create_effect_factory(Some(&color_blend_effect)).required()?;
        let color_blend_effect_brush = color_blend_effect_factory.create_brush().required()?;
        color_blend_effect_brush.set_source_parameter(background_handle1.handle(), Some(&luminosity_blend_effect_brush1))?;
        color_blend_effect_brush.set_source_parameter(foreground_handle1.handle(), Some(&tint_opacity_effect_brush))?;

        // colorBlendEffectBrush.SetSourceParameter(backgroundHandle, micaBackdropBrush);

        let mica_backdrop_brush1 = color_blend_effect_brush.cast::<ICompositionBrush>()?;
        Ok(Some(mica_backdrop_brush1))
    }

    /// The brush of the acrylic effect: the backdrop of the window,
    /// blurred.
    pub fn create_acrylic_blur_backdrop_brush(compositor: &ComPtr<ICompositor>) -> Result<ComPtr<ICompositionBrush>, HResult> {
        let back_drop_parameter_factory = NativeWinRTMethods::create_activation_factory::<ICompositionEffectSourceParameterFactory>(
            "Windows.UI.Composition.CompositionEffectSourceParameter",
        )?;
        let backdrop_string = HStringInterop::new(Some("backdrop"))?;
        let back_drop_parameter = back_drop_parameter_factory.create(backdrop_string.handle()).required()?;
        let back_drop_parameter_as_source = back_drop_parameter.cast::<IGraphicsEffectSource>()?;
        let blur_effect = WinUIEffectBase::new(WinUIGaussianBlurEffect, &[&back_drop_parameter_as_source]);

        // The reference leases the native pointer of the blur effect here,
        // because the system releases the reference `GetSource` returned
        // and uses the object afterwards. Here the saturation effect below
        // holds a counted reference to the blur effect and this function
        // holds one until it returns (`WinUIEffectBase::new`).
        let blur_effect_source = blur_effect.cast::<IGraphicsEffectSource>()?;

        let blur_effect_factory = compositor.create_effect_factory(Some(&blur_effect)).required()?;
        let composition_effect_brush = blur_effect_factory.create_brush().required()?;
        let backdrop_brush = Self::create_backdrop_brush(compositor)?;

        // As in the reference, the brush of the saturation effect is made
        // and not used: the brush of the mode is the blur alone.
        let saturate_effect = WinUIEffectBase::new(SaturationEffect, &[&blur_effect_source]);
        let sat_effect_factory = compositor.create_effect_factory(Some(&saturate_effect)).required()?;
        let _sat = sat_effect_factory.create_brush().required()?;
        composition_effect_brush.set_source_parameter(backdrop_string.handle(), Some(&backdrop_brush))?;
        composition_effect_brush.cast::<ICompositionBrush>()
    }

    /// Clips the visuals to a rectangle with rounded corners; `None`
    /// without a radius. The caller sizes the geometry it gets.
    pub fn clip_visual(
        compositor: &ComPtr<ICompositor>,
        backdrop_corner_radius: Option<f32>,
        container_visuals: &[Option<&IVisual>],
    ) -> Result<Option<ComPtr<ICompositionRoundedRectangleGeometry>>, HResult> {
        let Some(backdrop_corner_radius) = backdrop_corner_radius else {
            return Ok(None);
        };
        let compositor5 = compositor.cast::<ICompositor5>()?;
        let rounded_rectangle_geometry = compositor5.create_rounded_rectangle_geometry().required()?;
        rounded_rectangle_geometry.set_corner_radius(Vector2 { x: backdrop_corner_radius, y: backdrop_corner_radius })?;

        let compositor6 = compositor.cast::<ICompositor6>()?;
        let composition_geometry = rounded_rectangle_geometry.cast::<ICompositionGeometry>()?;

        let geometric_clip_with_geometry =
            compositor6.create_geometric_clip_with_geometry(Some(&composition_geometry)).required()?;
        for visual in container_visuals.iter().flatten() {
            let clip = geometric_clip_with_geometry.cast::<ICompositionClip>()?;
            visual.set_clip(Some(&clip))?;
        }

        Ok(Some(rounded_rectangle_geometry))
    }

    /// A visual that fills its parent with the brush, hidden until its
    /// effect is the one of the window.
    pub fn create_blur_visual(
        compositor: &ComPtr<ICompositor>,
        composition_brush: &ICompositionBrush,
    ) -> Result<ComPtr<IVisual>, HResult> {
        let sprite_visual = compositor.create_sprite_visual().required()?;
        let visual = sprite_visual.cast::<IVisual>()?;
        let visual2 = sprite_visual.cast::<IVisual2>()?;

        sprite_visual.set_set_brush(Some(composition_brush))?;
        visual.set_is_visible(0)?;
        visual2.set_relative_size_adjustment(Vector2 { x: 1.0, y: 1.0 })?;

        Ok(visual)
    }

    /// The brush of what is behind the window: the host backdrop on
    /// Windows 11, the backdrop of the compositor before.
    pub fn create_backdrop_brush(compositor: &ComPtr<ICompositor>) -> Result<ComPtr<ICompositionBrush>, HResult> {
        let brush = if Win32Platform::windows_version() >= WinUiCompositionShared::MIN_HOST_BACKDROP_VERSION {
            let compositor3 = compositor.cast::<ICompositor3>()?;
            compositor3.create_host_backdrop_brush().required()?
        } else {
            let compositor2 = compositor.cast::<ICompositor2>()?;
            compositor2.create_backdrop_brush().required()?
        };

        brush.cast::<ICompositionBrush>()
    }

    /// A source parameter of an effect with the string handle that names
    /// it. The reference never deletes the handle; here it is deleted
    /// when the caller drops it, after the brushes took the name.
    fn get_parameter_source(
        name: &str,
        back_drop_parameter_factory: &ComPtr<ICompositionEffectSourceParameterFactory>,
    ) -> Result<(ComPtr<IGraphicsEffectSource>, HStringInterop), HResult> {
        let backdrop_string = HStringInterop::new(Some(name))?;
        let back_drop_parameter = back_drop_parameter_factory.create(backdrop_string.handle()).required()?;
        let back_drop_parameter_as_source = back_drop_parameter.cast::<IGraphicsEffectSource>()?;
        Ok((back_drop_parameter_as_source, backdrop_string))
    }
}
