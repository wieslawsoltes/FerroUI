//! A video on a turning cube, over the same video shown flat (`--cube`, feature `video-cube`,
//! #608 D7): the material is WPF's form, a `VisualBrush` over the `MediaPlayerElement`, which
//! samples the frames of the element on the GPU. Each frame is acquired once for both.

use ferroui_base::{
    layout::{HorizontalAlignment, VerticalAlignment},
    media::{Color, IBrush, Points, Stretch, VisualBrush},
    threading::{DispatcherPriority, DispatcherTimer},
    Point, Ref, Thickness,
};
use ferroui_media3d::{
    AmbientLight, AxisAngleRotation3D, DiffuseMaterial, DirectionalLight, GeometryModel3D, Int32Collection,
    MeshGeometry3D, Model3DGroup, ModelVisual3D, PerspectiveCamera, Point3D, Point3DCollection,
    RotateTransform3D, Vector3D, Viewport3D,
};
use ferroui_media_video_controls::MediaPlayerElement;
use std::{cell::Cell, rc::Rc, time::Duration};

/// The four corners of each face of a cube from -1 to 1, counterclockwise seen from outside.
const FACES: &str = "-1,-1,1 1,-1,1 1,1,1 -1,1,1 \
                     1,-1,1 1,-1,-1 1,1,-1 1,1,1 \
                     1,-1,-1 -1,-1,-1 -1,1,-1 1,1,-1 \
                     -1,-1,-1 -1,-1,1 -1,1,1 -1,1,-1 \
                     -1,1,1 1,1,1 1,1,-1 -1,1,-1 \
                     -1,-1,-1 1,-1,-1 1,-1,1 -1,-1,1";

/// A cube whose six faces each show the whole brush.
fn cube() -> Ref<MeshGeometry3D> {
    let mesh = MeshGeometry3D::new();
    mesh.set_positions(Point3DCollection::parse(FACES).expect("the corners of the cube"));
    let indices: Vec<String> = (0..6)
        .flat_map(|face| [0, 1, 2, 0, 2, 3].map(|corner| (face * 4 + corner).to_string()))
        .collect();
    mesh.set_triangle_indices(Int32Collection::parse(&indices.join(" ")).expect("the triangles of the cube"));
    let corners = [Point::new(0.0, 1.0), Point::new(1.0, 1.0), Point::new(1.0, 0.0), Point::new(0.0, 0.0)];
    mesh.set_texture_coordinates(Points::from_items((0..6).flat_map(|_| corners)));
    mesh
}

/// The viewport with the cube, in the bottom right corner of what holds it.
pub fn create(view: &Ref<MediaPlayerElement>) -> Ref<Viewport3D> {
    let brush = VisualBrush::new();
    brush.set_visual(view);
    brush.set_stretch(Stretch::UniformToFill);
    let brush: Rc<dyn IBrush> = brush.into();
    let material = DiffuseMaterial::with_brush(Some(brush));
    let model = GeometryModel3D::with_geometry_and_material(cube().upcast(), material.upcast());

    let rotation = AxisAngleRotation3D::with_axis_angle(Vector3D::new(0.3, 1.0, 0.2), 0.0);
    model.set_transform(Some(RotateTransform3D::with_rotation(rotation.clone().upcast()).upcast()));

    let scene = Model3DGroup::new();
    scene.children().add(model.upcast());
    scene.children().add(AmbientLight::with_color(Color::from_argb(255, 90, 90, 90)).upcast());
    let light = DirectionalLight::with_color_and_direction(
        Color::from_argb(255, 200, 200, 200),
        Vector3D::new(-0.4, -0.6, -1.0),
    );
    scene.children().add(light.upcast());

    let viewport = Viewport3D::new();
    viewport.set_camera(Some(
        PerspectiveCamera::with_values(
            Point3D::new(0.0, 0.0, 5.0),
            Vector3D::new(0.0, 0.0, -1.0),
            Vector3D::new(0.0, 1.0, 0.0),
            45.0,
        )
        .upcast(),
    ));
    viewport.children().add(ModelVisual3D::with_content(scene.upcast()).upcast());
    viewport.set_width(280.0);
    viewport.set_height(280.0);
    viewport.set_margin(Thickness::uniform(12.0));
    viewport.set_horizontal_alignment(HorizontalAlignment::Right);
    viewport.set_vertical_alignment(VerticalAlignment::Bottom);

    let weak = viewport.downgrade();
    let angle = Cell::new(0.0);
    DispatcherTimer::run(
        move || {
            if weak.upgrade().is_none() {
                return false;
            }
            angle.set((angle.get() + 0.6) % 360.0);
            rotation.set_angle(angle.get());
            true
        },
        Duration::from_millis(16),
        DispatcherPriority::RENDER,
    );
    viewport
}
