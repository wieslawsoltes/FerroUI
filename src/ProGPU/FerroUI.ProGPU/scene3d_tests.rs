//! 3D scenes on the device, checked in the pixels that come back.

use super::*;
use ferroui_base::media::SharedBrush;
use ferroui_base::numerics::{Matrix4x4, Vector2, Vector3};
use ferroui_base::rendering::scene3d::{
    MeshData, Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DFrame, Scene3DLight, Scene3DMaterialKind,
    Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot,
};

fn layer(kind: Scene3DMaterialKind, color: Color) -> Scene3DMaterialLayer {
    let white = Color::from_argb(255, 255, 255, 255);
    Scene3DMaterialLayer {
        kind,
        color: white,
        ambient_color: white,
        specular_power: 0.0,
        brush: SharedBrush::from_brush(&ImmutableSolidColorBrush::new(color)),
    }
}

/// A quad from (-0.5, -0.5) to (0.5, 0.5) at `z`, its front towards the camera.
fn quad(id: u64, z: f32) -> Arc<MeshData> {
    let positions = vec![
        Vector3::new(-0.5, -0.5, z),
        Vector3::new(0.5, -0.5, z),
        Vector3::new(0.5, 0.5, z),
        Vector3::new(-0.5, 0.5, z),
    ];
    let bounds = Scene3DBounds::of_points(positions.iter().copied());
    Arc::new(MeshData {
        id,
        version: 1,
        positions,
        normals: vec![Vector3::new(0.0, 0.0, 1.0); 4],
        normals_generated: false,
        texture_coordinates: vec![
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        bounds,
    })
}

/// An orthographic camera at (0, 0, 5) looking down -z: x and y from -1 to 1 cover the viewport.
fn camera() -> Scene3DCamera {
    let (near, far) = (1.0f32, 10.0f32);
    let position = Vector3::new(0.0, 0.0, 5.0);
    let view = Matrix4x4::create_look_at(position, Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    let depth = 1.0 / (near - far);
    let projection =
        Matrix4x4::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, depth, 0.0, 0.0, 0.0, near * depth, 1.0);
    Scene3DCamera {
        kind: Scene3DCameraKind::Orthographic,
        view,
        projection,
        position,
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

fn scene(models: Vec<Scene3DModel>, lights: Vec<Scene3DLight>) -> Arc<Scene3DSnapshot> {
    Arc::new(Scene3DSnapshot {
        structure_generation: 1,
        record_generation: 1,
        camera_generation: 1,
        viewport: Rect::new(0.0, 0.0, 32.0, 32.0),
        camera: Some(camera()),
        models,
        lights,
        bounds: None,
    })
}

fn model(mesh: Arc<MeshData>, front: Vec<Scene3DMaterialLayer>) -> Scene3DModel {
    Scene3DModel { world: Matrix4x4::IDENTITY, world_bounds: mesh.bounds, mesh, front, back: Vec::new() }
}

/// Draws `scene` over a white 32 by 32 frame.
fn drawn(scene: Arc<Scene3DSnapshot>) -> Option<Arc<MemorySurface>> {
    let context = context()?;
    let surface = MemorySurface::new(32, 32);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(255, 255, 255));
    let scene_frame = Scene3DFrame { scene, bounds: Rect::new(0.0, 0.0, 32.0, 32.0) };
    let drew = frame.as_drawing_context_impl_with_scene_3d().expect("3D scenes").draw_scene_3d(&scene_frame);
    assert!(drew, "the scene is drawn on the device");
    frame.dispose();
    target.dispose();
    Some(surface)
}

#[test]
fn a_lit_diffuse_quad_is_drawn_on_the_device() {
    let white = Color::from_argb(255, 255, 255, 255);
    let quad = model(quad(1001, 0.0), vec![layer(Scene3DMaterialKind::Diffuse, Color::from_rgb(255, 0, 0))]);
    let Some(surface) = drawn(scene(vec![quad], vec![Scene3DLight::Ambient { color: white }])) else { return };
    assert_eq!(RED, surface.rgba(16, 16), "the quad");
    assert_eq!(WHITE, surface.rgba(2, 2), "the frame around it");
}

#[test]
fn the_nearer_quad_hides_the_farther_one_whatever_the_order() {
    let white = Color::from_argb(255, 255, 255, 255);
    let near = model(quad(1002, 1.0), vec![layer(Scene3DMaterialKind::Diffuse, Color::from_rgb(0, 0, 255))]);
    let far = model(quad(1003, 0.0), vec![layer(Scene3DMaterialKind::Diffuse, Color::from_rgb(255, 0, 0))]);
    let Some(surface) = drawn(scene(vec![near, far], vec![Scene3DLight::Ambient { color: white }])) else { return };
    assert_eq!(BLUE, surface.rgba(16, 16));
}

#[test]
fn an_emissive_layer_goes_through_the_resolve() {
    let quad = model(quad(1004, 0.0), vec![layer(Scene3DMaterialKind::Emissive, Color::from_rgb(255, 0, 0))]);
    let Some(surface) = drawn(scene(vec![quad], Vec::new())) else { return };
    let [r, _, _, a] = surface.rgba(16, 16);
    assert_eq!((255, 255), (r, a), "the alpha is raised to the largest channel");
}
