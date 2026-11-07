use super::*;
use crate::test_scenes::{diffuse, layer, model, orthographic, quad, snapshot};
use crate::uniforms::{DrawUniforms, Face, FrameUniforms, LightUniforms};
use ferroui_base::media::Color;
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::scene3d::{Scene3DLight, Scene3DMaterialKind};

const WHITE: Color = Color::from_argb(255, 255, 255, 255);

fn gray(level: u8) -> Color {
    Color::from_argb(255, level, level, level)
}

fn close(expected: f32, actual: f32) {
    assert!((expected - actual).abs() < 1e-5, "expected {expected}, got {actual}");
}

fn ambient(level: u8) -> LightUniforms {
    LightUniforms::of(&Scene3DLight::Ambient { color: gray(level) })
}

fn directional(level: u8, direction: [f32; 3]) -> LightUniforms {
    LightUniforms::of(&Scene3DLight::Directional {
        color: gray(level),
        direction: Vector3::new(direction[0], direction[1], direction[2]),
    })
}

fn point(position: [f32; 3], range: f32, attenuation: [f32; 3]) -> LightUniforms {
    LightUniforms::of(&Scene3DLight::Point {
        color: WHITE,
        position: Vector3::new(position[0], position[1], position[2]),
        range,
        attenuation,
    })
}

fn spot(inner: f32, outer: f32) -> LightUniforms {
    LightUniforms::of(&Scene3DLight::Spot {
        color: WHITE,
        position: Vector3::new(0.0, 0.0, 1.0),
        direction: Vector3::new(0.0, 0.0, -1.0),
        range: f32::INFINITY,
        attenuation: [1.0, 0.0, 0.0],
        inner_cone_angle: inner,
        outer_cone_angle: outer,
    })
}

const UP: [f32; 3] = [0.0, 0.0, 1.0];

#[test]
fn ambient_lights_add_up_and_a_directional_light_is_lambert() {
    let lights = [ambient(51), ambient(51), directional(255, [0.0, 0.0, -1.0])];
    let head_on = compute_wpf_lighting(&lights, UP, UP, [0.0; 3], 40.0, false);
    close(102.0 / 255.0, head_on.ambient[0]);
    close(1.0, head_on.diffuse[0]);
    assert_eq!([0.0; 3], head_on.specular, "not asked for");

    // The surface turned 60 degrees from the light: cos 60 = 0.5.
    let turned = [60f32.to_radians().sin(), 0.0, 60f32.to_radians().cos()];
    let terms = compute_wpf_lighting(&lights, turned, UP, [0.0; 3], 40.0, false);
    close(0.5, terms.diffuse[1]);

    // A light behind the surface lights only its ambient part.
    let behind = compute_wpf_lighting(&lights, [0.0, 0.0, -1.0], UP, [0.0; 3], 40.0, false);
    close(0.0, behind.diffuse[2]);
    close(102.0 / 255.0, behind.ambient[2]);
}

#[test]
fn a_point_light_is_attenuated_and_ends_at_its_range() {
    // At a distance of 2 straight above: 1 / (1 + 0.5 * 2 + 0.25 * 4) = 1 / 3.
    let lights = [point([0.0, 0.0, 2.0], 10.0, [1.0, 0.5, 0.25])];
    close(1.0 / 3.0, compute_wpf_lighting(&lights, UP, UP, [0.0; 3], 1.0, false).diffuse[0]);

    // Off to the side at (1.5, 0, 0): d = 2.5, N . L = 2 / 2.5, attenuation 1 / (1 + 1.25 + 1.5625).
    let terms = compute_wpf_lighting(&lights, UP, UP, [1.5, 0.0, 0.0], 1.0, false);
    close(0.8 / 3.8125, terms.diffuse[0]);

    // Beyond the range: nothing.
    let short = [point([0.0, 0.0, 2.0], 1.5, [1.0, 0.5, 0.25])];
    close(0.0, compute_wpf_lighting(&short, UP, UP, [0.0; 3], 1.0, false).diffuse[0]);

    // An attenuation below 1 does not brighten the light (the denominator is at least 1).
    let strong = [point([0.0, 0.0, 2.0], f32::INFINITY, [0.2, 0.0, 0.0])];
    close(1.0, compute_wpf_lighting(&strong, UP, UP, [0.0; 3], 1.0, false).diffuse[0]);
}

#[test]
fn a_spot_light_falls_off_between_its_cones_given_as_full_angles() {
    // Full angles 60 and 90: full light within 30 degrees of the axis, none beyond 45.
    let lights = [spot(60.0, 90.0)];
    let uniforms = lights[0];
    close(30f32.to_radians().cos(), uniforms.direction_inner_cos[3]);
    close(45f32.to_radians().cos(), uniforms.attenuation_outer_cos[3]);

    let at_angle = |degrees: f32| {
        // The point on the plane z = 0 seen from the light at (0, 0, 1) at `degrees` off the axis,
        // lit along its normal toward the light so that only the cone and the Lambert term count.
        let x = degrees.to_radians().tan();
        let to_light = crate::math::normalize([-x, 0.0, 1.0]);
        compute_wpf_lighting(&lights, to_light, UP, [x, 0.0, 0.0], 1.0, false).diffuse[0]
    };
    close(1.0, at_angle(0.0));
    close(1.0, at_angle(25.0));
    let expected = (40f32.to_radians().cos() - 45f32.to_radians().cos())
        / (30f32.to_radians().cos() - 45f32.to_radians().cos());
    assert!((at_angle(40.0) - expected).abs() < 1e-4, "{} against {expected}", at_angle(40.0));
    close(0.0, at_angle(50.0));
}

#[test]
fn the_specular_term_is_blinn_phong() {
    let lights = [directional(255, [0.0, 0.0, -1.0])];
    // The viewer on the light's side, head on: N . H = 1.
    close(1.0, compute_wpf_lighting(&lights, UP, UP, [0.0; 3], 40.0, true).specular[0]);

    // The light 60 degrees off the normal, the viewer on the normal: H halfway, at 30 degrees.
    let light = [-(60f32.to_radians().sin()), 0.0, -(60f32.to_radians().cos())];
    let lights = [directional(255, light)];
    let terms = compute_wpf_lighting(&lights, UP, UP, [0.0; 3], 40.0, true);
    let expected = 30f32.to_radians().cos().powf(40.0);
    assert!((terms.specular[0] - expected).abs() < 1e-5, "{} against {expected}", terms.specular[0]);
    close(0.5, terms.diffuse[0]);
}

#[test]
fn a_diffuse_layer_is_its_colour_times_the_ambient_and_the_diffuse_light() {
    let mut red = diffuse(Color::from_argb(255, 255, 0, 0));
    red.ambient_color = gray(128);
    let scene = snapshot(
        orthographic(),
        vec![model(quad(1, -1.0, -1.0, 1.0, 1.0, 0.0), vec![red.clone()], vec![red])],
        vec![
            Scene3DLight::Ambient { color: WHITE },
            Scene3DLight::Directional { color: gray(64), direction: Vector3::new(0.0, 0.0, -1.0) },
        ],
    );
    let frame = FrameUniforms::of(&scene).expect("a camera");
    let model = &scene.models[0];
    let (front, _) = DrawUniforms::of(model, &model.front[0], Face::Front);
    let color = shade(&frame, &front, [0.0, 0.0, 2.0], [0.0; 3]);
    close(128.0 / 255.0 + 64.0 / 255.0, color[0]);
    close(0.0, color[1]);
    close(1.0, color[3]);

    // The back face turns the normal around: away from the light, only the ambient part.
    let (back, _) = DrawUniforms::of(model, &model.back[0], Face::Back);
    let color = shade(&frame, &back, [0.0, 0.0, 1.0], [0.0; 3]);
    close(128.0 / 255.0, color[0]);
}

#[test]
fn specular_and_emissive_layers_are_added_with_no_alpha() {
    let mut half = layer(Scene3DMaterialKind::Emissive, Color::from_argb(128, 0, 255, 0));
    half.color = WHITE;
    let specular = layer(Scene3DMaterialKind::Specular, WHITE);
    let scene = snapshot(
        orthographic(),
        vec![model(quad(1, -1.0, -1.0, 1.0, 1.0, 0.0), vec![half, specular], vec![])],
        vec![Scene3DLight::Directional { color: WHITE, direction: Vector3::new(0.0, 0.0, -1.0) }],
    );
    let frame = FrameUniforms::of(&scene).expect("a camera");
    let model = &scene.models[0];
    let (emissive, _) = DrawUniforms::of(model, &model.front[0], Face::Front);
    let color = shade(&frame, &emissive, UP, [0.0; 3]);
    close(0.0, color[0]);
    close(128.0 / 255.0, color[1]);
    close(0.0, color[3]);

    // The orthographic camera looks along -z from everywhere: head on, N . H = 1.
    let (highlight, _) = DrawUniforms::of(model, &model.front[1], Face::Front);
    let color = shade(&frame, &highlight, UP, [0.7, -0.3, 0.0]);
    close(1.0, color[0]);
    close(0.0, color[3]);
}
