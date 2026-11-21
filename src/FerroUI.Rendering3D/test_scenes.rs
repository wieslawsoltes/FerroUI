//! Snapshots for the tests of the crate, made by hand: the matrices of WPF's cameras, quads and
//! layers of solid colours.

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, SharedBrush};
use ferroui_base::numerics::{Matrix4x4, Vector2, Vector3};
use ferroui_base::rendering::scene3d::{
    MeshData, Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DLight, Scene3DMaterialKind,
    Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot,
};
use ferroui_base::{PixelSize, Rect};
use std::sync::Arc;

/// A layer of `kind` with a solid colour brush of `color` (the material's `Color` white).
pub(crate) fn layer(kind: Scene3DMaterialKind, color: Color) -> Scene3DMaterialLayer {
    Scene3DMaterialLayer {
        kind,
        color: Color::from_argb(255, 255, 255, 255),
        ambient_color: Color::from_argb(255, 255, 255, 255),
        specular_power: if kind == Scene3DMaterialKind::Specular { 40.0 } else { 0.0 },
        brush: SharedBrush::from_brush(&ImmutableSolidColorBrush::new(color)),
    }
}

/// A diffuse layer of `color`.
pub(crate) fn diffuse(color: Color) -> Scene3DMaterialLayer {
    layer(Scene3DMaterialKind::Diffuse, color)
}

/// An emissive layer of `color`.
pub(crate) fn emissive(color: Color) -> Scene3DMaterialLayer {
    layer(Scene3DMaterialKind::Emissive, color)
}

/// A quad in the plane `z` from `(x0, y0)` to `(x1, y1)`, counterclockwise seen from +z (its
/// front faces the camera of [`orthographic`]), with normals along +z.
pub(crate) fn quad(id: u64, x0: f32, y0: f32, x1: f32, y1: f32, z: f32) -> Arc<MeshData> {
    let positions = vec![Vector3::new(x0, y0, z), Vector3::new(x1, y0, z), Vector3::new(x1, y1, z), Vector3::new(x0, y1, z)];
    let bounds = Scene3DBounds::of_points(positions.iter().copied());
    Arc::new(MeshData {
        id,
        version: 1,
        positions,
        normals: vec![Vector3::new(0.0, 0.0, 1.0); 4],
        normals_generated: false,
        texture_coordinates: vec![Vector2::new(0.0, 1.0), Vector2::new(1.0, 1.0), Vector2::new(1.0, 0.0), Vector2::new(0.0, 0.0)],
        indices: vec![0, 1, 2, 0, 2, 3],
        bounds,
    })
}

/// The same quad wound the other way: its front faces away from +z, so the camera sees its back.
pub(crate) fn reversed_quad(id: u64, x0: f32, y0: f32, x1: f32, y1: f32, z: f32) -> Arc<MeshData> {
    let mut mesh = (*quad(id, x0, y0, x1, y1, z)).clone();
    mesh.indices = vec![0, 2, 1, 0, 3, 2];
    mesh.normals = vec![Vector3::new(0.0, 0.0, -1.0); 4];
    Arc::new(mesh)
}

/// A model of `mesh` at the identity with `front` and `back` layers.
pub(crate) fn model(mesh: Arc<MeshData>, front: Vec<Scene3DMaterialLayer>, back: Vec<Scene3DMaterialLayer>) -> Scene3DModel {
    Scene3DModel { world: Matrix4x4::IDENTITY, world_bounds: mesh.bounds, mesh, front, back }
}

/// WPF's orthographic camera at (0, 0, 5) looking down -z with `Width` 2 (x and y from -1 to 1
/// cover the viewport of a square image), planes 1 and 10.
pub(crate) fn orthographic() -> Scene3DCamera {
    let (near, far, width, aspect) = (1.0f32, 10.0f32, 2.0f32, 1.0f32);
    let view = Matrix4x4::create_look_at(Vector3::new(0.0, 0.0, 5.0), Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    let m11 = 2.0 / width;
    let m33 = 1.0 / (near - far);
    let projection = Matrix4x4::new(m11, 0.0, 0.0, 0.0, 0.0, aspect * m11, 0.0, 0.0, 0.0, 0.0, m33, 0.0, 0.0, 0.0, near * m33, 1.0);
    Scene3DCamera {
        kind: Scene3DCameraKind::Orthographic,
        view,
        projection,
        position: Vector3::new(0.0, 0.0, 5.0),
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

/// WPF's perspective camera at `position` looking at the origin, with the horizontal field of
/// view `fov` in degrees and finite planes.
pub(crate) fn perspective(position: Vector3, fov: f32, near: f32, far: f32) -> Scene3DCamera {
    let view = Matrix4x4::create_look_at(position, Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    let m11 = 1.0 / (fov.to_radians() / 2.0).tan();
    let m33 = far / (near - far);
    let projection = Matrix4x4::new(m11, 0.0, 0.0, 0.0, 0.0, m11, 0.0, 0.0, 0.0, 0.0, m33, -1.0, 0.0, 0.0, near * m33, 0.0);
    Scene3DCamera {
        kind: Scene3DCameraKind::Perspective,
        view,
        projection,
        position,
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

/// A snapshot of `models` lit by `lights` through `camera`.
pub(crate) fn snapshot(camera: Scene3DCamera, models: Vec<Scene3DModel>, lights: Vec<Scene3DLight>) -> Arc<Scene3DSnapshot> {
    Arc::new(Scene3DSnapshot {
        structure_generation: 1,
        record_generation: 1,
        camera_generation: 1,
        viewport: Rect::new(0.0, 0.0, 100.0, 100.0),
        camera: Some(camera),
        models,
        lights,
        bounds: None,
    })
}

/// A white ambient light: a diffuse layer is then drawn in its own colour.
pub(crate) fn white_ambient() -> Scene3DLight {
    Scene3DLight::Ambient { color: Color::from_argb(255, 255, 255, 255) }
}

/// The square size of the tests.
pub(crate) const SIZE: PixelSize = PixelSize::new(100, 100);

/// A layer of `kind` that paints with `brush` (the material's `Color` white).
pub(crate) fn brush_layer(kind: Scene3DMaterialKind, brush: &dyn ferroui_base::media::IBrush) -> Scene3DMaterialLayer {
    Scene3DMaterialLayer { brush: SharedBrush::from_brush(brush), ..layer(kind, Color::from_argb(255, 255, 255, 255)) }
}

/// A bitmap of pixels in memory, readable through its framebuffer, whose version the tests bump.
pub(crate) struct PixelBitmap {
    size: PixelSize,
    format: ferroui_base::platform::PixelFormat,
    alpha: ferroui_base::platform::AlphaFormat,
    pixels: std::sync::Mutex<Vec<u8>>,
    version: std::sync::atomic::AtomicI32,
}

impl PixelBitmap {
    /// A bitmap of `width` by `height` premultiplied RGBA pixels.
    pub(crate) fn rgba(width: i32, height: i32, pixels: Vec<u8>) -> Arc<PixelBitmap> {
        PixelBitmap::with_format(
            width,
            height,
            pixels,
            ferroui_base::platform::PixelFormat::RGBA8888,
            ferroui_base::platform::AlphaFormat::Premul,
        )
    }

    /// A bitmap of `width` by `height` pixels of `format` and `alpha`.
    pub(crate) fn with_format(
        width: i32,
        height: i32,
        pixels: Vec<u8>,
        format: ferroui_base::platform::PixelFormat,
        alpha: ferroui_base::platform::AlphaFormat,
    ) -> Arc<PixelBitmap> {
        assert_eq!(pixels.len(), (width * height * 4) as usize);
        Arc::new(PixelBitmap {
            size: PixelSize::new(width, height),
            format,
            alpha,
            pixels: std::sync::Mutex::new(pixels),
            version: std::sync::atomic::AtomicI32::new(0),
        })
    }

    /// Replaces the pixels and bumps the version.
    pub(crate) fn set_pixels(&self, pixels: Vec<u8>) {
        *self.pixels.lock().unwrap() = pixels;
        self.version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl ferroui_base::platform::IBitmapImpl for PixelBitmap {
    fn dpi(&self) -> ferroui_base::Vector {
        ferroui_base::Vector::new(96.0, 96.0)
    }

    fn pixel_size(&self) -> PixelSize {
        self.size
    }

    fn version(&self) -> i32 {
        self.version.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn save(
        &self,
        _stream: &mut dyn std::io::Write,
        _options: &ferroui_base::media::imaging::BitmapEncoderOptions,
    ) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "a test bitmap is not saved"))
    }

    fn dispose(&self) {}


    fn as_readable_bitmap(&self) -> Option<&dyn ferroui_base::platform::IReadableBitmapImpl> {
        Some(self)
    }
}

impl ferroui_base::platform::IReadableBitmapImpl for PixelBitmap {
    fn format(&self) -> Option<ferroui_base::platform::PixelFormat> {
        Some(self.format)
    }

    fn alpha_format(&self) -> Option<ferroui_base::platform::AlphaFormat> {
        Some(self.alpha)
    }

    fn lock(&self) -> std::rc::Rc<dyn ferroui_base::platform::ILockedFramebuffer> {
        std::rc::Rc::new(PixelFramebuffer {
            data: std::cell::RefCell::new(self.pixels.lock().unwrap().clone()),
            size: self.size,
            format: self.format,
            alpha: self.alpha,
        })
    }
}

/// A copy of the pixels of a [`PixelBitmap`].
struct PixelFramebuffer {
    data: std::cell::RefCell<Vec<u8>>,
    size: PixelSize,
    format: ferroui_base::platform::PixelFormat,
    alpha: ferroui_base::platform::AlphaFormat,
}

impl ferroui_base::platform::ILockedFramebuffer for PixelFramebuffer {
    fn address(&self) -> *mut u8 {
        self.data.borrow_mut().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.data.borrow_mut());
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.size.width * 4
    }

    fn dpi(&self) -> ferroui_base::Vector {
        ferroui_base::Vector::new(96.0, 96.0)
    }

    fn format(&self) -> ferroui_base::platform::PixelFormat {
        self.format
    }

    fn alpha_format(&self) -> ferroui_base::platform::AlphaFormat {
        self.alpha
    }

    fn dispose(&self) {}
}

/// An image brush of `bitmap` stretched by `stretch` with `tile_mode` over `destination`, its
/// viewbox `source` (both relative to their boxes).
pub(crate) fn image_brush(
    bitmap: &Arc<PixelBitmap>,
    stretch: ferroui_base::media::Stretch,
    tile_mode: ferroui_base::media::TileMode,
    destination: ferroui_base::RelativeRect,
    source: ferroui_base::RelativeRect,
) -> ferroui_base::media::immutable::ImmutableImageBrush {
    let shared: Arc<ferroui_base::platform::SharedBitmapImpl> = bitmap.clone();
    let bitmap = std::rc::Rc::new(ferroui_base::media::imaging::Bitmap::from_impl(shared));
    ferroui_base::media::immutable::ImmutableImageBrush::new(
        Some(bitmap),
        ferroui_base::media::AlignmentX::Center,
        ferroui_base::media::AlignmentY::Center,
        Some(destination),
        1.0,
        None,
        ferroui_base::RelativePoint::TOP_LEFT,
        Some(source),
        stretch,
        tile_mode,
        None,
    )
}
