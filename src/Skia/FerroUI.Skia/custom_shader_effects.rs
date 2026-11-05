//! The runtime shader effects of the Skia backend (#455, E4; not from
//! upstream; docs/porting/custom-effects.md, section 3.4): the SkSL program
//! of a custom effect whose type is a shader
//! ([`CustomEffect::shader`](ferroui_base::media::effects::CustomEffect::shader)),
//! compiled once per thread and type by the runtime-effect cache
//! ([`crate::runtime_effects`]), checked once against the uniform convention
//! of its parameters, and given the uniforms of the snapshot on each draw.
//!
//! The drawing context draws it around the content it pushes the effect for:
//!
//! - a content-sampling program is the runtime shader image filter of a
//!   save layer (`image_filters::runtime_shader`), the layer of the content
//!   bound to the child `content`, so the content stays a texture of the GPU
//!   (Ganesh, Graphite) and is never read back;
//! - an overlay is drawn over the content inside a save layer, with the blend
//!   mode of its description, before the layer is composed.
//!
//! Skia evaluates a runtime shader image filter in the space of the canvas at
//! its save layer, with the rest of the transform applied to the result (its
//! filter takes translations only). The layer is therefore saved under a
//! matrix that maps the coordinates of the convention (device pixels from the
//! top left corner of the area of the effect, rounded out to whole pixels)
//! to the device: a translation by whole pixels when the transform of the
//! content scales and translates, so the layer has the resolution of the
//! device and its pixels are those of the target; the content is drawn into
//! it with its own transform.
//!
//! The images of an overlay (its children, #455, E6) are image shaders of the
//! Skia images of their bitmaps, stretched over the area of the effect and
//! sampled by the nearest texel. An image is made a Skia image once per
//! thread (and version of its bitmap) and kept by the identity of its handle
//! ([`effect_image`]); a GPU context makes the texture of that image once and
//! keeps it while the image is held (Graphite through
//! [`drawable_image`](crate::gpu::drawable_image), Ganesh through its cache of
//! uploads by the identity of the image), so a frame that changes only
//! uniforms uploads nothing. When the last handle of an image is dropped, on
//! whatever thread, the next frame of this thread drops its Skia image
//! ([`release_dropped_images`]), and the context releases the texture with
//! its next flush.
//!
//! A program that does not compile, whose uniforms or children are not those
//! of the convention, or a description whose children are not image
//! parameters of an overlay
//! ([`ImmutableCustomEffect::check_shader_children`]), is reported once
//! ([`report_failure`]) and the effect is drawn as an effect without a
//! factory: its content unchanged.

use crate::gpu::ISkiaGrContext;
use crate::runtime_effects::{self, make_shader, ProgramKind, Uniforms};
use crate::skia_sharp_extensions::to_sk_blend_mode;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::media::effects::{
    shader_uniform_name, EffectImage, EffectImageWeak, ImmutableCustomEffect, ShaderEffectDescription,
    ShaderUniformType, ShaderUniformValue, CUSTOM_EFFECT_CONTENT,
};
use skia_safe as sk;
use skia_safe::runtime_effect::{uniform, ChildPtr, ChildType, RuntimeShaderBuilder};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

/// The compiled program of the shader of a custom effect type, checked
/// against the uniform convention.
pub(crate) struct ShaderEffectProgram {
    /// The compiled SkSL.
    effect: sk::RuntimeEffect,
    /// Whether the program samples the content (the child `content`).
    pub(crate) samples_content: bool,
    /// How an overlay is drawn over the content.
    pub(crate) blend_mode: sk::BlendMode,
    /// How far the program samples the content from the pixel it colours,
    /// in device independent pixels.
    sample_radius: f32,
}

ferroui_base::ferro_thread_local! {
    /// The programs of this thread, by the type of the effect; `None` for a
    /// type whose program failed (reported once).
    static PROGRAMS: RefCell<HashMap<TypeId, Option<Rc<ShaderEffectProgram>>>> = RefCell::new(HashMap::new());
}

/// The checked program of the shader effect `effect`: compiled and checked
/// on the first call of this thread for the type of the effect. `None` when
/// the effect is not a shader effect, runtime effects are switched off
/// ([`runtime_effects::set_enabled`]), or the program failed (reported once).
pub(crate) fn program(effect: &ImmutableCustomEffect) -> Option<Rc<ShaderEffectProgram>> {
    if !runtime_effects::is_enabled() {
        return None;
    }
    let type_id = effect.value_type_id();
    if let Some(found) = PROGRAMS.with(|programs| programs.borrow().get(&type_id).cloned()) {
        return found;
    }
    let description = effect.shader()?;
    let program = build(effect, &description).map(Rc::new);
    PROGRAMS.with(|programs| programs.borrow_mut().insert(type_id, program.clone()));
    program
}

fn build(effect: &ImmutableCustomEffect, description: &ShaderEffectDescription) -> Option<ShaderEffectProgram> {
    let name = effect.name();
    if let Err(reason) = effect.check_shader_children() {
        report_failure(name, &reason);
        return None;
    }
    let Some(compiled) = runtime_effects::compile_named(ProgramKind::Shader, &description.sksl, name) else {
        // The runtime-effect cache logged the error of the compiler.
        report_failure(name, "its SkSL program does not compile");
        return None;
    };
    if let Err(reason) = check(&compiled, effect, description) {
        report_failure(name, &reason);
        return None;
    }
    let sample_radius = if description.sample_radius.is_finite() { description.sample_radius.max(0.0) } else { 0.0 };
    Some(ShaderEffectProgram {
        effect: compiled,
        samples_content: description.samples_content(),
        blend_mode: to_sk_blend_mode(description.blend_mode),
        sample_radius: sample_radius as f32,
    })
}

/// Whether a declared uniform type is the type of the convention.
fn type_matches(declared: uniform::Type, expected: ShaderUniformType) -> bool {
    matches!(
        (declared, expected),
        (uniform::Type::Float, ShaderUniformType::Float)
            | (uniform::Type::Float2, ShaderUniformType::Float2)
            | (uniform::Type::Float4, ShaderUniformType::Float4)
            | (uniform::Type::Int, ShaderUniformType::Int)
    )
}

/// Checks the uniforms and the children `program` declares against the
/// convention for `effect` and its `description`: the uniforms of
/// [`ImmutableCustomEffect::shader_uniform_layout`], in order, with their
/// names and types, none an array; the child `content` (a shader) for a
/// content-sampling program, the children of the description (shaders named
/// as their image parameters with every `-` written `_`, in order) for an
/// overlay.
pub(crate) fn check(
    program: &sk::RuntimeEffect,
    effect: &ImmutableCustomEffect,
    description: &ShaderEffectDescription,
) -> Result<(), String> {
    let samples_content = description.samples_content();
    let layout = effect.shader_uniform_layout();
    let declared = program.uniforms();
    let expected = || {
        let names: Vec<String> = layout.iter().map(|(name, ty)| format!("{} {name}", ty.sksl())).collect();
        names.join(", ")
    };
    if declared.len() != layout.len() {
        return Err(format!(
            "the program declares {} uniforms, the convention has {} ({})",
            declared.len(),
            layout.len(),
            expected()
        ));
    }
    for (index, (uniform, (name, ty))) in declared.iter().zip(&layout).enumerate() {
        if uniform.name() != name.as_str() {
            return Err(format!("uniform {index} is {}, the convention has {name} ({})", uniform.name(), expected()));
        }
        if uniform.is_array() || !type_matches(uniform.ty(), *ty) {
            return Err(format!("the uniform {name} is not a {} ({})", ty.sksl(), expected()));
        }
    }
    let children = program.children();
    if samples_content {
        let content = children.len() == 1
            && children[0].name() == CUSTOM_EFFECT_CONTENT
            && matches!(children[0].ty(), ChildType::Shader);
        if !content {
            return Err(format!("a content-sampling program declares one child, uniform shader {CUSTOM_EFFECT_CONTENT}"));
        }
    } else {
        let expected: Vec<String> = description.children.iter().map(|child| shader_uniform_name(child)).collect();
        let matches = children.len() == expected.len()
            && children
                .iter()
                .zip(&expected)
                .all(|(child, name)| child.name() == name.as_str() && matches!(child.ty(), ChildType::Shader));
        if !matches {
            if expected.is_empty() {
                return Err("an overlay without images declares no child".to_owned());
            }
            let declared: Vec<String> = expected.iter().map(|name| format!("uniform shader {name}")).collect();
            return Err(format!("the overlay declares the children of its images, in order: {}", declared.join(", ")));
        }
    }
    Ok(())
}

/// The names of the effects whose program failed, each reported once.
static REPORTED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// Reports that the shader effect `name` is not drawn by its program
/// (`reason`) and that its content is drawn unchanged: logged the first time
/// for a name (an error in the log area `Skia`), when it returns `true`.
pub(crate) fn report_failure(name: &'static str, reason: &str) -> bool {
    {
        let mut reported = REPORTED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if reported.contains(&name) {
            return false;
        }
        reported.push(name);
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Skia") {
        logger.log(
            None,
            &format!("The shader of the custom effect {name} is not drawn ({reason}); its content is drawn unchanged."),
        );
    }
    true
}

/// How many times the failure of the shader effect `name` was reported (0
/// or 1).
#[cfg(test)]
pub(crate) fn failure_reports(name: &str) -> usize {
    REPORTED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).iter().filter(|reported| **reported == name).count()
}

/// The layer a shader effect is drawn in: the area of the effect in the
/// coordinates of the convention.
pub(crate) struct ShaderLayer {
    /// The matrix of the canvas at the save layer: from the coordinates of
    /// the layer to the device.
    pub(crate) to_device: sk::M44,
    /// The area in the coordinates of the layer: `(0, 0, width, height)`.
    pub(crate) bounds: sk::Rect,
    /// The width and the height of the area, in device pixels.
    pub(crate) size: [f32; 2],
    /// Device pixels per device independent pixel (the mean of the two
    /// axes when they differ).
    pub(crate) scale: f32,
}

impl ShaderLayer {
    /// The layer of the area `bounds` (in the coordinates of the canvas)
    /// under the transform `matrix` of the canvas; `None` for an empty area or
    /// a transform that collapses it.
    pub(crate) fn new(matrix: &sk::M44, bounds: sk::Rect) -> Option<ShaderLayer> {
        if bounds.is_empty() || !bounds.is_finite() {
            return None;
        }
        let m = matrix.to_m33();
        if m.is_scale_translate() {
            // Device pixels from the top left pixel of the area: a whole
            // pixel translation, the content at the resolution of the device.
            let (device, _) = m.map_rect(bounds);
            let (left, top) = (device.left.floor(), device.top.floor());
            let (width, height) = (device.right.ceil() - left, device.bottom.ceil() - top);
            let scale = (m.scale_x().abs() + m.scale_y().abs()) / 2.0;
            if !(width >= 1.0 && height >= 1.0 && scale > 0.0) || !(width.is_finite() && height.is_finite()) {
                return None;
            }
            return Some(ShaderLayer {
                to_device: sk::M44::translate(left, top, 0.0),
                bounds: sk::Rect::from_wh(width, height),
                size: [width, height],
                scale,
            });
        }
        // A rotation, a skew or a perspective: the area scaled to the
        // lengths of the axes of the transform, the rest applied to the
        // result by Skia.
        let scale_x = m.scale_x().hypot(m.skew_y());
        let scale_y = m.skew_x().hypot(m.scale_y());
        if !(scale_x > 0.0 && scale_y > 0.0 && scale_x.is_finite() && scale_y.is_finite()) {
            return None;
        }
        let (width, height) = ((bounds.width() * scale_x).ceil().max(1.0), (bounds.height() * scale_y).ceil().max(1.0));
        let mut to_device = matrix.clone();
        to_device.pre_translate(bounds.left, bounds.top, None);
        to_device.pre_scale(1.0 / scale_x, 1.0 / scale_y);
        Some(ShaderLayer {
            to_device,
            bounds: sk::Rect::from_wh(width, height),
            size: [width, height],
            scale: (scale_x + scale_y) / 2.0,
        })
    }
}

/// The runtime shader image filter of a content-sampling program for the
/// snapshot `effect` in `layer`: the uniforms of this frame, the layer bound
/// to the child `content`. `None` when Skia refuses them (reported once).
pub(crate) fn content_filter(
    program: &ShaderEffectProgram,
    effect: &ImmutableCustomEffect,
    layer: &ShaderLayer,
) -> Option<sk::ImageFilter> {
    let mut builder = RuntimeShaderBuilder::new(program.effect.clone());
    let declared = program.effect.uniforms();
    let mut index = 0;
    let mut failed = false;
    effect.for_each_shader_uniform(layer.size, layer.scale, &mut |_, value| {
        // The uniforms were checked against the convention: the index is
        // that of the declaration.
        let Some(uniform) = declared.get(index) else {
            failed = true;
            return;
        };
        index += 1;
        let result = match value {
            ShaderUniformValue::Int(value) => builder.set_uniform_int(uniform.name(), &[value]),
            other => builder.set_uniform_float(uniform.name(), other.floats()),
        };
        failed |= result.is_err();
    });
    if failed || index != declared.len() {
        report_failure(effect.name(), "Skia did not take its uniforms");
        return None;
    }
    let radius = program.sample_radius * layer.scale;
    sk::image_filters::runtime_shader_with_options(&builder, radius, CUSTOM_EFFECT_CONTENT, None, false)
}

/// The shader of an overlay program for the snapshot `effect` in `layer`,
/// with the uniforms of this frame and its images as children: the image
/// shaders of [`child_shader`], for a canvas that draws with `gr_context`.
/// `None` when they do not match its program (reported once).
pub(crate) fn overlay_shader(
    program: &ShaderEffectProgram,
    effect: &ImmutableCustomEffect,
    layer: &ShaderLayer,
    gr_context: Option<&dyn ISkiaGrContext>,
) -> Option<sk::Shader> {
    let mut uniforms = Uniforms::new(&program.effect);
    let declared = program.effect.uniforms();
    let mut index = 0;
    effect.for_each_shader_uniform(layer.size, layer.scale, &mut |_, value| {
        if let Some(uniform) = declared.get(index) {
            match value {
                ShaderUniformValue::Int(value) => {
                    uniforms.int(uniform.name(), value);
                }
                other => {
                    uniforms.floats(uniform.name(), other.floats());
                }
            }
        }
        index += 1;
    });
    let mut children: Vec<ChildPtr> = Vec::new();
    effect.for_each_shader_child(&mut |_, image| children.push(ChildPtr::Shader(child_shader(image, layer, gr_context))));
    let shader = make_shader(&program.effect, &uniforms, &children, None);
    if shader.is_none() {
        report_failure(effect.name(), "Skia did not take its uniforms or its images");
    }
    shader
}

// --- images (E6) ----------------------------------------------------------------

/// The Skia image of an effect image, made on this thread.
struct SkiaEffectImage {
    /// Tells when the last handle of the image is dropped.
    handle: EffectImageWeak,
    /// The version of the bitmap the image was made at.
    version: i32,
    /// The image; `None` when the bitmap has none (reported once).
    image: Option<sk::Image>,
}

ferroui_base::ferro_thread_local! {
    /// The Skia images of the effect images of this thread, by the identity
    /// of the handle.
    static EFFECT_IMAGES: RefCell<HashMap<u64, SkiaEffectImage>> = RefCell::new(HashMap::new());
    /// The generation of releases of effect images this thread last looked
    /// at its images for ([`EffectImage::release_generation`]).
    static RELEASES_SEEN: Cell<u64> = const { Cell::new(0) };
    /// How many Skia images this thread made of effect images.
    static IMAGES_MADE: Cell<u64> = const { Cell::new(0) };
}

/// Drops the Skia images of the effect images whose last handle was dropped
/// since this thread last looked (on any thread): one atomic read when none
/// was. The GPU context releases the texture of such an image with its next
/// flush. Called by the drawing contexts of this thread when they are
/// created, so a dropped image is released by the next frame.
pub(crate) fn release_dropped_images() {
    let generation = EffectImage::release_generation();
    if RELEASES_SEEN.with(|seen| seen.replace(generation)) == generation {
        return;
    }
    EFFECT_IMAGES.with(|images| images.borrow_mut().retain(|_, entry| entry.handle.is_alive()));
}

/// How many Skia images this thread made of effect images: constant over
/// frames that only change uniforms.
#[cfg(test)]
pub(crate) fn effect_images_made() -> u64 {
    IMAGES_MADE.with(Cell::get)
}

/// How many effect images this thread keeps a Skia image for.
#[cfg(test)]
pub(crate) fn effect_images_kept() -> usize {
    EFFECT_IMAGES.with(|images| images.borrow().len())
}

/// The Skia image of `image`, made the first time this thread draws the
/// image (or a new version of its bitmap) and kept while a handle of the
/// image lives; `None` for the empty image or a bitmap Skia cannot draw.
pub(crate) fn effect_image(image: &EffectImage) -> Option<sk::Image> {
    if image.is_empty() {
        return None;
    }
    release_dropped_images();
    let (id, version) = (image.id(), image.version());
    EFFECT_IMAGES.with(|images| {
        let mut images = images.borrow_mut();
        if let Some(entry) = images.get(&id) {
            if entry.version == version {
                return entry.image.clone();
            }
        }
        let made = skia_image_of(image);
        IMAGES_MADE.with(|count| count.set(count.get() + 1));
        if made.is_none() {
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Skia") {
                let size = image.pixel_size();
                logger.log(
                    None,
                    &format!(
                        "An image of {}x{} pixels of a shader effect is not a bitmap Skia draws; it is drawn transparent.",
                        size.width, size.height
                    ),
                );
            }
        }
        images.insert(id, SkiaEffectImage { handle: image.downgrade(), version, image: made.clone() });
        made
    })
}

/// The Skia image of the bitmap of `image`: the image of an immutable
/// bitmap as it is, a snapshot of a writeable bitmap or of a surface, or a
/// raster image of the pixels of another readable bitmap.
fn skia_image_of(image: &EffectImage) -> Option<sk::Image> {
    use crate::immutable_bitmap::ImmutableBitmap;
    use crate::surface_render_target::SurfaceRenderTarget;
    use crate::writeable_bitmap_impl::WriteableBitmapImpl;

    let bitmap = image.bitmap()?;
    let any = bitmap.as_any();
    if let Some(bitmap) = any.downcast_ref::<ImmutableBitmap>() {
        return Some(bitmap.image());
    }
    if let Some(bitmap) = any.downcast_ref::<WriteableBitmapImpl>() {
        return Some(bitmap.get_snapshot());
    }
    if let Some(bitmap) = any.downcast_ref::<SurfaceRenderTarget>() {
        return Some(bitmap.snapshot_image());
    }
    let pixels = image.pixels()?;
    let (width, height) = (i32::try_from(pixels.width()).ok()?, i32::try_from(pixels.height()).ok()?);
    let info = sk::ImageInfo::new((width, height), sk::ColorType::RGBA8888, sk::AlphaType::Premul, None);
    sk::images::raster_from_data(&info, sk::Data::new_copy(pixels.rgba()), pixels.width() * 4)
}

/// The child shader of `image` in `layer`: the image stretched over the
/// area (a local matrix from the pixels of the image to those of the
/// layer), sampled by the nearest texel, transparent beyond it (decal), in
/// the form a canvas drawing with `gr_context` draws (a texture on Graphite);
/// a transparent shader for the empty image.
fn child_shader(image: &EffectImage, layer: &ShaderLayer, gr_context: Option<&dyn ISkiaGrContext>) -> sk::Shader {
    let transparent = || sk::shaders::color(sk::Color::TRANSPARENT);
    let Some(source) = effect_image(image) else {
        return transparent();
    };
    let (width, height) = (source.width(), source.height());
    if width < 1 || height < 1 {
        return transparent();
    }
    let drawable = crate::gpu::drawable_image(gr_context, source, false);
    let stretch = sk::Matrix::scale((layer.size[0] / width as f32, layer.size[1] / height as f32));
    let sampling = sk::SamplingOptions::new(sk::FilterMode::Nearest, sk::MipmapMode::None);
    drawable
        .to_shader((sk::TileMode::Decal, sk::TileMode::Decal), sampling, &stretch)
        .unwrap_or_else(transparent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::effects::library::{
        DissolveTransitionEffect, MaskTransitionEffect, PixelateEffect, ScanlineShaderEffect, SpotlightShaderEffect,
    };
    use ferroui_base::media::effects::{CustomEffect, EffectParameterValue};
    use ferroui_base::media::imaging::BitmapBlendingMode;

    #[test]
    fn the_programs_of_the_library_compile_and_declare_their_parameters() {
        let effects = [
            ImmutableCustomEffect::new(PixelateEffect::default()),
            ImmutableCustomEffect::new(ScanlineShaderEffect::default()),
            ImmutableCustomEffect::new(SpotlightShaderEffect::default()),
            ImmutableCustomEffect::new(DissolveTransitionEffect::default()),
            ImmutableCustomEffect::new(MaskTransitionEffect::default()),
        ];
        for effect in effects {
            let description = effect.shader().expect("a shader effect");
            let compiled = runtime_effects::compile_named(ProgramKind::Shader, &description.sksl, effect.name())
                .unwrap_or_else(|| panic!("{effect}: the SkSL compiles"));
            assert_eq!(Ok(()), check(&compiled, &effect, &description), "{effect}");
            let program = program(&effect).unwrap_or_else(|| panic!("{effect}: a program"));
            assert_eq!(description.samples_content(), program.samples_content, "{effect}");
            // Compiled once per thread and type.
            assert!(Rc::ptr_eq(&program, &super::program(&effect).unwrap()), "{effect}");
            // The uniforms of a frame make a filter or a shader.
            let layer = ShaderLayer::new(&sk::M44::new_identity(), sk::Rect::from_wh(40.0, 30.0)).unwrap();
            if program.samples_content {
                assert!(content_filter(&program, &effect, &layer).is_some(), "{effect}");
            } else {
                assert!(overlay_shader(&program, &effect, &layer, None).is_some(), "{effect}");
            }
        }
    }

    /// An effect with a colour and a flag, for the checks.
    #[derive(Clone, Debug, Default, PartialEq)]
    struct Checked {
        color: ferroui_base::media::Color,
        on: bool,
    }

    impl CustomEffect for Checked {
        const NAME: &'static str = "skia-shader-test-checked";

        fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
            f("tint-color", self.color.into());
            f("on", self.on.into());
        }

        fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
            false
        }
    }

    #[test]
    fn a_program_is_checked_against_the_convention() {
        let effect = ImmutableCustomEffect::new(Checked::default());
        let as_overlay = ShaderEffectDescription::overlay("", "", BitmapBlendingMode::SourceOver);
        let as_sampling = ShaderEffectDescription::content_sampling("", "");
        let compile = |source: &str| runtime_effects::compile(ProgramKind::Shader, source).expect("compiles");
        let head = "uniform float2 layer_size; uniform float layer_scale;";
        let body = "half4 main(float2 coord) { return half4(tint_color) * half(on); }";
        let overlay = compile(&format!("{head} uniform float4 tint_color; uniform int on; {body}"));
        assert_eq!(Ok(()), check(&overlay, &effect, &as_overlay));
        // An overlay is not a content-sampling program.
        assert!(check(&overlay, &effect, &as_sampling).is_err());
        let sampling = compile(&format!(
            "{head} uniform float4 tint_color; uniform int on; uniform shader content;\n\
             half4 main(float2 coord) {{ return content.eval(coord) * half(on); }}"
        ));
        assert_eq!(Ok(()), check(&sampling, &effect, &as_sampling));
        assert!(check(&sampling, &effect, &as_overlay).is_err());
        // A child of another name.
        let other = compile(&format!(
            "{head} uniform float4 tint_color; uniform int on; uniform shader image;\n\
             half4 main(float2 coord) {{ return image.eval(coord); }}"
        ));
        assert!(check(&other, &effect, &as_sampling).is_err());
        // The children of an overlay (E6): its images, by name and in order.
        let with_image = as_overlay.clone().with_children(["image"]);
        assert_eq!(Ok(()), check(&other, &effect, &with_image));
        assert!(check(&other, &effect, &as_overlay).is_err(), "an overlay without images declares none");
        assert!(check(&other, &effect, &as_overlay.clone().with_children(["page"])).is_err());
        let two = compile(&format!(
            "{head} uniform float4 tint_color; uniform int on; uniform shader to_page; uniform shader mask;\n\
             half4 main(float2 coord) {{ return to_page.eval(coord) * mask.eval(coord).a; }}"
        ));
        assert_eq!(Ok(()), check(&two, &effect, &as_overlay.clone().with_children(["to-page", "mask"])));
        assert!(check(&two, &effect, &as_overlay.clone().with_children(["mask", "to-page"])).is_err(), "in order");
        assert!(check(&two, &effect, &as_overlay.clone().with_children(["to-page"])).is_err());

        // A uniform of the wrong type, out of order, missing, renamed or an
        // extra one.
        for uniforms in [
            "uniform float3 tint_color; uniform int on;",
            "uniform int on; uniform float4 tint_color;",
            "uniform float4 tint_color;",
            "uniform float4 tintColor; uniform int on;",
            "uniform float4 tint_color; uniform int on; uniform float extra;",
            "uniform float4 tint_color; uniform float on;",
            "uniform float4 tint_color[2]; uniform int on;",
        ] {
            let program = compile(&format!(
                "{head} {uniforms}\nhalf4 main(float2 coord) {{ return half4(0.0); }}"
            ));
            assert!(check(&program, &effect, &as_overlay).is_err(), "{uniforms}");
        }
        // The layer uniforms come first.
        let program = compile(&format!(
            "uniform float layer_scale; uniform float2 layer_size; uniform float4 tint_color; uniform int on;\n\
             half4 main(float2 coord) {{ return half4(0.0); }}"
        ));
        assert!(check(&program, &effect, &as_overlay).is_err());
    }

    #[test]
    fn a_layer_is_in_whole_device_pixels_from_the_top_left_of_the_area() {
        // A scale of 2 and a translation of half a pixel.
        let matrix = sk::M44::new(2.0, 0.0, 0.0, 10.5, 0.0, 2.0, 0.0, 20.25, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        let layer = ShaderLayer::new(&matrix, sk::Rect::from_xywh(5.0, 5.0, 20.0, 10.0)).unwrap();
        // The device area is 20.5, 30.25 to 60.5, 50.25.
        assert_eq!(sk::M44::translate(20.0, 30.0, 0.0), layer.to_device);
        assert_eq!([41.0, 21.0], layer.size);
        assert_eq!(sk::Rect::from_wh(41.0, 21.0), layer.bounds);
        assert_eq!(2.0, layer.scale);
        assert!(ShaderLayer::new(&matrix, sk::Rect::from_wh(0.0, 10.0)).is_none());
        // A rotation: the area scaled to the lengths of the axes.
        let rotation = sk::M44::new(0.0, -3.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        let layer = ShaderLayer::new(&rotation, sk::Rect::from_wh(10.0, 5.0)).unwrap();
        assert_eq!([30.0, 15.0], layer.size);
        assert_eq!(3.0, layer.scale);
    }
}
