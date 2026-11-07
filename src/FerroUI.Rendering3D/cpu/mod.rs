//! The CPU reference rasterizer (`docs/porting/media3d.md`, sections 3.9 and 4, step D3): a
//! deterministic renderer of a [`Scene3DSnapshot`] in safe Rust, the reference the GPU executors
//! are compared with and the renderer of the surfaces without a GPU (Skia raster, headless, the
//! CPU mode of Vello).
//!
//! The pipeline is that of the GPU executors, stage for stage:
//!
//! 1. **Uniforms.** [`FrameUniforms`] and one [`DrawUniforms`] per draw, the structures the GPU
//!    uploads.
//! 2. **Vertex stage.** Each vertex of a model through its world matrix (the world position and
//!    the normal through the normal matrix, at unit length) and the camera's
//!    `view * projection`, once per model.
//! 3. **Clipping** in homogeneous coordinates against the near and far planes (and a guard band;
//!    `clip.rs`), the polygon drawn as a fan.
//! 4. **Setup.** The perspective division, the viewport (y down) and the vertices snapped to
//!    1/256 of a sample; the winding gives the face: counterclockwise on the screen is the front
//!    (WPF), and a face without layers is culled.
//! 5. **Rasterization** by edge functions in integers with the top-left rule (a sample on an edge
//!    shared by two triangles is covered by exactly one), over the bounding box of the triangle
//!    within a tile of 64 by 64 pixels; the triangles are binned to the tiles once.
//! 6. **Depth.** An `f32` depth buffer (z over w, from 0 at the near plane to 1 at the far
//!    plane), cleared to 1: the first diffuse layer of a material writes it with the test
//!    `Less`; the other layers test `LessEqual` and do not write it.
//! 7. **Fragment stage** per sample: the attributes interpolated perspective-correctly, the
//!    lighting and colour of the layer ([`crate::lighting::shade`]).
//! 8. **Blend.** A diffuse layer premultiplied source-over, a specular or emissive layer added
//!    (`One, One`), each channel clamped to 0..1 as an RGBA8 target stores it.
//! 9. **Resolve.** 4x supersampling on an ordered grid (samples at a quarter and three quarters
//!    of a pixel each way), the approximation of 4x MSAA, averaged to the output; 1x for aliased
//!    edges. The resolve raises the alpha of a pixel to its largest colour channel, so added
//!    light over a transparent pixel stays a valid premultiplied colour.
//!
//! The output is premultiplied RGBA8, top row first, transparent where nothing was drawn.
//! Nothing is allocated per pixel or per triangle: the buffers of a [`CpuRasterizer`] are kept
//! from one render to the next.

mod clip;

use crate::lighting::shade;
use crate::uniforms::{transform_point, BrushSampling, DrawUniforms, Face, FrameUniforms, LayerKind};
use clip::{clip_triangle, ClipVertex, Clipped, ATTRIBUTES};
use ferroui_base::rendering::scene3d::{Scene3DMaterialLayer, Scene3DSnapshot};
use ferroui_base::PixelSize;
use std::cell::Cell;
use std::ops::Range;

/// The largest side of an image the rasterizer makes, in pixels: larger sizes are clamped (the
/// image is then drawn scaled up), which keeps the fixed-point coordinates in range.
pub const MAX_SIDE: u32 = 8192;

/// The side of a tile, in output pixels.
const TILE: usize = 64;

/// The bits of the fractional part of a fixed-point coordinate, in samples.
const SUBPIXEL_BITS: u32 = 8;
/// One sample in fixed point.
const SUBPIXEL_ONE: i64 = 1 << SUBPIXEL_BITS;
/// Half a sample in fixed point: the centre of a sample.
const SUBPIXEL_HALF: i64 = SUBPIXEL_ONE / 2;

ferroui_base::ferro_thread_local! {
    /// The scenes rasterized on this thread.
    static RASTERIZED: Cell<u64> = const { Cell::new(0) };
}

/// How many scenes the CPU reference rasterized on this thread (each call of
/// [`CpuRasterizer::render`]): what the tests of the output cache count.
pub fn scenes_rasterized() -> u64 {
    RASTERIZED.with(Cell::get)
}

/// The image of a scene: premultiplied RGBA8, four bytes a pixel, rows from the top without
/// padding.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scene3DImage {
    /// The width in pixels.
    pub width: u32,
    /// The height in pixels.
    pub height: u32,
    /// The pixels.
    pub rgba: Vec<u8>,
}

impl Scene3DImage {
    /// The premultiplied RGBA of the pixel at `x`, `y`.
    ///
    /// # Panics
    /// Panics for a pixel outside the image.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(x < self.width && y < self.height, "the pixel {x}, {y} is outside the image");
        let index = (y as usize * self.width as usize + x as usize) * 4;
        [self.rgba[index], self.rgba[index + 1], self.rgba[index + 2], self.rgba[index + 3]]
    }

    /// Whether every pixel is transparent (0, 0, 0, 0).
    pub fn is_transparent(&self) -> bool {
        self.rgba.iter().all(|&byte| byte == 0)
    }
}

/// How a scene is rasterized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CpuRenderOptions {
    /// Whether edges are antialiased: 4x supersampling (the approximation of 4x MSAA); `false`
    /// takes one sample at the centre of each pixel (`RenderOptions.EdgeMode` `Aliased`).
    pub antialiased: bool,
}

impl Default for CpuRenderOptions {
    fn default() -> Self {
        CpuRenderOptions { antialiased: true }
    }
}

/// Renders `scene` into a new image of `size` pixels with a new rasterizer; see
/// [`CpuRasterizer::render`].
pub fn render(scene: &Scene3DSnapshot, size: PixelSize, scale: f64, options: CpuRenderOptions) -> Scene3DImage {
    CpuRasterizer::new().render(scene, size, scale, options)
}

/// A triangle after setup, in the fixed-point sample coordinates of the target.
#[derive(Clone, Copy)]
struct SetupTriangle {
    /// For each edge (opposite vertex 0, 1, 2), the change of its function per sample in x...
    step_x: [i64; 3],
    /// ...and in y.
    step_y: [i64; 3],
    /// The edge functions at the centre of sample (0, 0).
    origin: [i64; 3],
    /// What is added to an edge function before the test `>= 0`: 0 for a top or left edge, -1
    /// for the others (the top-left rule).
    bias: [i64; 3],
    /// One over twice the area, the edge functions to barycentric weights.
    inverse_area: f32,
    /// Depth (z over w) of the vertices.
    depth: [f32; 3],
    /// One over w of the vertices, for the perspective-correct weights.
    inverse_w: [f32; 3],
    /// The attributes of the vertices.
    attributes: [[f32; ATTRIBUTES]; 3],
    /// The samples the triangle may cover: the first and last column and row, inclusive.
    bounds: [i32; 4],
    /// Whether the triangle is a front face.
    front: bool,
}

/// A draw: one layer of one model on one side of its faces.
struct Pass {
    /// The triangles of the model (`CpuRasterizer::triangles`).
    triangles: Range<usize>,
    /// The faces the layer is drawn on.
    face: Face,
    /// The uniforms of the draw.
    draw: DrawUniforms,
    /// Whether the draw writes depth (with the test `Less`); otherwise it tests `LessEqual`
    /// without writing.
    writes_depth: bool,
    /// Whether the draw is added (`One, One`) rather than blended source-over.
    additive: bool,
}

/// The CPU reference rasterizer, with the buffers it keeps from one render to the next.
#[derive(Default)]
pub struct CpuRasterizer {
    vertices: Vec<ClipVertex>,
    triangles: Vec<SetupTriangle>,
    passes: Vec<Pass>,
    tile_offsets: Vec<u32>,
    tile_cursor: Vec<u32>,
    tile_triangles: Vec<u32>,
    color: Vec<[f32; 4]>,
    depth: Vec<f32>,
}

/// The geometry of a target: its pixels, its samples and its tiles.
#[derive(Clone, Copy)]
struct Target {
    width: usize,
    /// Samples per pixel in each direction (2, or 1 for aliased edges).
    samples: usize,
    sample_width: usize,
    sample_height: usize,
    tiles_x: usize,
    tiles_y: usize,
}

impl Target {
    fn tile_samples(&self) -> usize {
        TILE * self.samples
    }
}

impl CpuRasterizer {
    /// A rasterizer without buffers yet.
    pub fn new() -> CpuRasterizer {
        CpuRasterizer::default()
    }

    /// Renders `scene` into an image of `size` pixels (each side clamped to 0..[`MAX_SIDE`]).
    /// The projection of the camera covers the whole image. `scale` is the number of pixels per
    /// unit of the viewport; the rasterizer itself depends only on the size (the brushes of
    /// step D6 use the scale for the resolution of what they draw), and the output cache keys on
    /// it. A scene without a camera, or a size without pixels, gives a transparent image.
    pub fn render(
        &mut self,
        scene: &Scene3DSnapshot,
        size: PixelSize,
        scale: f64,
        options: CpuRenderOptions,
    ) -> Scene3DImage {
        let _ = scale;
        RASTERIZED.with(|count| count.set(count.get() + 1));
        let width = size.width.clamp(0, MAX_SIDE as i32) as u32;
        let height = size.height.clamp(0, MAX_SIDE as i32) as u32;
        let mut image = Scene3DImage { width, height, rgba: vec![0; width as usize * height as usize * 4] };
        if width == 0 || height == 0 {
            return image;
        }
        let Some(frame) = FrameUniforms::of(scene) else { return image };
        let samples = if options.antialiased { 2 } else { 1 };
        let (width, height) = (width as usize, height as usize);
        let target = Target {
            width,
            samples,
            sample_width: width * samples,
            sample_height: height * samples,
            tiles_x: width.div_ceil(TILE),
            tiles_y: height.div_ceil(TILE),
        };
        self.prepare(scene, &frame, &target);
        self.bin(&target);
        self.draw_tiles(&frame, &target, &mut image.rgba);
        image
    }

    /// The vertex stage, the clipping and the setup of every model, and its passes.
    fn prepare(&mut self, scene: &Scene3DSnapshot, frame: &FrameUniforms, target: &Target) {
        self.triangles.clear();
        self.passes.clear();
        for model in &scene.models {
            // The vertex stage, once per model: the uniforms of any layer have its matrices. A
            // model without layers draws nothing.
            let Some(first) = model.front.first().or(model.back.first()) else { continue };
            let mesh = &model.mesh;
            let (matrices, _) = DrawUniforms::of(model, first, Face::Front);
            self.vertices.clear();
            for (index, position) in mesh.positions.iter().enumerate() {
                let world = transform_point(&matrices.world, [position.x, position.y, position.z, 1.0]);
                let clip = transform_point(&frame.view_projection, world);
                let normal = mesh.normals.get(index).map_or([0.0; 3], |n| {
                    let n = transform_point(&matrices.normal_matrix, [n.x, n.y, n.z, 0.0]);
                    crate::math::normalize([n[0], n[1], n[2]])
                });
                let uv = mesh.texture_coordinates.get(index).map_or([0.0; 2], |uv| [uv.x, uv.y]);
                self.vertices.push(ClipVertex {
                    clip,
                    attributes: [world[0], world[1], world[2], normal[0], normal[1], normal[2], uv[0], uv[1]],
                });
            }

            let start = self.triangles.len();
            let (draws_front, draws_back) = (!model.front.is_empty(), !model.back.is_empty());
            for indices in mesh.indices.chunks_exact(3) {
                let (Some(a), Some(b), Some(c)) = (
                    self.vertices.get(indices[0] as usize),
                    self.vertices.get(indices[1] as usize),
                    self.vertices.get(indices[2] as usize),
                ) else {
                    continue;
                };
                let triangle = [*a, *b, *c];
                match clip_triangle(&triangle) {
                    Clipped::Outside => {}
                    Clipped::Inside => {
                        push_setup(&mut self.triangles, &triangle, target, draws_front, draws_back);
                    }
                    Clipped::Polygon(polygon) => {
                        let vertices = polygon.as_slice();
                        for i in 1..vertices.len() - 1 {
                            let fan = [vertices[0], vertices[i], vertices[i + 1]];
                            push_setup(&mut self.triangles, &fan, target, draws_front, draws_back);
                        }
                    }
                }
            }
            let triangles = start..self.triangles.len();
            if triangles.is_empty() {
                continue;
            }
            for (face, layers) in [(Face::Front, &model.front), (Face::Back, &model.back)] {
                add_passes(&mut self.passes, model, layers, face, triangles.clone());
            }
        }
    }

    /// Bins the triangles to the tiles their bounds touch (a counting sort, in triangle order).
    fn bin(&mut self, target: &Target) {
        let tiles = target.tiles_x * target.tiles_y;
        let tile_samples = target.tile_samples() as i32;
        self.tile_offsets.clear();
        self.tile_offsets.resize(tiles + 1, 0);
        let range = |bounds: &[i32; 4]| {
            (
                (bounds[0] / tile_samples) as usize,
                (bounds[1] / tile_samples) as usize,
                (bounds[2] / tile_samples) as usize,
                (bounds[3] / tile_samples) as usize,
            )
        };
        for triangle in &self.triangles {
            let (x0, y0, x1, y1) = range(&triangle.bounds);
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    self.tile_offsets[ty * target.tiles_x + tx + 1] += 1;
                }
            }
        }
        for tile in 0..tiles {
            self.tile_offsets[tile + 1] += self.tile_offsets[tile];
        }
        self.tile_triangles.clear();
        self.tile_triangles.resize(self.tile_offsets[tiles] as usize, 0);
        self.tile_cursor.clear();
        self.tile_cursor.extend_from_slice(&self.tile_offsets[..tiles]);
        for (index, triangle) in self.triangles.iter().enumerate() {
            let (x0, y0, x1, y1) = range(&triangle.bounds);
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    let cursor = &mut self.tile_cursor[ty * target.tiles_x + tx];
                    self.tile_triangles[*cursor as usize] = index as u32;
                    *cursor += 1;
                }
            }
        }
    }

    /// Draws every pass into each tile and resolves the tile into `rgba`.
    fn draw_tiles(&mut self, frame: &FrameUniforms, target: &Target, rgba: &mut [u8]) {
        let tile_samples = target.tile_samples();
        self.color.clear();
        self.color.resize(tile_samples * tile_samples, [0.0; 4]);
        self.depth.clear();
        self.depth.resize(tile_samples * tile_samples, 1.0);
        for ty in 0..target.tiles_y {
            for tx in 0..target.tiles_x {
                let tile = ty * target.tiles_x + tx;
                let list = &self.tile_triangles[self.tile_offsets[tile] as usize..self.tile_offsets[tile + 1] as usize];
                let x0 = tx * tile_samples;
                let y0 = ty * tile_samples;
                let area = TileArea {
                    x0: x0 as i32,
                    y0: y0 as i32,
                    x1: ((x0 + tile_samples).min(target.sample_width) - 1) as i32,
                    y1: ((y0 + tile_samples).min(target.sample_height) - 1) as i32,
                    stride: tile_samples,
                };
                self.color.fill([0.0; 4]);
                self.depth.fill(1.0);
                if !list.is_empty() {
                    for pass in &self.passes {
                        // The triangles of the pass in this tile: the list is in triangle order.
                        let start = list.partition_point(|&index| (index as usize) < pass.triangles.start);
                        let end = list.partition_point(|&index| (index as usize) < pass.triangles.end);
                        for &index in &list[start..end] {
                            let triangle = &self.triangles[index as usize];
                            if triangle.front == (pass.face == Face::Front) {
                                rasterize(triangle, pass, frame, &area, &mut self.color, &mut self.depth);
                            }
                        }
                    }
                }
                resolve(&self.color, &area, target, rgba);
            }
        }
    }
}

/// The passes of the layers of one side of a model, in order. The first diffuse layer writes
/// depth (the first layer when the material has no diffuse layer); the others test against it.
fn add_passes(
    passes: &mut Vec<Pass>,
    model: &ferroui_base::rendering::scene3d::Scene3DModel,
    layers: &[Scene3DMaterialLayer],
    face: Face,
    triangles: Range<usize>,
) {
    let depth_layer = layers
        .iter()
        .position(|layer| LayerKind::of(layer.kind) == LayerKind::Diffuse)
        .unwrap_or(0);
    for (index, layer) in layers.iter().enumerate() {
        let (draw, sampling) = DrawUniforms::of(model, layer, face);
        if sampling == BrushSampling::Unsampled {
            crate::note_unsampled_brush();
        }
        passes.push(Pass {
            triangles: triangles.clone(),
            face,
            draw,
            writes_depth: index == depth_layer,
            additive: draw.kind() != LayerKind::Diffuse,
        });
    }
}

/// The perspective division, the viewport and the snap of a triangle, appended when it covers a
/// sample and its face is drawn.
fn push_setup(
    triangles: &mut Vec<SetupTriangle>,
    vertices: &[ClipVertex; 3],
    target: &Target,
    draws_front: bool,
    draws_back: bool,
) {
    let mut x = [0i64; 3];
    let mut y = [0i64; 3];
    let mut depth = [0f32; 3];
    let mut inverse_w = [0f32; 3];
    let (sample_width, sample_height) = (target.sample_width as f64, target.sample_height as f64);
    for (i, vertex) in vertices.iter().enumerate() {
        let w = vertex.clip[3];
        inverse_w[i] = 1.0 / w;
        let ndc_x = f64::from(vertex.clip[0]) / f64::from(w);
        let ndc_y = f64::from(vertex.clip[1]) / f64::from(w);
        depth[i] = vertex.clip[2] * inverse_w[i];
        // NDC has y up; the target's rows go down.
        x[i] = ((ndc_x * 0.5 + 0.5) * sample_width * SUBPIXEL_ONE as f64).round() as i64;
        y[i] = ((0.5 - ndc_y * 0.5) * sample_height * SUBPIXEL_ONE as f64).round() as i64;
    }
    let mut order = [0usize, 1, 2];
    let area = (x[1] - x[0]) * (y[2] - y[0]) - (x[2] - x[0]) * (y[1] - y[0]);
    if area == 0 {
        return;
    }
    // With rows going down, a negative area is a counterclockwise triangle on the screen: a
    // front face (WPF). It is turned to the positive orientation the edge functions use.
    let front = area < 0;
    if (front && !draws_front) || (!front && !draws_back) {
        return;
    }
    if front {
        order = [0, 2, 1];
    }
    let (vx, vy) = (order.map(|i| x[i]), order.map(|i| y[i]));
    let area = area.abs();

    let mut bounds = [
        ((vx.iter().min().copied().unwrap_or(0) - SUBPIXEL_HALF) as f64 / SUBPIXEL_ONE as f64).ceil() as i64,
        ((vy.iter().min().copied().unwrap_or(0) - SUBPIXEL_HALF) as f64 / SUBPIXEL_ONE as f64).ceil() as i64,
        ((vx.iter().max().copied().unwrap_or(0) - SUBPIXEL_HALF) as f64 / SUBPIXEL_ONE as f64).floor() as i64,
        ((vy.iter().max().copied().unwrap_or(0) - SUBPIXEL_HALF) as f64 / SUBPIXEL_ONE as f64).floor() as i64,
    ];
    bounds[0] = bounds[0].max(0);
    bounds[1] = bounds[1].max(0);
    bounds[2] = bounds[2].min(target.sample_width as i64 - 1);
    bounds[3] = bounds[3].min(target.sample_height as i64 - 1);
    if bounds[0] > bounds[2] || bounds[1] > bounds[3] {
        return;
    }

    let mut setup = SetupTriangle {
        step_x: [0; 3],
        step_y: [0; 3],
        origin: [0; 3],
        bias: [0; 3],
        inverse_area: 1.0 / area as f32,
        depth: order.map(|i| depth[i]),
        inverse_w: order.map(|i| inverse_w[i]),
        attributes: order.map(|i| vertices[i].attributes),
        bounds: bounds.map(|value| value as i32),
        front,
    };
    // Edge k is opposite vertex k: from vertex k + 1 to vertex k + 2. Its function at p is
    // (b - a) x (p - a), positive inside, and equal to the area at vertex k.
    for k in 0..3 {
        let (a, b) = ((k + 1) % 3, (k + 2) % 3);
        let (dx, dy) = (vx[b] - vx[a], vy[b] - vy[a]);
        setup.step_x[k] = -dy * SUBPIXEL_ONE;
        setup.step_y[k] = dx * SUBPIXEL_ONE;
        setup.origin[k] = dx * (SUBPIXEL_HALF - vy[a]) - dy * (SUBPIXEL_HALF - vx[a]);
        // In this orientation (clockwise as seen with rows going down) a top edge goes right and
        // a left edge goes up.
        let top_left = (dy == 0 && dx > 0) || dy < 0;
        setup.bias[k] = if top_left { 0 } else { -1 };
    }
    triangles.push(setup);
}

/// The samples of a tile, in the coordinates of the target.
struct TileArea {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    /// The samples in a row of the buffers of the tile.
    stride: usize,
}

/// Draws one pass of one triangle into the samples of a tile.
fn rasterize(
    triangle: &SetupTriangle,
    pass: &Pass,
    frame: &FrameUniforms,
    area: &TileArea,
    color: &mut [[f32; 4]],
    depth: &mut [f32],
) {
    let x0 = triangle.bounds[0].max(area.x0);
    let y0 = triangle.bounds[1].max(area.y0);
    let x1 = triangle.bounds[2].min(area.x1);
    let y1 = triangle.bounds[3].min(area.y1);
    if x0 > x1 || y0 > y1 {
        return;
    }
    let width = (x1 - x0 + 1) as usize;
    for sy in y0..=y1 {
        let row_start = (sy - area.y0) as usize * area.stride + (x0 - area.x0) as usize;
        let color_row = &mut color[row_start..row_start + width];
        let depth_row = &mut depth[row_start..row_start + width];
        let mut edges = [0i64; 3];
        for k in 0..3 {
            edges[k] = triangle.origin[k] + triangle.step_x[k] * i64::from(x0) + triangle.step_y[k] * i64::from(sy);
        }
        for (dst, z_dst) in color_row.iter_mut().zip(depth_row.iter_mut()) {
            let inside = (edges[0] + triangle.bias[0]) | (edges[1] + triangle.bias[1]) | (edges[2] + triangle.bias[2]);
            if inside >= 0 {
                let weights = [
                    edges[0] as f32 * triangle.inverse_area,
                    edges[1] as f32 * triangle.inverse_area,
                    edges[2] as f32 * triangle.inverse_area,
                ];
                let z = weights[0] * triangle.depth[0] + weights[1] * triangle.depth[1] + weights[2] * triangle.depth[2];
                let passes = if pass.writes_depth { z < *z_dst } else { z <= *z_dst };
                if passes {
                    let attributes = interpolate(triangle, weights);
                    let normal = [attributes[3], attributes[4], attributes[5]];
                    let world = [attributes[0], attributes[1], attributes[2]];
                    let source = shade(frame, &pass.draw, normal, world);
                    blend(dst, source, pass.additive);
                    if pass.writes_depth {
                        *z_dst = z;
                    }
                }
            }
            for k in 0..3 {
                edges[k] += triangle.step_x[k];
            }
        }
    }
}

/// The attributes at the barycentric `weights` (in screen space), corrected for perspective.
#[inline]
fn interpolate(triangle: &SetupTriangle, weights: [f32; 3]) -> [f32; ATTRIBUTES] {
    let q = [
        weights[0] * triangle.inverse_w[0],
        weights[1] * triangle.inverse_w[1],
        weights[2] * triangle.inverse_w[2],
    ];
    let sum = q[0] + q[1] + q[2];
    let inverse = if sum != 0.0 { 1.0 / sum } else { 0.0 };
    let q = [q[0] * inverse, q[1] * inverse, q[2] * inverse];
    let mut result = [0f32; ATTRIBUTES];
    for (i, value) in result.iter_mut().enumerate() {
        *value = q[0] * triangle.attributes[0][i] + q[1] * triangle.attributes[1][i] + q[2] * triangle.attributes[2][i];
    }
    result
}

/// Blends a premultiplied source into a sample: source-over, or added; clamped as RGBA8 stores.
#[inline]
fn blend(dst: &mut [f32; 4], source: [f32; 4], additive: bool) {
    if additive {
        for i in 0..4 {
            dst[i] = (dst[i] + source[i]).clamp(0.0, 1.0);
        }
    } else {
        let keep = 1.0 - source[3];
        for i in 0..4 {
            dst[i] = (source[i] + dst[i] * keep).clamp(0.0, 1.0);
        }
    }
}

/// Averages the samples of each pixel of a tile into the output.
fn resolve(color: &[[f32; 4]], area: &TileArea, target: &Target, rgba: &mut [u8]) {
    let samples = target.samples;
    let px0 = area.x0 as usize / samples;
    let py0 = area.y0 as usize / samples;
    let px1 = (area.x1 as usize + 1) / samples;
    let py1 = (area.y1 as usize + 1) / samples;
    let factor = 1.0 / (samples * samples) as f32;
    for py in py0..py1 {
        let row = &mut rgba[(py * target.width + px0) * 4..(py * target.width + px1) * 4];
        for (offset, pixel) in row.chunks_exact_mut(4).enumerate() {
            let px = px0 + offset;
            let mut sum = [0f32; 4];
            for sy in 0..samples {
                let base = ((py * samples + sy) - area.y0 as usize) * area.stride + (px * samples - area.x0 as usize);
                for sample in &color[base..base + samples] {
                    for i in 0..4 {
                        sum[i] += sample[i];
                    }
                }
            }
            let mut value = sum.map(|channel| (channel * factor).clamp(0.0, 1.0));
            // Added light over a transparent sample: the alpha is raised to the colour, so the
            // pixel is a valid premultiplied colour.
            value[3] = value[3].max(value[0]).max(value[1]).max(value[2]);
            for i in 0..4 {
                pixel[i] = (value[i] * 255.0 + 0.5) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests;
