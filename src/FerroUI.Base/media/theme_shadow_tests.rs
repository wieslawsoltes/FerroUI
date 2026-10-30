//! Tests of `ThemeShadow` (phase 7 of docs/porting/winui-composition.md, not
//! from upstream): the recipe of `DropShadowRecipe.h`, the themes, and the
//! receivers with their validation.

use super::{DropShadowRecipe, Shadow, ThemeShadow, ThemeShadowError, ThemeShadowTheme};
use crate::media::Color;
use crate::styling::ThemeVariant;
use crate::{Ref, Thickness, Visual};

fn black(alpha: u8) -> Color {
    Color::from_argb(alpha, 0, 0, 0)
}

#[test]
fn a_flyout_has_a_directional_shadow_only() {
    // Translation.Z 32 (a flyout): elevation 16.
    let light = DropShadowRecipe::new(32.0, ThemeShadowTheme::Light);
    assert_eq!(16.0, light.elevation);
    assert_eq!(0.0, light.ambient_opacity);
    assert!((light.directional_opacity - 0.14).abs() < 1e-6);
    assert_eq!(16.0, light.directional_blur_radius);
    assert_eq!(8.0, light.directional_y_offset);
    // 0.14 * 255 = 35.7, truncated as WinUI converts it.
    assert_eq!(black(35), light.directional_color);
    assert_eq!(black(0), light.ambient_color);
    // The insets: the blur at the sides, the blur less the offset above, the
    // blur and the offset below (WinUI's ThemeShadowFlyoutMargin 16,8,16,24).
    assert_eq!(Thickness::new(16.0, 8.0, 16.0, 24.0), light.insets);

    let dark = DropShadowRecipe::new(32.0, ThemeShadowTheme::Dark);
    assert!((dark.directional_opacity - 0.26).abs() < 1e-6);
    assert_eq!(black(66), dark.directional_color);

    // One box shadow, blurred one more than the recipe (the dummy shape is a
    // pixel smaller on each side).
    let shadows = light.box_shadows();
    assert_eq!(1, shadows.len());
    assert_eq!((0.0, 8.0, 17.0), (shadows[0].offset_x, shadows[0].offset_y, shadows[0].blur));
    assert_eq!(black(35), shadows[0].color);
}

#[test]
fn a_low_elevation_is_lighter_in_the_light_theme() {
    // Translation.Z 16 (a tool tip): elevation 8, opacity 0.08 + 0.06.
    let recipe = DropShadowRecipe::new(16.0, ThemeShadowTheme::Light);
    assert!((recipe.directional_opacity - 0.14).abs() < 1e-6);
    let recipe = DropShadowRecipe::new(8.0, ThemeShadowTheme::Light);
    assert_eq!(4.0, recipe.elevation);
    assert!((recipe.directional_opacity - 0.10).abs() < 1e-6);
    assert_eq!(Thickness::new(4.0, 2.0, 4.0, 6.0), recipe.insets);
}

#[test]
fn a_high_elevation_has_an_ambient_and_a_directional_shadow() {
    // Translation.Z 40 (a submenu): elevation 20.
    let light = DropShadowRecipe::new(40.0, ThemeShadowTheme::Light);
    assert!((light.ambient_blur_radius - 20.0 / 3.0).abs() < 1e-5);
    assert_eq!(2.0, light.ambient_y_offset);
    assert!((light.ambient_opacity - 0.15).abs() < 1e-6);
    assert!((light.directional_opacity - 0.19).abs() < 1e-6);
    assert_eq!(Thickness::new(20.0, 10.0, 20.0, 30.0), light.insets);
    let shadows = light.box_shadows();
    assert_eq!(2, shadows.len());
    // The ambient shadow first, under the directional one.
    assert_eq!(2.0, shadows[0].offset_y);
    assert_eq!(10.0, shadows[1].offset_y);

    let dark = DropShadowRecipe::new(40.0, ThemeShadowTheme::Dark);
    assert!((dark.ambient_opacity - 0.37).abs() < 1e-6);
    assert!((dark.directional_opacity - 0.37).abs() < 1e-6);

    // A content dialog: 128, the elevation 64 (the largest).
    let dialog = DropShadowRecipe::new(128.0, ThemeShadowTheme::Light);
    assert_eq!(64.0, dialog.elevation);
    assert_eq!(64.0, DropShadowRecipe::new(1000.0, ThemeShadowTheme::Light).elevation);
}

#[test]
fn no_shadow_below_an_elevation_of_two_or_in_high_contrast() {
    let flat = DropShadowRecipe::new(3.0, ThemeShadowTheme::Light);
    assert!(!flat.is_visible());
    assert!(flat.box_shadows().is_empty());
    assert!(!DropShadowRecipe::new(32.0, ThemeShadowTheme::HighContrast).is_visible());
    assert!(!DropShadowRecipe::new(64.0, ThemeShadowTheme::HighContrast).is_visible());
    assert!(DropShadowRecipe::new(4.0, ThemeShadowTheme::Light).is_visible());
}

#[test]
fn the_theme_of_a_variant() {
    assert_eq!(ThemeShadowTheme::Light, ThemeShadowTheme::of_variant(None));
    assert_eq!(ThemeShadowTheme::Light, ThemeShadowTheme::of_variant(Some(&ThemeVariant::light())));
    assert_eq!(ThemeShadowTheme::Dark, ThemeShadowTheme::of_variant(Some(&ThemeVariant::dark())));
    // A variant of an application that inherits the dark one is dark; the
    // high contrast variant of the WinUI theme (which inherits the dark one)
    // is high contrast.
    let dim = ThemeVariant::new("Dim", Some(ThemeVariant::dark()));
    assert_eq!(ThemeShadowTheme::Dark, ThemeShadowTheme::of_variant(Some(&dim)));
    let high_contrast = ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()));
    assert_eq!(ThemeShadowTheme::HighContrast, ThemeShadowTheme::of_variant(Some(&high_contrast)));
}

#[test]
fn the_corner_radius_rides_along() {
    let recipe = DropShadowRecipe::new(32.0, ThemeShadowTheme::Light).with_corner_radius(8.0, 4.0);
    assert_eq!((8.0, 4.0), (recipe.radius_x, recipe.radius_y));
    assert_eq!((0.0, 0.0), {
        let negative = recipe.with_corner_radius(-1.0, -2.0);
        (negative.radius_x, negative.radius_y)
    });
}

#[test]
fn theme_shadows_are_drop_shadows_in_winui_3() {
    assert!(ThemeShadow::is_drop_shadow_mode());
    let shadow = ThemeShadow::new();
    let base: Ref<Shadow> = shadow.clone().upcast();
    assert!(base.as_theme_shadow().is_some());
    assert!(!shadow.has_custom_receivers());
}

#[test]
fn receivers_are_held_weakly() {
    let shadow = ThemeShadow::new();
    let first = Visual::new();
    let second = Visual::new();
    shadow.receivers().append(&first).expect("an unrelated element is a receiver");
    shadow.receivers().insert_at(0, &second).expect("an unrelated element is a receiver");
    assert_eq!(2, shadow.receivers().size());
    assert!(shadow.has_custom_receivers());
    assert_eq!(Some(1), shadow.receivers().index_of(&first));
    assert!(shadow.receivers().get_at(0).is_some_and(|item| std::ptr::eq(&*item, &*second)));

    // An element that is gone reads as nothing and stays in the collection.
    drop(second);
    assert!(shadow.receivers().get_at(0).is_none());
    assert_eq!(2, shadow.receivers().size());
    assert_eq!(1, shadow.receivers().items().len());

    assert_eq!(Err(ThemeShadowError::IndexOutOfRange), shadow.receivers().remove_at(5));
    assert!(shadow.receivers().remove(&first));
    shadow.receivers().clear();
    assert_eq!(0, shadow.receivers().size());
}

#[test]
fn an_ancestor_of_a_caster_cannot_receive_its_shadow() {
    let shadow = ThemeShadow::new();
    let parent = Visual::new();
    let caster = Visual::new();
    parent.visual_children().add(caster.clone());
    let as_shadow: Ref<Shadow> = shadow.clone().upcast();
    caster.set_shadow(Some(as_shadow));

    assert_eq!(
        Err(ThemeShadowError::AncestorElementCannotBeShadowReceiver),
        shadow.receivers().append(&parent)
    );
    // The caster itself is its own ancestor in WinUI's walk.
    assert_eq!(Err(ThemeShadowError::AncestorElementCannotBeShadowReceiver), shadow.receivers().append(&caster));
    // A sibling is fine.
    let sibling = Visual::new();
    parent.visual_children().add(sibling.clone());
    assert_eq!(Ok(()), shadow.receivers().append(&sibling));

    // Once the shadow is no longer used by the caster, the parent may receive it.
    caster.set_shadow(None);
    assert_eq!(Ok(()), shadow.receivers().append(&parent));
    let _: &Visual = &parent;
}
