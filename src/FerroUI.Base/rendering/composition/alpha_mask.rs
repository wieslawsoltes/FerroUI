//! The alpha mask of an element (phase 7, not from upstream): what
//! `Shape.GetAlphaMask`, `TextBlock.GetAlphaMask` and `Image.GetAlphaMask` of
//! WinUI return, ported from `dxaml/xcp/components/graphics/AlphaMask.cpp` of
//! microsoft-ui-xaml.

use super::{CompositionBrush, CompositionStretch, CompositionSurfaceBrush, Compositor, ICompositionSurface, LoadedImageSurface};
use crate::media::imaging::{Bitmap, BitmapBlendingMode, RenderTargetBitmap};
use crate::media::RenderOptions;
use crate::platform::{AlphaFormat, PixelFormats};
use crate::reactive::IDisposable;
use crate::{LocatorExtensions, PixelSize, Rect, Vector, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The largest side of the image of a mask, in pixels. A mask that would be
/// larger is drawn smaller, and its brush scaled back up.
///
/// Deviation (winui-composition.md): WinUI limits the mask to the largest
/// texture of the device (16384 pixels with Direct3D 11); the port draws
/// the mask into a bitmap of the render interface and keeps it smaller.
pub const ALPHA_MASK_MAX_SIZE: u32 = 4096;

/// The side of the black bitmap the colour of the mask is replaced with.
const BLACK_SIDE: i32 = 4;

/// The alpha mask of an element: a [`CompositionSurfaceBrush`] that shows the
/// alpha of what the element draws, for a `DropShadow.Mask` and other
/// composition effects.
///
/// The element embeds one and forwards to it: [`get`](Self::get) from its
/// `get_alpha_mask`, [`update_if_available`](Self::update_if_available) at the
/// end of its `render` (WinUI's `NotifyRenderContent`).
///
/// As in WinUI the brush is created on the first request and stays the same
/// object for the life of the element; its surface is an image of the
/// element's own drawing at the scale of its transform to the window and of
/// the scaling of the window (WinUI's realization scale), with the colour
/// dropped (black with the alpha, as an alpha-only texture is sampled). The
/// image is drawn again each time the element draws its content again, so
/// it follows the text, the geometry, the source and the size; while the
/// element has no size, is not visible or is not in a tree, the brush has no
/// surface and paints nothing. Its `Stretch` is `Fill`, so a visual of the
/// size of the element shows the mask over itself.
#[derive(Default)]
pub struct AlphaMask {
    brush: RefCell<Option<CompositionSurfaceBrush>>,
    /// Whether the brush has a surface (shared with the subscriptions).
    connected: Rc<Cell<bool>>,
    /// Set while the element is drawn into the mask: the drawing calls the
    /// `render` of the element, which must not update the mask again.
    updating: Cell<bool>,
    /// The subscriptions that hide the mask when the element leaves its tree
    /// or stops being visible.
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl AlphaMask {
    pub fn new() -> Self {
        Self::default()
    }

    /// `GetAlphaMask`: the brush of the mask of `element`, created on the
    /// first call and drawn at once when the element can be drawn (WinUI's
    /// `AlphaMask::Ensure`). `None` when no compositor is available for the
    /// element.
    pub fn get(&self, element: &Visual) -> Option<Rc<CompositionBrush>> {
        if let Some(brush) = self.brush.borrow().as_ref() {
            return Some((**brush).clone());
        }
        let compositor = element
            .composition_visual()
            .map(|visual| visual.compositor().clone())
            .or_else(Compositor::try_get_default_compositor)?;
        let brush = compositor.create_surface_brush();
        brush.set_stretch(CompositionStretch::Fill);
        let result: Rc<CompositionBrush> = (*brush).clone();
        *self.brush.borrow_mut() = Some(brush);
        self.subscribe(element);
        self.update_if_available(element);
        Some(result)
    }

    /// Whether the brush was created.
    pub fn is_present(&self) -> bool {
        self.brush.borrow().is_some()
    }

    /// Whether the brush shows an image of the element.
    pub fn is_connected(&self) -> bool {
        self.connected.get()
    }

    /// Draws the mask of `element` again when the brush was created (WinUI's
    /// `AlphaMask::UpdateIfAvailable`, called by the render walk for an
    /// element whose content is drawn again): an element with a size, in a
    /// tree and visible gets a new image; otherwise the brush loses its
    /// surface.
    pub fn update_if_available(&self, element: &Visual) {
        if self.updating.get() {
            return;
        }
        let Some(brush) = self.brush.borrow().clone() else { return };

        let bounds = element.bounds();
        let (scale_x, scale_y) = realization_scale(element);
        let width = (scale_x * bounds.width).floor();
        let height = (scale_y * bounds.height).floor();
        let drawable = width >= 1.0 && height >= 1.0
            && element.is_attached_to_visual_tree()
            && element.is_effectively_visible()
            && crate::FerroLocator::current().get_service::<dyn crate::platform::IPlatformRenderInterface>().is_some();
        if !drawable {
            self.hide();
            return;
        }

        // Larger than the largest image: drawn smaller, and the brush scaled
        // back (`EnsureBackingSurface`, `RasterizeElement`).
        let largest = width.max(height);
        let adjusted = if largest > ALPHA_MASK_MAX_SIZE as f64 { ALPHA_MASK_MAX_SIZE as f64 / largest } else { 1.0 };
        let pixel_size = PixelSize::new(
            ((width * adjusted) as i32).clamp(1, ALPHA_MASK_MAX_SIZE as i32),
            ((height * adjusted) as i32).clamp(1, ALPHA_MASK_MAX_SIZE as i32),
        );
        // The DPI that maps the size of the element onto the pixels.
        let dpi = Vector::new(
            96.0 * pixel_size.width as f64 / bounds.width,
            96.0 * pixel_size.height as f64 / bounds.height,
        );

        self.updating.set(true);
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.updating);

        let image = RenderTargetBitmap::with_dpi(pixel_size, dpi);
        {
            let mut context = image.create_drawing_context();
            element.render(&mut context);
            // The colour goes: black where the element has alpha, in its
            // proportion.
            let black = black_bitmap();
            let options = context.push_render_options(RenderOptions {
                bitmap_blending_mode: BitmapBlendingMode::SourceIn,
                ..RenderOptions::default()
            });
            let side = BLACK_SIDE as f64;
            context.draw_image_with_rects(
                &black,
                Rect::new(1.0, 1.0, side - 2.0, side - 2.0),
                Rect::new(0.0, 0.0, bounds.width, bounds.height).inflate(1.0),
            );
            context.pop(options);
            context.dispose();
            black.dispose();
        }

        let compositor = brush.compositor().clone();
        let surface: Rc<dyn ICompositionSurface> = LoadedImageSurface::from_bitmap(&compositor, &image);
        image.dispose();
        brush.set_surface(Some(surface));
        let inverse = 1.0 / adjusted;
        brush.set_scale(Vector::new(inverse, inverse));
        self.connected.set(true);
    }

    /// Takes the surface from the brush (WinUI's `AlphaMask::Hide`): the
    /// element is not drawn, so its mask is empty.
    pub fn hide(&self) {
        if !self.connected.get() {
            return;
        }
        if let Some(brush) = self.brush.borrow().as_ref() {
            brush.set_surface(None);
        }
        self.connected.set(false);
    }

    /// Hides the mask when the element leaves its tree or stops being
    /// visible; it is drawn again when the element draws its content.
    fn subscribe(&self, element: &Visual) {
        let mut subscriptions = self.subscriptions.borrow_mut();
        if !subscriptions.is_empty() {
            return;
        }
        let weak = element.to_ref().downgrade();
        let brush = self.brush.borrow().clone();
        let connected = self.connected.clone();
        let hide_brush = move || {
            if !connected.replace(false) {
                return;
            }
            if let Some(brush) = &brush {
                brush.set_surface(None);
            }
        };
        let hide = Rc::new(hide_brush);
        let on_detached = {
            let hide = hide.clone();
            element.detached_from_visual_tree(move |_| hide())
        };
        let on_visibility = {
            let hide = hide.clone();
            element.is_effectively_visible_changed(move || {
                if weak.upgrade().is_some_and(|element| !element.is_effectively_visible()) {
                    hide();
                }
            })
        };
        subscriptions.push(on_detached);
        subscriptions.push(on_visibility);
    }
}

impl Drop for AlphaMask {
    fn drop(&mut self) {
        // The application may keep the brush: it loses the image of an
        // element that is gone (WinUI disconnects the surface).
        self.hide();
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

/// The scale an element is drawn at in its window: the scale of its
/// transform to the root of its tree times the scaling of the window
/// (WinUI's `ComputeRealizationTransform` and `GetScaleDimensions`).
fn realization_scale(element: &Visual) -> (f64, f64) {
    let scaling = element.presentation_source().map_or(1.0, |source| source.render_scaling());
    let transform = element
        .get_visual_root()
        .and_then(|root| element.transform_to_visual(&root))
        .unwrap_or(crate::Matrix::IDENTITY);
    let scale_x = (transform.m11 * transform.m11 + transform.m12 * transform.m12).sqrt();
    let scale_y = (transform.m21 * transform.m21 + transform.m22 * transform.m22).sqrt();
    let positive = |scale: f64| if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    (positive(scale_x) * scaling, positive(scale_y) * scaling)
}

/// A few opaque black pixels.
fn black_bitmap() -> Bitmap {
    let pixels = [0u8, 0, 0, 255].repeat((BLACK_SIDE * BLACK_SIDE) as usize);
    Bitmap::from_pixels(
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
        &pixels,
        PixelSize::new(BLACK_SIDE, BLACK_SIDE),
        Vector::new(96.0, 96.0),
        BLACK_SIDE * 4,
    )
}
