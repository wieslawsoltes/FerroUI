use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::skia_sharp_extensions::{to_sk_alpha_type, to_sk_color_type_or_panic};
use ferroui_base::platform::surfaces::{IFramebufferPlatformSurface, IFramebufferRenderTarget};
use ferroui_base::platform::{
    IDrawingContextImpl, ILockedFramebuffer, IRenderTarget, PlatformRenderTargetState,
    RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use skia_safe::{surfaces, Bitmap, ColorType, ImageInfo, PixelGeometry, Surface, SurfaceProps, SurfacePropsFlags};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn surface_props() -> SurfaceProps {
    SurfaceProps::new(SurfacePropsFlags::default(), PixelGeometry::RGBH)
}

/// The surface over the locked framebuffer, kept between frames while the
/// framebuffer stays the same.
struct SurfaceState {
    current_image_info: Option<ImageInfo>,
    current_framebuffer_address: *mut u8,
    framebuffer_surface: Option<Surface>,
    conversion_shim: Option<Rc<PixelFormatConversionShim>>,
}

/// Skia render target that renders to a framebuffer surface. No GPU
/// acceleration available.
pub struct FramebufferRenderTarget {
    use_scaled_drawing: bool,
    state: RefCell<SurfaceState>,
    render_target: RefCell<Option<Rc<dyn IFramebufferRenderTarget>>>,
    had_conversion_shim: Cell<bool>,
}

impl FramebufferRenderTarget {
    /// Creates a framebuffer render target for a framebuffer surface.
    pub fn new(platform_surface: &dyn IFramebufferPlatformSurface, use_scaled_drawing: bool) -> Self {
        Self::from_render_target(platform_surface.create_framebuffer_render_target(), use_scaled_drawing)
    }

    /// Creates a framebuffer render target over the render target of a
    /// framebuffer surface.
    pub fn from_render_target(render_target: Rc<dyn IFramebufferRenderTarget>, use_scaled_drawing: bool) -> Self {
        Self {
            use_scaled_drawing,
            state: RefCell::new(SurfaceState {
                current_image_info: None,
                current_framebuffer_address: std::ptr::null_mut(),
                framebuffer_surface: None,
                conversion_shim: None,
            }),
            render_target: RefCell::new(Some(render_target)),
            had_conversion_shim: Cell::new(false),
        }
    }

    /// Checks whether the given infos are compatible: a surface created for
    /// one can be reused for the other.
    fn are_image_infos_compatible(current_image_info: &ImageInfo, desired_image_info: &ImageInfo) -> bool {
        current_image_info.width() == desired_image_info.width()
            && current_image_info.height() == desired_image_info.height()
            && current_image_info.color_type() == desired_image_info.color_type()
    }

    /// Creates the surface over the framebuffer if the current one is not
    /// compatible.
    fn create_surface(&self, desired_image_info: &ImageInfo, framebuffer: &dyn ILockedFramebuffer) {
        let mut state = self.state.borrow_mut();
        let address = framebuffer.address();

        if state.framebuffer_surface.is_some()
            && state
                .current_image_info
                .as_ref()
                .is_some_and(|current| Self::are_image_infos_compatible(current, desired_image_info))
            && state.current_framebuffer_address == address
        {
            return;
        }

        Self::free_surface(&mut state);

        state.current_framebuffer_address = address;

        // A surface with a width/height of 0 is invalid and can't be created.
        if desired_image_info.width() <= 0 || desired_image_info.height() <= 0 {
            panic!(
                "Unable to create a surface with size {}x{}",
                desired_image_info.width(),
                desired_image_info.height()
            );
        }

        let row_bytes = framebuffer.row_bytes().max(0) as usize;
        let mut surface = None;

        if !address.is_null() && row_bytes >= desired_image_info.min_row_bytes() {
            // SAFETY: the framebuffer contract guarantees `row_bytes * height`
            // bytes of memory at `address` that stay valid until the
            // framebuffer is disposed. The surface draws into that memory
            // only through drawing contexts created while the framebuffer is
            // locked; it is recreated when the address changes.
            let pixels =
                unsafe { std::slice::from_raw_parts_mut(address, row_bytes * desired_image_info.height() as usize) };

            surface = surfaces::wrap_pixels(desired_image_info, pixels, row_bytes, Some(&surface_props()))
                // SAFETY: releasing the borrow turns the lifetime-bound
                // surface into an owned one; the validity of the pixels is
                // upheld as described above.
                .map(|surface| unsafe { surface.release() });
        }

        // If the surface cannot be created, try to create a compatibility
        // shim first.
        if surface.is_none() {
            let shim = Rc::new(PixelFormatConversionShim::new(desired_image_info.clone(), address, row_bytes));
            surface = Some(shim.surface.clone());
            state.conversion_shim = Some(shim);
        }

        state.framebuffer_surface = surface;
        state.current_image_info = Some(desired_image_info.clone());
    }

    /// Frees the surface over the framebuffer.
    fn free_surface(state: &mut SurfaceState) {
        state.conversion_shim = None;
        state.framebuffer_surface = None;
        state.current_framebuffer_address = std::ptr::null_mut();
    }
}

impl IRenderTarget for FramebufferRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties {
            retains_previous_frame_contents: !self.had_conversion_shim.get()
                && self.render_target.borrow().as_ref().is_some_and(|target| target.retains_frame_contents()),
            is_suitable_for_direct_rendering: true,
        }
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let render_target =
            self.render_target.borrow().clone().unwrap_or_else(|| panic!("FramebufferRenderTarget has been disposed"));

        let (framebuffer, lock_properties) = render_target.lock(scene_info);
        let size = framebuffer.size();
        let framebuffer_image_info = ImageInfo::new(
            (size.width, size.height),
            to_sk_color_type_or_panic(framebuffer.format()),
            to_sk_alpha_type(framebuffer.alpha_format()),
            None,
        );

        self.create_surface(&framebuffer_image_info, &*framebuffer);

        let (mut surface, conversion_shim) = {
            let state = self.state.borrow();
            (state.framebuffer_surface.clone().expect("the surface was just created"), state.conversion_shim.clone())
        };

        self.had_conversion_shim.set(self.had_conversion_shim.get() || conversion_shim.is_some());

        {
            let canvas = surface.canvas();
            canvas.restore_to_count(1);
            canvas.save();
            canvas.reset_matrix();
        }

        let create_info = CreateInfo {
            surface: Some(surface),
            dpi: framebuffer.dpi(),
            scale_drawing_to_dpi: self.use_scaled_drawing,
            ..CreateInfo::default()
        };

        let properties = RenderTargetDrawingContextProperties {
            previous_frame_is_retained: !self.had_conversion_shim.get() && lock_properties.previous_frame_is_retained,
        };

        let mut disposables: Vec<Box<dyn FnOnce()>> = Vec::with_capacity(2);
        if let Some(shim) = conversion_shim {
            disposables.push(Box::new(move || shim.copy_surface()));
        }
        disposables.push(Box::new(move || framebuffer.dispose()));

        (Box::new(DrawingContextImpl::new(create_info, disposables)), properties)
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        match &*self.render_target.borrow() {
            Some(render_target) => render_target.state(),
            None => PlatformRenderTargetState::DISPOSED,
        }
    }

    fn dispose(&self) {
        if let Some(render_target) = self.render_target.borrow_mut().take() {
            render_target.dispose();
        }
        Self::free_surface(&mut self.state.borrow_mut());
    }
}

/// Converts a non-compatible pixel format using an intermediate surface: the
/// frame is drawn into a surface in the platform's native format and copied
/// (with conversion) into the framebuffer when the frame is finished.
struct PixelFormatConversionShim {
    surface: Surface,
    destination_info: ImageInfo,
    framebuffer_address: *mut u8,
    framebuffer_row_bytes: usize,
}

impl PixelFormatConversionShim {
    fn new(destination_info: ImageInfo, framebuffer_address: *mut u8, framebuffer_row_bytes: usize) -> Self {
        // Create the surface using the default platform settings.
        if !Self::can_copy_to(destination_info.color_type()) {
            panic!(
                "Unable to create pixel format shim for conversion from {:?} to {:?}",
                ColorType::N32,
                destination_info.color_type()
            );
        }

        let info = ImageInfo::new_n32_premul(destination_info.dimensions(), None);
        let surface = surfaces::raster(&info, None, Some(&surface_props())).unwrap_or_else(|| {
            panic!(
                "Unable to create pixel format shim surface for conversion from {:?} to {:?}",
                ColorType::N32,
                destination_info.color_type()
            )
        });

        Self { surface, destination_info, framebuffer_address, framebuffer_row_bytes }
    }

    /// Whether pixels can be converted to the given color type.
    fn can_copy_to(color_type: ColorType) -> bool {
        if color_type == ColorType::Unknown {
            return false;
        }

        let info = ImageInfo::new_n32_premul((1, 1), None).with_color_type(color_type);
        Bitmap::new().try_alloc_pixels_info(&info, None)
    }

    /// Copies the frame into the framebuffer, converting the pixel format.
    fn copy_surface(&self) {
        if self.framebuffer_address.is_null() {
            return;
        }

        let row_bytes = self.framebuffer_row_bytes.max(self.destination_info.min_row_bytes());

        // SAFETY: called while the framebuffer the shim was created for is
        // still locked (the copy runs before the framebuffer is disposed),
        // so `row_bytes * height` bytes at the address are valid.
        let pixels = unsafe {
            std::slice::from_raw_parts_mut(
                self.framebuffer_address,
                row_bytes * self.destination_info.height() as usize,
            )
        };

        self.surface.clone().read_pixels(&self.destination_info, pixels, row_bytes, (0, 0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::{AlphaType, Color};

    #[test]
    fn conversion_shim_copies_the_frame_in_the_destination_format() {
        // The frame is drawn in the native format and converted to
        // unpremultiplied RGBA when it is copied to the framebuffer.
        let destination_info = ImageInfo::new((4, 4), ColorType::RGBA8888, AlphaType::Unpremul, None);
        let row_bytes = 4 * 4 + 4;
        let mut framebuffer = vec![0u8; row_bytes * 4];

        let shim = PixelFormatConversionShim::new(destination_info, framebuffer.as_mut_ptr(), row_bytes);
        shim.surface.clone().canvas().clear(Color::from_argb(128, 255, 0, 0));
        shim.copy_surface();
        drop(shim);

        for y in 0..4 {
            let row = &framebuffer[y * row_bytes..];
            assert_eq!([255, 0, 0, 128], row[4..8]);
            // The padding after the row is untouched.
            assert_eq!([0, 0, 0, 0], row[16..20]);
        }
    }

    #[test]
    fn conversion_shim_rejects_unknown_color_types() {
        assert!(!PixelFormatConversionShim::can_copy_to(ColorType::Unknown));
        assert!(PixelFormatConversionShim::can_copy_to(ColorType::RGB565));
    }
}
