//! The constants of a 3D render, in one place for every renderer (`docs/porting/media3d.md`,
//! sections 3.2 and 3.4): the CPU reference reads them from these structures, and the GPU
//! executors upload the same structures to their uniform buffers, so the CPU and the GPU light a
//! scene with the same numbers.
//!
//! The layouts are `#[repr(C)]` and std140-compatible: every member is a `vec4` (four 4-byte
//! components, 16-byte aligned), a `mat4x4` (four `vec4` columns) or an array of structures of
//! whole `vec4`s, so the Rust layout is the std140 (and WGSL uniform) layout without padding
//! rules of its own. The tests assert the offsets and sizes; [`WGSL_DECLARATIONS`] is the same
//! layout in WGSL, for the shader of the executors.
//!
//! **Matrices.** The snapshot's matrices are row-vector matrices (`p * M`). A matrix is stored as
//! its four rows, which a column-major `mat4x4<f32>` reads as its columns: the shader's `M * v`
//! then computes the snapshot's `v * M`. `view_projection` is `view * projection` (the view
//! first), so the shader writes `frame.view_projection * (draw.world * vec4(position, 1.0))`.

use crate::math::{normalize, Float3, Float4};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::numerics::Matrix4x4;
use ferroui_base::rendering::scene3d::{
    Scene3DCameraKind, Scene3DLight, Scene3DMaterialKind, Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot,
};

/// The number of lights a render takes (WGSL's fixed loop bound). Further lights of a scene are
/// left out and counted ([`crate::lights_left_out`]).
pub const MAX_LIGHTS: usize = 16;

/// The stride of [`DrawUniforms`] in a buffer bound with dynamic offsets: the 256 bytes every
/// GPU API accepts as an offset alignment.
pub const DRAW_UNIFORMS_STRIDE: usize = 256;

/// The kind of a light in [`LightUniforms::metadata`] (its `x`, as a float, as ProGPU stores it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum LightKind {
    /// WPF's `AmbientLight`.
    Ambient = 0,
    /// WPF's `DirectionalLight`.
    Directional = 1,
    /// WPF's `PointLight`.
    Point = 2,
    /// WPF's `SpotLight`.
    Spot = 3,
}

impl LightKind {
    /// The value of the kind in the uniforms.
    pub fn uniform_value(self) -> f32 {
        self as u32 as f32
    }

    /// The kind of a value of the uniforms (rounded, as the shader's `u32(x + 0.5)`).
    pub fn from_uniform_value(value: f32) -> LightKind {
        match (value + 0.5) as u32 {
            0 => LightKind::Ambient,
            1 => LightKind::Directional,
            2 => LightKind::Point,
            _ => LightKind::Spot,
        }
    }
}

/// The kind of a material layer in [`DrawUniforms::material`] (its `x`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum LayerKind {
    /// A `DiffuseMaterial`: lit, blended source-over.
    Diffuse = 0,
    /// A `SpecularMaterial`: the highlights, added.
    Specular = 1,
    /// An `EmissiveMaterial`: the colour without light, added.
    Emissive = 2,
}

impl LayerKind {
    /// The kind of a layer of a snapshot.
    pub fn of(kind: Scene3DMaterialKind) -> LayerKind {
        match kind {
            Scene3DMaterialKind::Diffuse => LayerKind::Diffuse,
            Scene3DMaterialKind::Specular => LayerKind::Specular,
            Scene3DMaterialKind::Emissive => LayerKind::Emissive,
        }
    }

    /// The value of the kind in the uniforms.
    pub fn uniform_value(self) -> f32 {
        self as u32 as f32
    }

    /// The kind of a value of the uniforms.
    pub fn from_uniform_value(value: f32) -> LayerKind {
        match (value + 0.5) as u32 {
            0 => LayerKind::Diffuse,
            1 => LayerKind::Specular,
            _ => LayerKind::Emissive,
        }
    }
}

/// Which faces of a mesh a draw paints: the front faces (counterclockwise on the screen, WPF's
/// convention) with the `Material`, the back faces with the `BackMaterial`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    /// The faces wound counterclockwise as seen by the camera.
    Front,
    /// The faces wound clockwise as seen by the camera.
    Back,
}

/// A light in world space (ProGPU's `GpuLight3DRecord`): 80 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LightUniforms {
    /// `x`: the [`LightKind`]; `y`, `z`, `w`: 0.
    pub metadata: [f32; 4],
    /// The colour of the light, 0 to 1 (`w` is its alpha, which lighting does not read).
    pub color: [f32; 4],
    /// `xyz`: the position of a point or spot light; `w`: its range (`f32::MAX` for WPF's
    /// infinite range).
    pub position_range: [f32; 4],
    /// `xyz`: the direction the light travels in (directional and spot lights, unit length);
    /// `w`: the cosine of half the inner cone angle of a spot light.
    pub direction_inner_cos: [f32; 4],
    /// `xyz`: the constant, linear and quadratic attenuation of a point or spot light; `w`: the
    /// cosine of half the outer cone angle of a spot light.
    pub attenuation_outer_cos: [f32; 4],
}

impl LightUniforms {
    /// The uniforms of a light of a snapshot. The cone angles of a spot light are WPF's full
    /// angles in degrees: the uniforms hold the cosines of their halves.
    pub fn of(light: &Scene3DLight) -> LightUniforms {
        let kind = |kind: LightKind| [kind.uniform_value(), 0.0, 0.0, 0.0];
        let range = |range: f32| if range.is_finite() { range } else { f32::MAX };
        match *light {
            Scene3DLight::Ambient { color } => LightUniforms {
                metadata: kind(LightKind::Ambient),
                color: color_uniform(color),
                ..LightUniforms::default()
            },
            Scene3DLight::Directional { color, direction } => LightUniforms {
                metadata: kind(LightKind::Directional),
                color: color_uniform(color),
                direction_inner_cos: [direction.x, direction.y, direction.z, 0.0],
                ..LightUniforms::default()
            },
            Scene3DLight::Point { color, position, range: light_range, attenuation } => LightUniforms {
                metadata: kind(LightKind::Point),
                color: color_uniform(color),
                position_range: [position.x, position.y, position.z, range(light_range)],
                attenuation_outer_cos: [attenuation[0], attenuation[1], attenuation[2], 0.0],
                ..LightUniforms::default()
            },
            Scene3DLight::Spot {
                color,
                position,
                direction,
                range: light_range,
                attenuation,
                inner_cone_angle,
                outer_cone_angle,
            } => LightUniforms {
                metadata: kind(LightKind::Spot),
                color: color_uniform(color),
                position_range: [position.x, position.y, position.z, range(light_range)],
                direction_inner_cos: [direction.x, direction.y, direction.z, half_angle_cosine(inner_cone_angle)],
                attenuation_outer_cos: [
                    attenuation[0],
                    attenuation[1],
                    attenuation[2],
                    half_angle_cosine(outer_cone_angle),
                ],
            },
        }
    }

    /// The kind of the light.
    pub fn kind(&self) -> LightKind {
        LightKind::from_uniform_value(self.metadata[0])
    }
}

/// The cosine of half of a full cone angle in degrees.
fn half_angle_cosine(full_angle_degrees: f32) -> f32 {
    (full_angle_degrees * 0.5).to_radians().cos()
}

/// What every draw of a render shares: the camera and the lights. 1376 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameUniforms {
    /// `view * projection`, stored as its rows (see the module documentation).
    pub view_projection: [[f32; 4]; 4],
    /// `xyz` and `w = 1`: the position of the camera in the world (perspective and matrix
    /// cameras); `xyz` and `w = 0`: the unit direction toward an orthographic camera, the same for
    /// every point. The specular term reads it.
    pub camera_position: [f32; 4],
    /// `x`: the number of lights in `lights` (at most [`MAX_LIGHTS`]); `y`, `z`, `w`: 0.
    pub light_count: [u32; 4],
    /// The lights, in tree order; those past `light_count` are zero.
    pub lights: [LightUniforms; MAX_LIGHTS],
}

impl FrameUniforms {
    /// The uniforms of a snapshot; `None` without a camera (nothing is drawn, as in WPF). Lights
    /// past [`MAX_LIGHTS`] are left out, and counted with [`crate::lights_left_out`].
    pub fn of(scene: &Scene3DSnapshot) -> Option<FrameUniforms> {
        let camera = scene.camera.as_ref()?;
        let view_projection = camera.view * camera.projection;
        let camera_position = match camera.kind {
            Scene3DCameraKind::Orthographic => {
                // The camera's +z axis in the world: the third column of the view's rotation.
                let toward = normalize([camera.view.m13, camera.view.m23, camera.view.m33]);
                [toward[0], toward[1], toward[2], 0.0]
            }
            Scene3DCameraKind::Perspective | Scene3DCameraKind::Matrix => {
                [camera.position.x, camera.position.y, camera.position.z, 1.0]
            }
        };
        let mut lights = [LightUniforms::default(); MAX_LIGHTS];
        let count = scene.lights.len().min(MAX_LIGHTS);
        for (slot, light) in lights.iter_mut().zip(&scene.lights) {
            *slot = LightUniforms::of(light);
        }
        if scene.lights.len() > MAX_LIGHTS {
            crate::note_lights_left_out(scene.lights.len() - MAX_LIGHTS);
        }
        Some(FrameUniforms {
            view_projection: matrix_uniform(&view_projection),
            camera_position,
            light_count: [count as u32, 0, 0, 0],
            lights,
        })
    }

    /// The lights that are lit with.
    pub fn active_lights(&self) -> &[LightUniforms] {
        &self.lights[..(self.light_count[0] as usize).min(MAX_LIGHTS)]
    }
}

/// What a draw of one material layer of one model needs. 176 bytes, placed at multiples of
/// [`DRAW_UNIFORMS_STRIDE`] in a buffer of many draws.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawUniforms {
    /// The world matrix of the model, stored as its rows.
    pub world: [[f32; 4]; 4],
    /// The matrix of the normals (the inverse transpose of the upper 3x3 of `world`), stored
    /// as its rows; its fourth row and column are those of the identity.
    pub normal_matrix: [[f32; 4]; 4],
    /// The colour of the layer: the colour of its brush times its `Color`, with the opacity of
    /// the brush in the alpha; straight (not premultiplied), 0 to 1.
    pub color: [f32; 4],
    /// The `AmbientColor` of a diffuse layer (white for the other kinds), 0 to 1.
    pub ambient_color: [f32; 4],
    /// `x`: the [`LayerKind`]; `y`: the `SpecularPower`; `z`: the sign of the normals (1 for the
    /// front faces, -1 for the back faces, whose normals are turned around); `w`: 0.
    pub material: [f32; 4],
}

/// How the brush of a layer is sampled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrushSampling {
    /// A solid colour brush: its colour is in the uniforms.
    Solid,
    /// A brush the renderer cannot sample yet (a gradient, an image, a visual or drawing brush
    /// without a shared form; step D6 of the port): the layer is drawn with its `Color` and the
    /// opacity of the brush, and counted ([`crate::layers_with_unsampled_brushes`]).
    Unsampled,
}

impl DrawUniforms {
    /// The uniforms of a draw of `layer` of `model` on its `face` faces, with how its brush is
    /// sampled.
    pub fn of(model: &Scene3DModel, layer: &Scene3DMaterialLayer, face: Face) -> (DrawUniforms, BrushSampling) {
        let (color, sampling) = layer_color(layer);
        let kind = LayerKind::of(layer.kind);
        let face_sign = match face {
            Face::Front => 1.0,
            Face::Back => -1.0,
        };
        let uniforms = DrawUniforms {
            world: matrix_uniform(&model.world),
            normal_matrix: matrix_uniform(&normal_matrix(&model.world)),
            color,
            ambient_color: color_uniform(layer.ambient_color),
            material: [kind.uniform_value(), layer.specular_power, face_sign, 0.0],
        };
        (uniforms, sampling)
    }

    /// The kind of the layer.
    pub fn kind(&self) -> LayerKind {
        LayerKind::from_uniform_value(self.material[0])
    }
}

/// The colour a layer is drawn with: its brush's colour times its `Color`, the opacity of the
/// brush in the alpha (straight alpha, 0 to 1).
pub fn layer_color(layer: &Scene3DMaterialLayer) -> ([f32; 4], BrushSampling) {
    let material = color_uniform(layer.color);
    let (brush, opacity, sampling) = match layer.brush.as_ref() {
        Some(brush) => match brush.as_solid_color_brush() {
            Some(solid) => (color_uniform(solid.color()), brush.opacity() as f32, BrushSampling::Solid),
            None => ([1.0; 4], brush.opacity() as f32, BrushSampling::Unsampled),
        },
        None => ([1.0; 4], 1.0, BrushSampling::Unsampled),
    };
    let color = [
        brush[0] * material[0],
        brush[1] * material[1],
        brush[2] * material[2],
        brush[3] * material[3] * opacity.clamp(0.0, 1.0),
    ];
    (color, sampling)
}

/// A colour as four components from 0 to 1, in WPF's space (the gamma-encoded components, not
/// linearized; `docs/porting/media3d.md`, section 6.2).
pub fn color_uniform(color: Color) -> [f32; 4] {
    const SCALE: f32 = 1.0 / 255.0;
    [color.r as f32 * SCALE, color.g as f32 * SCALE, color.b as f32 * SCALE, color.a as f32 * SCALE]
}

/// A row-vector matrix as uniform columns: its rows in order.
pub fn matrix_uniform(matrix: &Matrix4x4) -> [[f32; 4]; 4] {
    [
        [matrix.m11, matrix.m12, matrix.m13, matrix.m14],
        [matrix.m21, matrix.m22, matrix.m23, matrix.m24],
        [matrix.m31, matrix.m32, matrix.m33, matrix.m34],
        [matrix.m41, matrix.m42, matrix.m43, matrix.m44],
    ]
}

/// The matrix of the normals of `world`: the inverse transpose of its upper 3x3, for row
/// vectors (`n * N`). A matrix without an inverse (a scale of zero) keeps its own 3x3, which
/// draws the flattened model with some normals rather than none.
pub fn normal_matrix(world: &Matrix4x4) -> Matrix4x4 {
    let a: [Float3; 3] = [
        [world.m11, world.m12, world.m13],
        [world.m21, world.m22, world.m23],
        [world.m31, world.m32, world.m33],
    ];
    // The cofactors of `a`; the inverse is their transpose over the determinant, so the inverse
    // transpose is the cofactors over the determinant.
    let c = [
        [
            a[1][1] * a[2][2] - a[1][2] * a[2][1],
            a[1][2] * a[2][0] - a[1][0] * a[2][2],
            a[1][0] * a[2][1] - a[1][1] * a[2][0],
        ],
        [
            a[0][2] * a[2][1] - a[0][1] * a[2][2],
            a[0][0] * a[2][2] - a[0][2] * a[2][0],
            a[0][1] * a[2][0] - a[0][0] * a[2][1],
        ],
        [
            a[0][1] * a[1][2] - a[0][2] * a[1][1],
            a[0][2] * a[1][0] - a[0][0] * a[1][2],
            a[0][0] * a[1][1] - a[0][1] * a[1][0],
        ],
    ];
    let determinant = a[0][0] * c[0][0] + a[0][1] * c[0][1] + a[0][2] * c[0][2];
    let n = if determinant.abs() > f32::MIN_POSITIVE && determinant.is_finite() {
        let inverse = 1.0 / determinant;
        [
            [c[0][0] * inverse, c[0][1] * inverse, c[0][2] * inverse],
            [c[1][0] * inverse, c[1][1] * inverse, c[1][2] * inverse],
            [c[2][0] * inverse, c[2][1] * inverse, c[2][2] * inverse],
        ]
    } else {
        a
    };
    Matrix4x4::new(
        n[0][0], n[0][1], n[0][2], 0.0, n[1][0], n[1][1], n[1][2], 0.0, n[2][0], n[2][1], n[2][2], 0.0, 0.0, 0.0, 0.0,
        1.0,
    )
}

/// The bytes of a uniform structure in the layout the GPU reads (std140, the byte order of the
/// machine), appended to a buffer.
pub trait Std140 {
    /// Appends the bytes of `self`: exactly `size_of::<Self>()` of them.
    fn write_std140(&self, out: &mut Vec<u8>);
}

fn write_floats(out: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        out.extend_from_slice(&value.to_ne_bytes());
    }
}

fn write_matrix(out: &mut Vec<u8>, matrix: &[[f32; 4]; 4]) {
    for column in matrix {
        write_floats(out, column);
    }
}

impl Std140 for LightUniforms {
    fn write_std140(&self, out: &mut Vec<u8>) {
        write_floats(out, &self.metadata);
        write_floats(out, &self.color);
        write_floats(out, &self.position_range);
        write_floats(out, &self.direction_inner_cos);
        write_floats(out, &self.attenuation_outer_cos);
    }
}

impl Std140 for FrameUniforms {
    fn write_std140(&self, out: &mut Vec<u8>) {
        write_matrix(out, &self.view_projection);
        write_floats(out, &self.camera_position);
        for value in self.light_count {
            out.extend_from_slice(&value.to_ne_bytes());
        }
        for light in &self.lights {
            light.write_std140(out);
        }
    }
}

impl Std140 for DrawUniforms {
    fn write_std140(&self, out: &mut Vec<u8>) {
        write_matrix(out, &self.world);
        write_matrix(out, &self.normal_matrix);
        write_floats(out, &self.color);
        write_floats(out, &self.ambient_color);
        write_floats(out, &self.material);
    }
}

/// The structures of this module in WGSL, member for member, for the shader of the GPU executors
/// (`scene3d.wgsl`, step D4): `uniforms_tests.rs` checks that the members are these, in this
/// order.
pub const WGSL_DECLARATIONS: &str = "\
const MAX_LIGHTS: u32 = 16u;

struct LightUniforms {
    metadata: vec4<f32>,
    color: vec4<f32>,
    position_range: vec4<f32>,
    direction_inner_cos: vec4<f32>,
    attenuation_outer_cos: vec4<f32>,
};

struct FrameUniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_count: vec4<u32>,
    lights: array<LightUniforms, 16>,
};

struct DrawUniforms {
    world: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
    color: vec4<f32>,
    ambient_color: vec4<f32>,
    material: vec4<f32>,
};
";

/// A point through the matrix of `columns` (see [`matrix_uniform`]).
pub(crate) fn transform_point(columns: &[[f32; 4]; 4], point: Float4) -> Float4 {
    crate::math::transform_columns(point, columns)
}

#[cfg(test)]
#[path = "uniforms_tests.rs"]
mod tests;
