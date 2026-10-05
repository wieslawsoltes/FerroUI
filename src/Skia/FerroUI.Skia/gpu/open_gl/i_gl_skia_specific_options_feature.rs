/// Options an OpenGL context has for the Skia backend.
///
/// A context announces them through its optional features:
/// `try_get_feature(TypeId::of::<dyn IGlSkiaSpecificOptionsFeature>())`
/// returns an `Rc<dyn IGlSkiaSpecificOptionsFeature>`.
pub trait IGlSkiaSpecificOptionsFeature {
    /// Whether Skia should resolve the OpenGL entry points itself, with the
    /// interface native to the platform, instead of through the entry point
    /// lookup of the context.
    fn use_native_skia_gr_gl_interface(&self) -> bool;
}
