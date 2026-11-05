//! Tests of the images of custom effects (#455, E6; not from upstream): the
//! handle (identity, `Send` and `Sync`, the lease of the bitmap, the weak
//! handle and the generation of releases, the pixels read once and sampled
//! stretched over an area), the image parameter (interpolation, text,
//! parsing), the children of a shader (the checks, the visit of a frame, the
//! WGSL the pass adds), and the page transitions of the library: their
//! programs declare the uniforms and the children of the convention, they
//! print and parse back without their images, are registered and prewarmed,
//! leave the content unchanged at their neutral values, keep their images
//! through a transition from no effect, and have CPU references. What Skia and
//! Vello draw is tested against those references in their crates.

use super::library::{DissolveTransitionEffect, MaskTransitionEffect, PixelateEffect};
use super::*;
use crate::media::imaging::{BitmapBlendingMode, BitmapEncoderOptions};
use crate::platform::{AlphaFormat, IBitmapImpl, ILockedFramebuffer, IReadableBitmapImpl, PixelFormat, SharedBitmapImpl};
use crate::rendering::composition::effects::effect_prewarm_names;
use crate::{PixelSize, Vector};
use std::any::Any;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;

/// A bitmap of the tests: pixels in a format, which a lock copies out.
struct TestBitmap {
    size: PixelSize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    pixels: Vec<u8>,
    version: AtomicI32,
    disposed: Arc<AtomicBool>,
}

impl TestBitmap {
    /// A `width` by `height` bitmap of premultiplied RGBA `pixels`.
    fn rgba(width: i32, height: i32, pixels: Vec<u8>) -> TestBitmap {
        Self::with_format(width, height, PixelFormat::RGBA8888, AlphaFormat::Premul, pixels)
    }

    fn with_format(width: i32, height: i32, format: PixelFormat, alpha_format: AlphaFormat, pixels: Vec<u8>) -> TestBitmap {
        assert_eq!((width * height * 4) as usize, pixels.len());
        TestBitmap {
            size: PixelSize::new(width, height),
            format,
            alpha_format,
            pixels,
            version: AtomicI32::new(1),
            disposed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn shared(self) -> Arc<SharedBitmapImpl> {
        Arc::new(self)
    }
}

impl IBitmapImpl for TestBitmap {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }
    fn pixel_size(&self) -> PixelSize {
        self.size
    }
    fn version(&self) -> i32 {
        self.version.load(Ordering::SeqCst)
    }
    fn save(&self, _: &mut dyn Write, _: &BitmapEncoderOptions) -> std::io::Result<()> {
        Ok(())
    }
    fn dispose(&self) {
        self.disposed.store(true, Ordering::SeqCst);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IReadableBitmapImpl for TestBitmap {
    fn format(&self) -> Option<PixelFormat> {
        Some(self.format)
    }
    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(self.alpha_format)
    }
    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        Rc::new(TestFramebuffer {
            size: self.size,
            format: self.format,
            alpha_format: self.alpha_format,
            pixels: RefCell::new(self.pixels.clone()),
        })
    }
}

struct TestFramebuffer {
    size: PixelSize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    pixels: RefCell<Vec<u8>>,
}

impl ILockedFramebuffer for TestFramebuffer {
    fn address(&self) -> *mut u8 {
        self.pixels.borrow_mut().as_mut_ptr()
    }
    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.pixels.borrow_mut());
    }
    fn size(&self) -> PixelSize {
        self.size
    }
    fn row_bytes(&self) -> i32 {
        self.size.width * 4
    }
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }
    fn format(&self) -> PixelFormat {
        self.format
    }
    fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }
    fn dispose(&self) {}
}

/// A `width` by `height` image of one premultiplied colour.
fn solid(width: i32, height: i32, rgba: [u8; 4]) -> EffectImage {
    EffectImage::from_impl(TestBitmap::rgba(width, height, rgba.repeat((width * height) as usize)).shared())
}

/// A 2 by 2 image of four colours: red, green, blue and translucent white.
fn quad() -> EffectImage {
    let pixels = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [128, 128, 128, 128]].concat();
    EffectImage::from_impl(TestBitmap::rgba(2, 2, pixels).shared())
}

// --- the handle ----------------------------------------------------------------

#[test]
fn a_handle_is_send_sync_and_compares_by_identity() {
    fn assert_send_sync<T: Send + Sync + Clone + PartialEq>() {}
    assert_send_sync::<EffectImage>();
    assert_send_sync::<EffectParameterValue>();

    let image = quad();
    let clone = image.clone();
    assert_eq!(image, clone);
    assert_eq!(image.id(), clone.id());
    // The same pixels in another handle are another image.
    let other = quad();
    assert_ne!(image, other);
    assert_ne!(image.id(), other.id());
    // The empty handles are equal, and differ from any image.
    assert_eq!(EffectImage::empty(), EffectImage::default());
    assert!(EffectImage::empty().is_empty());
    assert_eq!(0, EffectImage::empty().id());
    assert_ne!(EffectImage::empty(), image);
    assert_eq!(PixelSize::new(2, 2), image.pixel_size());
    assert_eq!(PixelSize::default(), EffectImage::empty().pixel_size());

    // A handle crosses to another thread and back.
    let moved = std::thread::spawn(move || clone).join().unwrap();
    assert_eq!(image, moved);
}

#[test]
fn a_handle_holds_its_bitmap_until_the_last_clone_is_dropped() {
    let bitmap = TestBitmap::rgba(1, 1, vec![1, 2, 3, 4]);
    let disposed = bitmap.disposed.clone();
    let image = EffectImage::from_impl(bitmap.shared());
    let weak = image.downgrade();
    assert!(weak.is_alive());
    assert_eq!(image.id(), weak.id());
    assert_eq!(Some(image.clone()), weak.upgrade());

    let generation = EffectImage::release_generation();
    let clone = image.clone();
    drop(image);
    assert!(weak.is_alive(), "a clone holds the image");
    assert!(!disposed.load(Ordering::SeqCst));
    // The last clone, dropped on another thread.
    std::thread::spawn(move || drop(clone)).join().unwrap();
    assert!(!weak.is_alive());
    assert_eq!(None, weak.upgrade());
    assert!(disposed.load(Ordering::SeqCst), "the bitmap is disposed with the last handle");
    assert!(EffectImage::release_generation() > generation, "a release is a new generation");
    // The weak handle of the empty handle is never alive.
    assert!(!EffectImage::empty().downgrade().is_alive());
}

#[test]
fn a_handle_of_a_bitmap_shares_its_count() {
    use crate::utilities::RefCountable;
    let bitmap: Arc<SharedBitmapImpl> = TestBitmap::rgba(1, 1, vec![0; 4]).shared();
    let released = Arc::new(AtomicBool::new(false));
    let flag = released.clone();
    let reference = RefCountable::create(bitmap, move || flag.store(true, Ordering::SeqCst));
    let image = EffectImage::from_ref(&reference);
    assert_eq!(2, reference.ref_count());
    reference.dispose();
    assert!(!released.load(Ordering::SeqCst), "the handle keeps the bitmap");
    drop(image);
    assert!(released.load(Ordering::SeqCst));
}

#[test]
fn the_pixels_are_read_once_per_version_as_premultiplied_rgba() {
    // BGRA, not premultiplied: half transparent red.
    let bitmap = TestBitmap::with_format(1, 1, PixelFormat::BGRA8888, AlphaFormat::Unpremul, vec![0, 0, 255, 128]);
    let image = EffectImage::from_impl(bitmap.shared());
    let pixels = image.pixels().expect("readable");
    assert_eq!((1, 1), (pixels.width(), pixels.height()));
    let [r, g, b, a] = pixels.texel(0, 0);
    assert_eq!((0, 0, 128), (g, b, a));
    assert!(r.abs_diff(128) <= 1, "premultiplied red: {r}");
    // Read once: the same bytes.
    assert!(std::ptr::eq(pixels.rgba().as_ptr(), image.pixels().unwrap().rgba().as_ptr()));
    assert_eq!(None, EffectImage::empty().pixels());
}

#[test]
fn an_image_is_sampled_stretched_over_the_area_by_the_nearest_texel() {
    let pixels = quad().pixels().unwrap();
    // A 2 by 2 image over a 4 by 4 area: each texel covers 2 by 2 pixels.
    assert_eq!([255, 0, 0, 255], pixels.sample_stretched([0.5, 0.5], [4.0, 4.0]));
    assert_eq!([255, 0, 0, 255], pixels.sample_stretched([1.5, 1.5], [4.0, 4.0]));
    assert_eq!([0, 255, 0, 255], pixels.sample_stretched([2.5, 0.5], [4.0, 4.0]));
    assert_eq!([0, 0, 255, 255], pixels.sample_stretched([0.5, 3.5], [4.0, 4.0]));
    assert_eq!([128, 128, 128, 128], pixels.sample_stretched([3.5, 3.5], [4.0, 4.0]));
    // Beyond the area: transparent.
    assert_eq!([0; 4], pixels.sample_stretched([-0.5, 0.5], [4.0, 4.0]));
    assert_eq!([0; 4], pixels.sample_stretched([4.5, 0.5], [4.0, 4.0]));
    assert_eq!([0; 4], pixels.texel(2, 0));
}

// --- the parameter -------------------------------------------------------------

#[test]
fn an_image_parameter_switches_at_half_and_keeps_an_image_over_an_empty_one() {
    let (a, b) = (EffectParameterValue::Image(quad()), EffectParameterValue::Image(quad()));
    let empty = EffectParameterValue::Image(EffectImage::empty());
    assert_eq!(Some(a.clone()), a.interpolate(&b, 0.25));
    assert_eq!(Some(b.clone()), a.interpolate(&b, 0.5));
    assert_eq!(Some(b.clone()), a.interpolate(&b, 0.9));
    for progress in [0.0, 0.3, 0.7, 1.0] {
        assert_eq!(Some(a.clone()), empty.interpolate(&a, progress), "{progress}");
        assert_eq!(Some(a.clone()), a.interpolate(&empty, progress), "{progress}");
    }
    assert_eq!(None, a.interpolate(&EffectParameterValue::Float(1.0), 0.5));
    assert!(a.as_image().is_some() && EffectParameterValue::Float(1.0).as_image().is_none());
}

#[test]
fn an_image_parameter_is_written_but_never_parsed() {
    let image = EffectParameterValue::Image(quad());
    assert_eq!("image(2x2)", image.to_string());
    assert_eq!("none", EffectParameterValue::Image(EffectImage::empty()).to_string());
    assert_eq!(None, image.parse_like("image(2x2)"));
    assert_eq!(None, image.parse_like("none"));
    assert_eq!(None, ShaderUniformValue::from_parameter(&image), "an image is no uniform");
}

// --- the children --------------------------------------------------------------

/// An overlay of the tests with an image `page`, a number and the
/// description `$description`.
macro_rules! child_effect {
    ($type:ident, $name:literal, $description:expr) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        struct $type {
            page: EffectImage,
            amount: f64,
        }

        impl CustomEffect for $type {
            const NAME: &'static str = $name;

            fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
                f("page", (&self.page).into());
                f("amount", self.amount.into());
            }

            fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
                match name {
                    "page" => value.as_image().map(|image| self.page = image.clone()).is_some(),
                    "amount" => value.as_float().map(|amount| self.amount = amount).is_some(),
                    _ => false,
                }
            }

            fn shader() -> Option<ShaderEffectDescription> {
                Some($description)
            }
        }
    };
}

const CHILD_SKSL: &str = "uniform float2 layer_size; uniform float layer_scale; uniform float amount; uniform shader page;\n\
                          half4 main(float2 coord) { return page.eval(coord) * half(amount); }";
const CHILD_WGSL: &str = "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, }\n\
                          fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_page(coord) * uniforms.amount; }";

child_effect!(
    WithPage,
    "image-test-with-page",
    ShaderEffectDescription::overlay(CHILD_SKSL, CHILD_WGSL, BitmapBlendingMode::SourceOver).with_children(["page"])
);
child_effect!(
    SamplingWithPage,
    "image-test-sampling-with-page",
    ShaderEffectDescription::content_sampling(CHILD_SKSL, CHILD_WGSL).with_children(["page"])
);
child_effect!(
    NotAnImage,
    "image-test-child-not-an-image",
    ShaderEffectDescription::overlay(CHILD_SKSL, CHILD_WGSL, BitmapBlendingMode::SourceOver).with_children(["amount"])
);
child_effect!(
    Missing,
    "image-test-child-missing",
    ShaderEffectDescription::overlay(CHILD_SKSL, CHILD_WGSL, BitmapBlendingMode::SourceOver).with_children(["other"])
);
child_effect!(
    Twice,
    "image-test-child-twice",
    ShaderEffectDescription::overlay(CHILD_SKSL, CHILD_WGSL, BitmapBlendingMode::SourceOver)
        .with_children(["page", "page"])
);
child_effect!(
    TooMany,
    "image-test-too-many-children",
    ShaderEffectDescription::overlay(CHILD_SKSL, CHILD_WGSL, BitmapBlendingMode::SourceOver)
        .with_children(["page"; MAX_SHADER_CHILDREN + 1])
);

#[test]
fn the_children_of_a_shader_are_image_parameters_of_an_overlay() {
    assert_eq!(Ok(()), ImmutableCustomEffect::new(WithPage::default()).check_shader_children());
    assert_eq!(Ok(()), ImmutableCustomEffect::new(PixelateEffect::default()).check_shader_children());
    for effect in [
        ImmutableCustomEffect::new(SamplingWithPage::default()),
        ImmutableCustomEffect::new(NotAnImage::default()),
        ImmutableCustomEffect::new(Missing::default()),
        ImmutableCustomEffect::new(Twice::default()),
        ImmutableCustomEffect::new(TooMany::default()),
    ] {
        assert!(effect.check_shader_children().is_err(), "{}: refused", effect.name());
    }
}

#[test]
fn a_frame_visits_the_images_of_the_children_in_order() {
    let page = quad();
    let effect = ImmutableCustomEffect::new(WithPage { page: page.clone(), amount: 0.5 });
    let mut visited = Vec::new();
    effect.for_each_shader_child(&mut |name, image| visited.push((name.to_owned(), image.clone())));
    assert_eq!(vec![("page".to_owned(), page)], visited);
    // The image is no uniform.
    assert_eq!(
        vec![
            ("layer_size".to_owned(), ShaderUniformType::Float2),
            ("layer_scale".to_owned(), ShaderUniformType::Float),
            ("amount".to_owned(), ShaderUniformType::Float),
        ],
        effect.shader_uniform_layout()
    );
    // A child without its parameter is the empty image.
    let mut visited = Vec::new();
    ImmutableCustomEffect::new(Missing::default())
        .for_each_shader_child(&mut |name, image| visited.push((name.to_owned(), image.is_empty())));
    assert_eq!(vec![("other".to_owned(), true)], visited);
}

#[test]
fn the_wgsl_module_binds_each_child_and_samples_it_stretched() {
    let module = ImmutableCustomEffect::new(DissolveTransitionEffect::default()).shader().unwrap().wgsl_module();
    for (index, name) in ["from_page", "to_page", "pattern"].into_iter().enumerate() {
        let binding = SHADER_WGSL_CHILDREN_BINDING + index as u32;
        let declaration = format!("@group(0) @binding({binding}) var ferroui_child_{name}: texture_2d<f32>;");
        assert!(module.contains(&declaration), "{declaration}");
        assert!(module.contains(&format!("fn sample_{name}(coord: vec2<f32>) -> vec4<f32>")), "{name}");
        assert!(module.contains(&format!("textureDimensions(ferroui_child_{name})")), "{name}");
    }
    assert!(!module.contains("{name}") && !module.contains("{binding}"));
    assert!(!module.contains(&format!("@binding({})", SHADER_WGSL_CHILDREN_BINDING + 3)));
}

// --- the transitions of the library ----------------------------------------------

/// The transitions of the library at values that are not neutral, with
/// their images.
fn transitions() -> Vec<ImmutableCustomEffect> {
    vec![
        ImmutableCustomEffect::new(DissolveTransitionEffect {
            from_page: solid(4, 4, [255, 0, 0, 255]),
            to_page: solid(4, 4, [0, 0, 255, 255]),
            pattern: quad(),
            progress: 0.5,
            ..DissolveTransitionEffect::default()
        }),
        ImmutableCustomEffect::new(MaskTransitionEffect {
            to_page: solid(4, 4, [0, 0, 255, 255]),
            mask: quad(),
            progress: 0.5,
            softness: 0.1,
        }),
    ]
}

/// The declarations `uniform <type> <name>;` of an SkSL program: the
/// uniforms and, apart, the children (`uniform shader <name>;`).
fn sksl_declarations(source: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut uniforms = Vec::new();
    let mut children = Vec::new();
    for statement in source.split(';') {
        let words: Vec<&str> = statement.split_whitespace().collect();
        if let ["uniform", ty, name] = words[..] {
            if ty == "shader" {
                children.push(name.to_owned());
            } else {
                uniforms.push((name.to_owned(), ty.to_owned()));
            }
        }
    }
    (uniforms, children)
}

#[test]
fn the_programs_of_the_transitions_declare_their_uniforms_and_children() {
    for effect in transitions() {
        let shader = effect.shader().unwrap_or_else(|| panic!("{effect} is a shader effect"));
        assert_eq!(ShaderEffectMode::Overlay, shader.mode, "{effect}");
        assert_eq!(BitmapBlendingMode::SourceOver, shader.blend_mode, "{effect}");
        assert_eq!(Ok(()), effect.check_shader_children(), "{effect}");
        let (uniforms, children) = sksl_declarations(&shader.sksl);
        let expected: Vec<(String, String)> =
            effect.shader_uniform_layout().iter().map(|(name, ty)| (name.clone(), ty.sksl().to_owned())).collect();
        assert_eq!(expected, uniforms, "{effect}: the uniforms of the SkSL");
        let wanted: Vec<String> = shader.children.iter().map(|child| shader_uniform_name(child)).collect();
        assert_eq!(wanted, children, "{effect}: the children of the SkSL, in order");
        for child in &wanted {
            assert!(shader.wgsl.contains(&format!("sample_{child}(")), "{effect}: {child} in the WGSL");
        }
        assert!(!shader.wgsl.contains("sample_content("), "{effect}");
    }
}

#[test]
fn the_transitions_print_parse_back_without_their_images_and_are_registered() {
    for effect in transitions() {
        let text = effect.to_string();
        assert!(!text.contains("page") && !text.contains("mask=") && !text.contains("pattern"), "{text}");
        let parsed = Effect::parse(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
        let parsed = parsed.as_custom_effect().expect("a custom effect").clone();
        assert_eq!(effect.name(), parsed.name());
        // The numbers come back; the images are empty.
        let numbers = |effect: &ImmutableCustomEffect| -> Vec<(&'static str, EffectParameterValue)> {
            effect.parameters().into_iter().filter(|(_, value)| value.as_image().is_none()).collect()
        };
        assert_eq!(numbers(&effect), numbers(&parsed), "{text}");
        parsed.for_each_shader_child(&mut |name, image| assert!(image.is_empty(), "{text}: {name}"));
    }
    // An effect string cannot name an image.
    assert!(Effect::parse("mask-transition(mask=photo.png)").is_err());

    let names = CustomEffectRegistry::names();
    for name in ["dissolve-transition", "mask-transition"] {
        assert!(names.contains(&name), "{name}: {names:?}");
        assert!(effect_prewarm_names().contains(&format!("custom-effect:{name}")), "{name}");
    }
}

/// A 4 by 4 area of premultiplied colours.
fn content() -> Vec<u8> {
    (0..16u8).flat_map(|index| [index * 10, 255 - index * 10, 40, 255]).collect()
}

#[test]
fn a_neutral_transition_leaves_the_content_unchanged() {
    for effect in transitions() {
        let neutral = effect.neutral().expect("a neutral value");
        let mut pixels = content();
        assert!(neutral.cpu_reference(&mut pixels, 4, 4, 1.0), "{neutral}");
        assert_eq!(content(), pixels, "{neutral}");
        // At a progress of 0 the mask transition shows the content even
        // with its images.
        if let Some(mask) = effect.downcast_ref::<MaskTransitionEffect>() {
            let start = ImmutableCustomEffect::new(MaskTransitionEffect { progress: 0.0, ..mask.clone() });
            let mut pixels = content();
            assert!(start.cpu_reference(&mut pixels, 4, 4, 2.0));
            assert_eq!(content(), pixels);
        }
    }
}

#[test]
fn a_transition_from_no_effect_keeps_the_images_of_the_effect() {
    for effect in transitions() {
        let neutral = effect.neutral().unwrap();
        for progress in [0.1, 0.4, 0.6, 0.9] {
            let between = neutral.interpolate(&effect, progress).expect("the same type");
            let mut images = Vec::new();
            between.for_each_shader_child(&mut |_, image| images.push(image.clone()));
            let mut expected = Vec::new();
            effect.for_each_shader_child(&mut |_, image| expected.push(image.clone()));
            assert_eq!(expected, images, "{effect} at {progress}");
            let progress_of = |effect: &ImmutableCustomEffect| effect.parameter("progress").and_then(|p| p.as_float());
            let value = progress_of(&between).unwrap();
            assert!((value - 0.5 * progress).abs() < 1e-9, "{effect} at {progress}: {value}");
        }
    }
}

#[test]
fn the_dissolve_goes_from_one_page_to_the_other() {
    let red = solid(4, 4, [255, 0, 0, 255]);
    let blue = solid(4, 4, [0, 0, 255, 255]);
    let dissolve = |progress: f64| {
        ImmutableCustomEffect::new(DissolveTransitionEffect {
            from_page: red.clone(),
            to_page: blue.clone(),
            pattern: quad(),
            progress,
            ..DissolveTransitionEffect::default()
        })
    };
    let at = |progress: f64| {
        let mut pixels = content();
        assert!(dissolve(progress).cpu_reference(&mut pixels, 4, 4, 1.0));
        pixels
    };
    assert_eq!([255, 0, 0, 255].repeat(16), at(0.0), "the page it leaves");
    assert_eq!([0, 0, 255, 255].repeat(16), at(1.0), "the page it comes to");
    // Half way the dark parts of the pattern (luminance below the frontier)
    // show the page that comes first: the blue quarter (luminance 0.07) is
    // blue, the green one (0.72) red; the glow lights the frontier.
    let half = at(0.5);
    let pixel = |x: usize, y: usize| &half[(y * 4 + x) * 4..][..4];
    assert_eq!(255, pixel(0, 3)[2], "{:?}", pixel(0, 3));
    assert!(pixel(0, 3)[0] < 128, "{:?}", pixel(0, 3));
    assert!(pixel(3, 0)[0] > 200, "{:?}", pixel(3, 0));
}

#[test]
fn the_mask_transition_reveals_the_page_where_the_mask_is_dark() {
    // A mask black on the left and white on the right.
    let mask = EffectImage::from_impl(TestBitmap::rgba(2, 1, [[0, 0, 0, 255], [255, 255, 255, 255]].concat()).shared());
    let page = solid(4, 4, [0, 0, 255, 255]);
    let at = |progress: f64| {
        let effect = ImmutableCustomEffect::new(MaskTransitionEffect {
            to_page: page.clone(),
            mask: mask.clone(),
            progress,
            softness: 0.1,
        });
        let mut pixels = content();
        assert!(effect.cpu_reference(&mut pixels, 4, 4, 1.0));
        pixels
    };
    let half = at(0.5);
    let pixel = |pixels: &[u8], x: usize, y: usize| pixels[(y * 4 + x) * 4..][..4].to_vec();
    assert_eq!(vec![0, 0, 255, 255], pixel(&half, 0, 2), "the dark half shows the page");
    assert_eq!(pixel(&content(), 3, 2), pixel(&half, 3, 2), "the light half shows the content");
    assert_eq!([0, 0, 255, 255].repeat(16), at(1.0));
    // Without a mask: a fade of the page in, half way at a quarter with a
    // softness of 1 (the reveal is `progress * (1 + softness) / softness`).
    let fade = ImmutableCustomEffect::new(MaskTransitionEffect {
        to_page: page.clone(),
        mask: EffectImage::empty(),
        progress: 0.25,
        softness: 1.0,
    });
    let mut pixels = content();
    assert!(fade.cpu_reference(&mut pixels, 4, 4, 1.0));
    let [r, _, b, a] = [pixels[0], pixels[1], pixels[2], pixels[3]];
    assert_eq!(255, a);
    assert!(b > 120 && b < 170 && r == 0, "{:?}", &pixels[..4]);
}
