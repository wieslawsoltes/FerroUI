use super::*;
use crate::math::transform4;
use crate::test_scenes::{diffuse, layer, model, orthographic, quad, snapshot, white_ambient};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::SharedBrush;
use ferroui_base::numerics::Vector3;
use std::mem::{offset_of, size_of};

/// The members of each structure with their std140 offsets: what the WGSL declares.
const LIGHT_MEMBERS: [(&str, usize); 5] = [
    ("metadata", 0),
    ("color", 16),
    ("position_range", 32),
    ("direction_inner_cos", 48),
    ("attenuation_outer_cos", 64),
];
const FRAME_MEMBERS: [(&str, usize); 4] =
    [("view_projection", 0), ("camera_position", 64), ("light_count", 80), ("lights", 96)];
const DRAW_MEMBERS: [(&str, usize); 5] =
    [("world", 0), ("normal_matrix", 64), ("color", 128), ("ambient_color", 144), ("material", 160)];

#[test]
fn the_layouts_are_std140() {
    assert_eq!(80, size_of::<LightUniforms>());
    assert_eq!(
        [0, 16, 32, 48, 64],
        [
            offset_of!(LightUniforms, metadata),
            offset_of!(LightUniforms, color),
            offset_of!(LightUniforms, position_range),
            offset_of!(LightUniforms, direction_inner_cos),
            offset_of!(LightUniforms, attenuation_outer_cos),
        ]
    );

    // An array of structures in std140 has a stride rounded to 16 bytes: 80 is.
    assert_eq!(0, size_of::<LightUniforms>() % 16);
    assert_eq!(96 + 16 * 80, size_of::<FrameUniforms>());
    assert_eq!(
        [0, 64, 80, 96],
        [
            offset_of!(FrameUniforms, view_projection),
            offset_of!(FrameUniforms, camera_position),
            offset_of!(FrameUniforms, light_count),
            offset_of!(FrameUniforms, lights),
        ]
    );
    // Uniform blocks of WebGL2 and GLES 3.0 are at least 16 KiB.
    assert!(size_of::<FrameUniforms>() <= 16 * 1024);

    assert_eq!(176, size_of::<DrawUniforms>());
    assert_eq!(
        [0, 64, 128, 144, 160],
        [
            offset_of!(DrawUniforms, world),
            offset_of!(DrawUniforms, normal_matrix),
            offset_of!(DrawUniforms, color),
            offset_of!(DrawUniforms, ambient_color),
            offset_of!(DrawUniforms, material),
        ]
    );
    assert!(size_of::<DrawUniforms>() <= DRAW_UNIFORMS_STRIDE);
    assert_eq!(0, DRAW_UNIFORMS_STRIDE % 256);

    for (members, offsets) in [
        (LIGHT_MEMBERS.as_slice(), [0, 16, 32, 48, 64].as_slice()),
        (FRAME_MEMBERS.as_slice(), [0, 64, 80, 96].as_slice()),
        (DRAW_MEMBERS.as_slice(), [0, 64, 128, 144, 160].as_slice()),
    ] {
        let listed: Vec<usize> = members.iter().map(|(_, offset)| *offset).collect();
        assert_eq!(offsets, listed.as_slice());
    }
}

#[test]
fn the_bytes_written_are_the_layout() {
    let scene = snapshot(
        orthographic(),
        vec![model(quad(1, -1.0, -1.0, 1.0, 1.0, 0.0), vec![diffuse(Color::from_argb(255, 255, 0, 0))], vec![])],
        vec![white_ambient()],
    );
    let frame = FrameUniforms::of(&scene).expect("a camera");
    let mut bytes = Vec::new();
    frame.write_std140(&mut bytes);
    assert_eq!(size_of::<FrameUniforms>(), bytes.len());
    let read = |offset: usize| f32::from_ne_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"));
    // The colour of the first light, at 96 + 16.
    assert_eq!(1.0, read(96 + 16));
    // The light count, a u32 at 80.
    assert_eq!(1, u32::from_ne_bytes(bytes[80..84].try_into().expect("four bytes")));

    let (draw, _) = DrawUniforms::of(&scene.models[0], &scene.models[0].front[0], Face::Front);
    let mut bytes = Vec::new();
    draw.write_std140(&mut bytes);
    assert_eq!(size_of::<DrawUniforms>(), bytes.len());
    // The red of the colour at 128.
    assert_eq!(1.0f32.to_ne_bytes(), bytes[128..132]);
}

#[test]
fn the_wgsl_declares_the_same_members_in_the_same_order() {
    for (structure, members) in [
        ("LightUniforms", LIGHT_MEMBERS.as_slice()),
        ("FrameUniforms", FRAME_MEMBERS.as_slice()),
        ("DrawUniforms", DRAW_MEMBERS.as_slice()),
    ] {
        let start = WGSL_DECLARATIONS.find(&format!("struct {structure} {{")).expect("the structure is declared");
        let body = &WGSL_DECLARATIONS[start..start + WGSL_DECLARATIONS[start..].find("};").expect("its end")];
        let declared: Vec<&str> = body
            .lines()
            .skip(1)
            .filter_map(|line| line.trim().split(':').next())
            .filter(|name| !name.is_empty())
            .collect();
        let expected: Vec<&str> = members.iter().map(|(name, _)| *name).collect();
        assert_eq!(expected, declared, "{structure}");
    }
    assert!(WGSL_DECLARATIONS.contains(&format!("array<LightUniforms, {MAX_LIGHTS}>")));
}

#[test]
fn a_matrix_as_uniform_columns_transforms_as_the_row_vector_matrix() {
    let matrix = Matrix4x4::create_rotation_y(0.3) * Matrix4x4::create_translation_xyz(1.0, -2.0, 3.0);
    let point = [0.5, 0.25, -1.5, 1.0];
    assert_eq!(transform4(point, &matrix), transform_point(&matrix_uniform(&matrix), point));
}

#[test]
fn the_normal_matrix_is_the_inverse_transpose() {
    // A scale of 2 along x: a normal of the plane x = y becomes (0.5, 1, 0) up to its length.
    let world = Matrix4x4::create_scale_xyz(2.0, 1.0, 1.0);
    let normal = transform4([1.0, 1.0, 0.0, 0.0], &normal_matrix(&world));
    assert!((normal[0] / normal[1] - 0.5).abs() < 1e-6, "{normal:?}");
    // A rotation is its own inverse transpose.
    let rotation = Matrix4x4::create_rotation_z(0.7);
    let rotated = transform4([1.0, 0.0, 0.0, 0.0], &normal_matrix(&rotation));
    let expected = transform4([1.0, 0.0, 0.0, 0.0], &rotation);
    for i in 0..3 {
        assert!((rotated[i] - expected[i]).abs() < 1e-6);
    }
    // No inverse: the 3x3 itself.
    let flat = Matrix4x4::create_scale_xyz(1.0, 0.0, 1.0);
    assert_eq!(0.0, normal_matrix(&flat).m22);
}

#[test]
fn a_layer_is_its_brush_times_its_colour_with_the_opacity_of_the_brush() {
    let mut layered = diffuse(Color::from_argb(255, 255, 128, 0));
    layered.color = Color::from_argb(255, 255, 255, 0);
    let (color, sampling) = layer_color(&layered);
    assert_eq!(BrushSampling::Solid, sampling);
    assert_eq!([1.0, 128.0 / 255.0, 0.0, 1.0], color);

    let translucent = ImmutableSolidColorBrush::with_opacity(Color::from_argb(255, 0, 0, 255), 0.5);
    layered.brush = SharedBrush::from_brush(&translucent);
    assert_eq!(0.5, layer_color(&layered).0[3]);

    layered.brush = None;
    let (color, sampling) = layer_color(&layered);
    assert_eq!(BrushSampling::Unsampled, sampling);
    assert_eq!([1.0, 1.0, 0.0, 1.0], color);
}

#[test]
fn the_frame_takes_sixteen_lights_and_the_camera_toward_the_viewer() {
    let lights = (0..20).map(|_| white_ambient()).collect();
    let before = crate::lights_left_out();
    let scene = snapshot(orthographic(), vec![], lights);
    let frame = FrameUniforms::of(&scene).expect("a camera");
    assert_eq!(16, frame.active_lights().len());
    assert!(crate::lights_left_out() >= before + 4);
    // An orthographic camera: the direction toward it.
    assert_eq!([0.0, 0.0, 1.0, 0.0], frame.camera_position);

    let mut without_camera = (*scene).clone();
    without_camera.camera = None;
    assert!(FrameUniforms::of(&without_camera).is_none());
}

#[test]
fn a_spot_light_holds_the_cosines_of_its_half_angles() {
    let light = LightUniforms::of(&Scene3DLight::Spot {
        color: Color::from_argb(255, 255, 255, 255),
        position: Vector3::new(1.0, 2.0, 3.0),
        direction: Vector3::new(0.0, -1.0, 0.0),
        range: f32::INFINITY,
        attenuation: [1.0, 0.5, 0.25],
        inner_cone_angle: 180.0,
        outer_cone_angle: 90.0,
    });
    assert_eq!(LightKind::Spot, light.kind());
    assert!((light.direction_inner_cos[3] - 0.0).abs() < 1e-6);
    assert!((light.attenuation_outer_cos[3] - 45f32.to_radians().cos()).abs() < 1e-6);
    assert_eq!(f32::MAX, light.position_range[3]);
    assert_eq!([1.0, 0.5, 0.25], [light.attenuation_outer_cos[0], light.attenuation_outer_cos[1], light.attenuation_outer_cos[2]]);
    assert_eq!(LayerKind::Specular, LayerKind::from_uniform_value(LayerKind::of(layer(Scene3DMaterialKind::Specular, Color::default()).kind).uniform_value()));
}
