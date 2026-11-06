//! Runtime selection between the sample's Metal and CGL producers.
#[path = "native_renderer.rs"]
mod gl;
#[path = "renderer.rs"]
mod metal;
pub fn probes_complete() -> bool {
    std::env::args().any(|a| a == "--opengl") || metal::probes_complete()
}
pub fn retained() -> bool {
    metal::retained() || gl::retained()
}
pub fn install(
    root: &ferroui_base::Ref<ferroui_controls::Panel>,
    source: std::sync::Arc<ferroui_base::rendering::composition::frames::FrameMailbox>,
    interop: &dyn ferroui_base::rendering::composition::ICompositionGpuInterop,
) -> Option<ferroui_base::Ref<ferroui_media_controls::RendererHost>> {
    if interop.shared_context_id().is_some() {
        gl::install(root, source, interop)
    } else {
        metal::install(root, source, interop)
    }
}
