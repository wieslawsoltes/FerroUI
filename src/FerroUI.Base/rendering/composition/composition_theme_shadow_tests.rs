//! Tests of the theme shadows of elements (phase 7, not from upstream): the
//! shadow visual a parent shows before a caster, what it draws, and how it
//! follows the elevation, the theme, the size and the visibility of the
//! caster. A child module of the compositor tests, whose fixture renders the
//! composition visuals into a recording drawing context.

use super::{CompositorCanvas, TestBorder};
use crate::media::{Color, Shadow, ThemeShadow};
use crate::rendering::composition::CompositionVisual;
use crate::styling::ThemeVariant;
use crate::{Rect, Ref, Vector3D};
use std::rc::Rc;

/// A border at (10, 20) of 100 x 50 in the canvas, casting a theme shadow at
/// the elevation `z`.
fn caster(s: &CompositorCanvas, z: f64) -> (Ref<TestBorder>, Ref<ThemeShadow>) {
    let border = TestBorder::filled(10.0, 20.0, 100.0, 50.0);
    let shadow = ThemeShadow::new();
    border.set_translation(Vector3D::new(0.0, 0.0, z));
    let as_shadow: Ref<Shadow> = shadow.clone().upcast();
    border.set_shadow(Some(as_shadow));
    s.canvas.add(&border);
    s.run_jobs();
    (border, shadow)
}

fn children(visual: &Rc<CompositionVisual>) -> Vec<Rc<CompositionVisual>> {
    let children = visual.children();
    (0..children.count()).map(|index| children.get(index)).collect()
}

#[test]
fn the_parent_shows_the_shadow_visual_just_before_the_caster() {
    let s = CompositorCanvas::new();
    let before = TestBorder::filled(0.0, 0.0, 10.0, 10.0);
    s.canvas.add(&before);
    let (border, _shadow) = caster(&s, 32.0);

    let canvas = s.canvas.composition_visual().expect("the canvas has a composition visual");
    let caster_visual = border.composition_visual().expect("the caster has a composition visual");
    let shadow_visual = border.theme_shadow_visual().expect("the caster has a shadow visual");
    let list = children(&canvas);
    assert_eq!(3, list.len());
    assert!(Rc::ptr_eq(&list[0], &before.composition_visual().unwrap()));
    assert!(Rc::ptr_eq(&list[1], shadow_visual.visual()));
    assert!(Rc::ptr_eq(&list[2], &caster_visual));

    // It follows the transform of the caster, is not hit-testable, and has
    // the size of the caster.
    let transform_parent = shadow_visual.visual().parent_for_transform().expect("a transform parent");
    assert!(Rc::ptr_eq(&transform_parent, &caster_visual));
    assert!(!shadow_visual.visual().is_hit_test_visible());
    assert_eq!(crate::Vector::new(100.0, 50.0), shadow_visual.visual().size());
    assert!(shadow_visual.visual().visible());

    // A flyout's shadow in the light theme: the directional one only.
    let shadows = shadow_visual.box_shadows();
    assert_eq!(1, shadows.len());
    assert_eq!((8.0, 17.0), (shadows[0].offset_y, shadows[0].blur));
    assert_eq!(Color::from_argb(35, 0, 0, 0), shadows[0].color);
}

#[test]
fn the_shadow_visual_draws_the_box_shadows_of_the_caster_one_pixel_smaller() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (_border, _shadow) = caster(&s, 32.0);

    let entries = log.entries();
    let shadow = entries
        .iter()
        .position(|e| e == "DrawRectangle none none 1, 1, 98, 48 shadows=1")
        .unwrap_or_else(|| panic!("the shadow is drawn: {entries:?}"));
    let content = entries
        .iter()
        .position(|e| e.starts_with("DrawRectangle Red none 0, 0, 100, 50"))
        .unwrap_or_else(|| panic!("the caster is drawn: {entries:?}"));
    assert!(shadow < content, "the shadow is under the caster: {entries:?}");
}

#[test]
fn the_shadow_follows_the_elevation_and_the_theme() {
    let s = CompositorCanvas::new();
    let (border, _shadow) = caster(&s, 32.0);
    let shadow_visual = border.theme_shadow_visual().expect("a shadow visual");

    // A submenu: the ambient and the directional shadow.
    border.set_translation(Vector3D::new(0.0, 0.0, 40.0));
    s.run_jobs();
    assert_eq!(2, shadow_visual.box_shadows().len());

    // The dark theme.
    border.set_translation(Vector3D::new(0.0, 0.0, 32.0));
    border.set_value(ThemeVariant::actual_theme_variant_property(), Some(ThemeVariant::dark()));
    s.run_jobs();
    let shadows = shadow_visual.box_shadows();
    assert_eq!(1, shadows.len());
    assert_eq!(Color::from_argb(66, 0, 0, 0), shadows[0].color);

    // High contrast: no shadow.
    let high_contrast = ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()));
    border.set_value(ThemeVariant::actual_theme_variant_property(), Some(high_contrast));
    s.run_jobs();
    assert!(!shadow_visual.visual().visible());

    // No elevation: no shadow.
    border.set_value(ThemeVariant::actual_theme_variant_property(), Some(ThemeVariant::light()));
    border.set_translation(Vector3D::new(0.0, 0.0, 0.0));
    s.run_jobs();
    assert!(!shadow_visual.visual().visible());
    border.set_translation(Vector3D::new(0.0, 0.0, 32.0));
    s.run_jobs();
    assert!(shadow_visual.visual().visible());
}

#[test]
fn the_shadow_follows_the_size_and_the_visibility_of_the_caster() {
    let s = CompositorCanvas::new();
    let (border, _shadow) = caster(&s, 32.0);
    let shadow_visual = border.theme_shadow_visual().expect("a shadow visual");

    border.set_bounds(Rect::new(10.0, 20.0, 60.0, 30.0));
    border.invalidate_visual();
    s.run_jobs();
    assert_eq!(crate::Vector::new(60.0, 30.0), shadow_visual.visual().size());

    border.set_is_visible(false);
    s.run_jobs();
    assert!(!shadow_visual.visual().visible());
    border.set_is_visible(true);
    s.run_jobs();
    assert!(shadow_visual.visual().visible());
}

#[test]
fn removing_the_shadow_removes_the_shadow_visual() {
    let s = CompositorCanvas::new();
    let (border, _shadow) = caster(&s, 32.0);
    let canvas = s.canvas.composition_visual().expect("the canvas has a composition visual");
    assert_eq!(2, children(&canvas).len());

    border.set_shadow(None);
    s.run_jobs();
    assert!(border.theme_shadow_visual().is_none());
    let list = children(&canvas);
    assert_eq!(1, list.len());
    assert!(Rc::ptr_eq(&list[0], &border.composition_visual().unwrap()));
}

#[test]
fn the_child_of_a_popup_casts_the_shadow_of_the_popup() {
    let s = CompositorCanvas::new();
    // A visual standing for a popup: its shadow is cast by its child, with
    // its elevation.
    let popup = TestBorder::new();
    popup.set_bounds(Rect::new(0.0, 0.0, 0.0, 0.0));
    popup.set_translation(Vector3D::new(0.0, 0.0, 40.0));
    let shadow = ThemeShadow::new();
    let as_shadow: Ref<Shadow> = shadow.clone().upcast();
    popup.set_shadow(Some(as_shadow));
    popup.set_theme_shadow_delegated_to_child(true);
    s.canvas.add(&popup);

    let child = TestBorder::filled(10.0, 10.0, 80.0, 40.0);
    s.canvas.add(&child);
    let popup_visual: &crate::Visual = &popup;
    child.set_theme_shadow_popup(Some(popup_visual));
    s.run_jobs();

    assert!(popup.theme_shadow_caster().is_none());
    assert!(popup.theme_shadow_visual().is_none());
    let (cast, elevation) = child.theme_shadow_caster().expect("the child casts the popup's shadow");
    assert!(std::ptr::eq(&*cast, &*shadow));
    assert!(std::ptr::eq(&*elevation, popup_visual));
    let shadow_visual = child.theme_shadow_visual().expect("the child has a shadow visual");
    // The elevation of the popup: 40, the ambient and directional shadows.
    assert_eq!(2, shadow_visual.box_shadows().len());

    child.set_theme_shadow_popup(None);
    s.run_jobs();
    assert!(child.theme_shadow_visual().is_none());
}
