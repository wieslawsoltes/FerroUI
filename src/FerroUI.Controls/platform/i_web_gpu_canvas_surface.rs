use ferroui_base::PixelSize;

/// The canvas of a web page that a render backend draws to with the WebGPU of the page.
///
/// A browser platform hands it out as a kind of its render surface. The canvas belongs to the
/// thread of the page, as the WebGPU device of a page does.
pub trait IWebGpuCanvasSurface {
    /// The CSS selector that names the canvas element in the page.
    fn selector(&self) -> String;

    /// The size of the next frame in device pixels. The canvas has that size afterwards.
    fn frame_size(&self) -> PixelSize;

    /// Whether the page prefers `RGBA8Unorm` for a canvas (`BGRA8Unorm` otherwise).
    fn prefers_rgba8(&self) -> bool;

    /// A frame of `size` was drawn to the canvas.
    fn frame_presented(&self, size: PixelSize);
}
