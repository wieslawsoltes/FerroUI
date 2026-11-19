//! Tests of the pages of the section "3D": the pages of the 3D API of the framework
//! (`ferroui-media3d`, docs/porting/media3d.md).
//!
//! Not ports: the pages are not from upstream. Each page is created and shown in a window; the
//! tests check that its scene is the one of its document and that its controls change the scene.

use super::support::*;
use crate::pages::{
    Interaction3DPage, Transforms3DPage, Viewport3DPage, CUBE_COLOURS, CUBE_NAMES, MATERIAL_COLOURS, SELECTED_COLOUR,
    SPIN_AXES,
};
use ferroui_base::animation::SignedDuration;
use ferroui_base::media::{Color, Colors, IBrush, ISolidColorBrush};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Ref};
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{CheckBox, ComboBox, Control, Slider, TextBlock, Window};
use ferroui_media3d::{Point3D, Scene3DCameraKind};
use std::rc::Rc;

/// Shows `page` in a window of 1100 by 800 and lays it out.
fn show(page: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(page)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
    window
}

/// The colour of a solid colour brush.
fn colour(brush: Option<Rc<dyn IBrush>>) -> Option<Color> {
    brush.and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
}

#[test]
fn the_viewport_3d_page_shows_a_lit_cube_and_its_controls_change_the_scene() {
    let _app = start_catalog_application();
    let page = Viewport3DPage::new();
    for name in ["Yaw", "Pitch", "Roll", "Distance", "FieldOfView"] {
        assert!(page.find_control::<Slider>(name).is_some(), "the slider {name}");
    }
    for name in ["Specular", "Ambient", "Directional"] {
        assert!(page.find_control::<CheckBox>(name).is_some(), "the check box {name}");
    }
    assert!(page.find_control::<ComboBox>("MaterialColour").is_some());
    let window = show(page.clone().upcast());
    let scene = page.scene();
    assert_eq!(scene.viewport.bounds().width, 560.0, "the viewport is arranged");

    // The scene of the document: two models (the floor and the cube), two lights, the camera.
    let snapshot = scene.viewport.snapshot();
    assert!(snapshot.draws_anything());
    assert_eq!(snapshot.models.len(), 2);
    assert_eq!(snapshot.lights.len(), 2);
    assert_eq!(snapshot.camera.expect("a camera").kind, Scene3DCameraKind::Perspective);
    assert_eq!(snapshot.models[1].mesh.triangle_count(), 12, "the cube");
    assert_eq!(snapshot.models[1].front.len(), 2, "the diffuse and the specular layer of the cube");
    assert_eq!((scene.yaw.angle(), scene.pitch.angle(), scene.roll.angle()), (30.0, -20.0, 0.0));
    assert_eq!(colour(scene.diffuse.brush()), Some(MATERIAL_COLOURS[0]));

    // The controls change the objects of the scene.
    page.get_control::<Slider>("Yaw").set_range_value(-90.0);
    page.get_control::<Slider>("Pitch").set_range_value(45.0);
    page.get_control::<Slider>("Roll").set_range_value(10.0);
    assert_eq!((scene.yaw.angle(), scene.pitch.angle(), scene.roll.angle()), (-90.0, 45.0, 10.0));
    page.get_control::<Slider>("Distance").set_range_value(10.0);
    assert_eq!(scene.camera.position(), Point3D::new(0.0, 2.5, 10.0));
    page.get_control::<Slider>("FieldOfView").set_range_value(60.0);
    assert_eq!(scene.camera.field_of_view(), 60.0);
    page.get_control::<ComboBox>("MaterialColour").set_selected_index(3);
    assert_eq!(colour(scene.diffuse.brush()), Some(MATERIAL_COLOURS[3]));

    // The specular highlight off: its material has no brush and is no layer of the cube.
    page.get_control::<CheckBox>("Specular").set_is_checked(Some(false));
    assert!(scene.specular.brush().is_none());
    assert_eq!(scene.viewport.snapshot().models[1].front.len(), 1);
    page.get_control::<CheckBox>("Ambient").set_is_checked(Some(false));
    page.get_control::<CheckBox>("Directional").set_is_checked(Some(false));
    assert_eq!((scene.ambient.color(), scene.directional.color()), (Colors::BLACK, Colors::BLACK));
    page.get_control::<CheckBox>("Directional").set_is_checked(Some(true));
    assert_eq!(scene.directional.color(), Colors::WHITE);

    window.close();
}

#[test]
fn the_3d_interaction_page_reports_the_model_under_the_pointer_and_selects_cubes() {
    let _app = start_catalog_application();
    let page = Interaction3DPage::new();
    let window = show(page.clone().upcast());
    let viewport = page.viewport();
    assert_eq!((viewport.bounds().width, viewport.bounds().height), (560.0, 320.0), "the viewport is arranged");

    let cubes = page.cubes();
    assert_eq!(cubes.iter().map(|cube| cube.name).collect::<Vec<_>>(), CUBE_NAMES.to_vec());
    for (cube, expected) in cubes.iter().zip(CUBE_COLOURS) {
        assert_eq!(colour(cube.material.brush()), Some(expected), "{}", cube.name);
        assert!(cube.element.is_hit_test_visible());
    }
    // The three cubes share the mesh of the resources of the page.
    assert!(cubes[0].model.geometry().unwrap().ptr_eq(&cubes[2].model.geometry().unwrap()));
    assert_eq!(viewport.snapshot().models.len(), 3);

    // The centre of the viewport is on the middle cube, a corner on no model.
    assert_eq!(page.report_hit_at(Point::new(280.0, 160.0)), Some(1));
    let hit = page.get_control::<TextBlock>("HitText").text().unwrap_or_default();
    assert!(hit.starts_with("Green cube: point ("), "{hit}");
    assert!(hit.ends_with("front face"), "{hit}");
    assert_eq!(page.report_hit_at(Point::new(2.0, 2.0)), None);
    assert_eq!(page.get_control::<TextBlock>("HitText").text().as_deref(), Some("Nothing under the pointer"));

    // A press selects a cube and colours it; a second press on it clears the selection.
    page.select(2, None);
    assert_eq!(page.selected(), Some(2));
    assert_eq!(colour(cubes[2].material.brush()), Some(SELECTED_COLOUR));
    assert_eq!(page.get_control::<TextBlock>("SelectedText").text().as_deref(), Some("Selected: Blue cube"));
    page.select(2, None);
    assert_eq!(page.selected(), None);
    assert_eq!(colour(cubes[2].material.brush()), Some(CUBE_COLOURS[2]));
    assert_eq!(page.over(), None);

    window.close();
}

#[test]
fn the_3d_transforms_page_animates_its_rotations_while_it_is_shown() {
    let clock = Rc::new(TestGlobalClock::default());
    let services = start_catalog_compositor_application_with_clock(clock.clone());
    let window_impl = MockWindowingPlatform::create_window_mock_with_size(1100.0, 800.0);
    services.setup(&window_impl);
    let page = Transforms3DPage::new();
    let scene = page.scene();
    assert!(!page.is_animating(), "no animation before the page is shown");
    assert_eq!(scene.viewport.snapshot().models.len(), 2);

    let window = Window::with_impl(window_impl.clone());
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(page.is_animating(), "the rotations are animated while the page is shown");

    // Eight seconds a turn: two seconds in, the large cube has turned a quarter, the small one
    // (a turn in twelve seconds) less.
    clock.pulse(SignedDuration::from_seconds(0.0));
    clock.pulse(SignedDuration::from_seconds(2.0));
    let (spin, orbit) = (scene.spin.angle(), scene.orbit.angle());
    assert!(spin > 45.0 && spin < 135.0, "{spin}");
    assert!(orbit > 0.0 && orbit < spin, "{orbit}");

    // The controls of the large cube.
    page.get_control::<ComboBox>("SpinAxis").set_selected_index(3);
    assert_eq!(scene.spin.axis(), SPIN_AXES[3]);
    page.get_control::<Slider>("Scale").set_range_value(1.5);
    assert_eq!((scene.scale.scale_x(), scene.scale.scale_y(), scene.scale.scale_z()), (1.5, 1.5, 1.5));
    page.get_control::<Slider>("OffsetX").set_range_value(-1.0);
    page.get_control::<Slider>("OffsetY").set_range_value(0.5);
    assert_eq!((scene.offset.offset_x(), scene.offset.offset_y()), (-1.0, 0.5));

    // Unchecked, the animations stop and the angles return to the ones of the document.
    page.get_control::<CheckBox>("Animate").set_is_checked(Some(false));
    assert!(!page.is_animating());
    assert_eq!((scene.spin.angle(), scene.orbit.angle()), (0.0, 0.0));
    page.get_control::<CheckBox>("Animate").set_is_checked(Some(true));
    assert!(page.is_animating());

    // A page that leaves the visual tree keeps no animation.
    window.close();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!page.is_animating(), "no animation once the page is not shown");
}
