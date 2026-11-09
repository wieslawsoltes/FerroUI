//! A scene loaded from XAML through the run-time loader: the classes of the crate in the XML
//! namespace of the framework, their properties from their text forms, and the content
//! properties of the groups and the visuals.

use crate::*;
use ferroui_base::media::Color;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{FerroLocator, Rect, Ref, Size};
use ferroui_controls::Control;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

/// The scope of a test that loads markup: the types of the crate registered, the run-time loader
/// installed, and a dispatcher and a service locator of the test's own.
struct XamlTestScope {
    locator: Option<std::rc::Rc<dyn IDisposable>>,
    _dispatcher: UnitTestDispatcherScope,
}

fn xaml_test_base() -> XamlTestScope {
    register_types();
    let dispatcher = Dispatcher::unit_test_scope();
    Control::reset_loaded_queue_for_unit_tests();
    let locator = FerroLocator::enter_scope();
    FerroRuntimeXamlLoader::register();
    XamlTestScope { locator: Some(locator), _dispatcher: dispatcher }
}

impl Drop for XamlTestScope {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            Dispatcher::reset_for_unit_tests();
        }
        if let Some(locator) = self.locator.take() {
            locator.dispose();
        }
        Control::reset_loaded_queue_for_unit_tests();
    }
}

#[track_caller]
fn load_as<T: Clone + 'static>(xaml: &str) -> T {
    FerroRuntimeXamlLoader::parse_as::<T>(xaml, None).unwrap_or_else(|error| panic!("the document loads: {error:?}"))
}

const NS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

/// A lit, rotated red cube seen by a perspective camera, written as WPF writes it.
fn cube_scene() -> String {
    format!(
        "<Viewport3D {NS} Width='200' Height='100'>
           <Viewport3D.Camera>
             <PerspectiveCamera Position='0,0,5' LookDirection='0,0,-1' UpDirection='0,1,0' FieldOfView='60'/>
           </Viewport3D.Camera>
           <ModelVisual3D>
             <ModelVisual3D.Content>
               <Model3DGroup>
                 <AmbientLight Color='#404040'/>
                 <DirectionalLight Color='White' Direction='-1,-1,-1'/>
                 <GeometryModel3D>
                   <GeometryModel3D.Geometry>
                     <MeshGeometry3D
                       Positions='-1,-1,1 1,-1,1 1,1,1 -1,1,1 -1,-1,-1 1,-1,-1 1,1,-1 -1,1,-1'
                       TriangleIndices='0 1 2 0 2 3 1 5 6 1 6 2 5 4 7 5 7 6 4 0 3 4 3 7 3 2 6 3 6 7 4 5 1 4 1 0'
                       TextureCoordinates='0,1 1,1 1,0 0,0 0,1 1,1 1,0 0,0'/>
                   </GeometryModel3D.Geometry>
                   <GeometryModel3D.Material>
                     <MaterialGroup>
                       <DiffuseMaterial Brush='Red'/>
                       <SpecularMaterial Brush='White' SpecularPower='20'/>
                     </MaterialGroup>
                   </GeometryModel3D.Material>
                   <GeometryModel3D.Transform>
                     <Transform3DGroup>
                       <RotateTransform3D>
                         <RotateTransform3D.Rotation>
                           <AxisAngleRotation3D Axis='0,1,0' Angle='30'/>
                         </RotateTransform3D.Rotation>
                       </RotateTransform3D>
                       <TranslateTransform3D OffsetX='0.5'/>
                     </Transform3DGroup>
                   </GeometryModel3D.Transform>
                 </GeometryModel3D>
               </Model3DGroup>
             </ModelVisual3D.Content>
           </ModelVisual3D>
         </Viewport3D>"
    )
}

#[test]
fn a_cube_scene_loads_from_xaml() {
    let _scope = xaml_test_base();
    let viewport = load_as::<Ref<Viewport3D>>(&cube_scene());

    let camera = viewport.camera().and_then(|camera| camera.cast::<PerspectiveCamera>()).expect("a perspective camera");
    assert_eq!(camera.position(), Point3D::new(0.0, 0.0, 5.0));
    assert_eq!(camera.look_direction(), Vector3D::new(0.0, 0.0, -1.0));
    assert_eq!(camera.field_of_view(), 60.0);

    assert_eq!(viewport.children().len(), 1);
    let visual = viewport.children().get(0).cast::<ModelVisual3D>().expect("a model visual");
    let group = visual.content().and_then(|content| content.cast::<Model3DGroup>()).expect("a group");
    assert_eq!(group.children().len(), 3);
    let ambient = group.children().get(0).cast::<AmbientLight>().expect("an ambient light");
    assert_eq!(ambient.color(), Color::from_rgb(0x40, 0x40, 0x40));
    let directional = group.children().get(1).cast::<DirectionalLight>().expect("a directional light");
    assert_eq!(directional.direction(), Vector3D::new(-1.0, -1.0, -1.0));

    let model = group.children().get(2).cast::<GeometryModel3D>().expect("a geometry model");
    let mesh = model.geometry().and_then(|geometry| geometry.cast::<MeshGeometry3D>()).expect("a mesh");
    assert_eq!(mesh.positions().len(), 8);
    assert_eq!(mesh.triangle_indices().len(), 36);
    assert_eq!(mesh.texture_coordinates().len(), 8);
    assert_eq!(mesh.bounds(), Rect3D::new(-1.0, -1.0, -1.0, 2.0, 2.0, 2.0).unwrap());
    let material = model.material().and_then(|material| material.cast::<MaterialGroup>()).expect("a material group");
    assert_eq!(material.children().len(), 2);
    let transform = model.transform().and_then(|transform| transform.cast::<Transform3DGroup>()).expect("a group");
    assert_eq!(transform.children().len(), 2);

    // The scene compiles: one model of two layers, two lights, the camera of the viewport.
    viewport.measure(Size::new(f64::INFINITY, f64::INFINITY));
    viewport.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));
    let snapshot = viewport.snapshot();
    assert!(snapshot.draws_anything());
    assert_eq!(snapshot.models.len(), 1);
    assert_eq!(snapshot.models[0].front.len(), 2);
    assert_eq!(snapshot.models[0].mesh.triangle_count(), 12);
    assert_eq!(snapshot.models[0].mesh.texture_coordinates.len(), 8);
    assert_eq!(snapshot.lights.len(), 2);
    assert_eq!(snapshot.camera.unwrap().kind, Scene3DCameraKind::Perspective);
    // The model is rotated and moved: the translation of its world matrix is (0.5, 0, 0).
    assert_eq!(snapshot.models[0].world.m41, 0.5);
}

#[test]
fn the_other_cameras_and_lights_load_from_xaml() {
    let _scope = xaml_test_base();
    let viewport = load_as::<Ref<Viewport3D>>(&format!(
        "<Viewport3D {NS}>
           <Viewport3D.Camera>
             <OrthographicCamera Position='0,0,10' Width='4' NearPlaneDistance='1' FarPlaneDistance='20'/>
           </Viewport3D.Camera>
           <ModelVisual3D>
             <ModelVisual3D.Transform>
               <ScaleTransform3D ScaleX='2' ScaleY='2' ScaleZ='2'/>
             </ModelVisual3D.Transform>
             <ModelVisual3D.Content>
               <Model3DGroup>
                 <PointLight Color='Yellow' Position='0,2,0' Range='10' QuadraticAttenuation='0.5'/>
                 <SpotLight Position='0,0,3' Direction='0,0,-1' InnerConeAngle='20' OuterConeAngle='40'/>
               </Model3DGroup>
             </ModelVisual3D.Content>
             <ModelVisual3D>
               <ModelVisual3D.Content>
                 <GeometryModel3D>
                   <GeometryModel3D.Geometry>
                     <MeshGeometry3D Positions='0,0,0 1,0,0 0,1,0' Normals='0,0,1 0,0,1 0,0,1'/>
                   </GeometryModel3D.Geometry>
                   <GeometryModel3D.BackMaterial>
                     <EmissiveMaterial Brush='Blue'/>
                   </GeometryModel3D.BackMaterial>
                 </GeometryModel3D>
               </ModelVisual3D.Content>
             </ModelVisual3D>
           </ModelVisual3D>
         </Viewport3D>"
    ));
    let camera = viewport.camera().and_then(|camera| camera.cast::<OrthographicCamera>()).expect("an orthographic camera");
    assert_eq!((camera.width(), camera.near_plane_distance(), camera.far_plane_distance()), (4.0, 1.0, 20.0));
    let snapshot = viewport.snapshot();
    assert_eq!(snapshot.lights.len(), 2);
    let Scene3DLight::Point { range, attenuation, .. } = snapshot.lights[0] else { panic!("a point light") };
    assert_eq!((range, attenuation[2]), (10.0, 0.5));
    assert!(matches!(snapshot.lights[1], Scene3DLight::Spot { .. }));
    assert_eq!(snapshot.models.len(), 1, "the nested visual is under the scale of its parent");
    assert_eq!(snapshot.models[0].world.m11, 2.0);
    assert!(snapshot.models[0].front.is_empty());
    assert_eq!(snapshot.models[0].back[0].kind, Scene3DMaterialKind::Emissive);
    assert!(!snapshot.models[0].mesh.normals_generated);
    let camera = snapshot.camera.unwrap();
    assert_eq!(camera.kind, Scene3DCameraKind::Orthographic);
    assert_eq!((camera.near_plane_distance, camera.far_plane_distance), (1.0, 20.0));
}
