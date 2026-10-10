//! Port of upstream's `Media/EffectTests.cs`.

use crate::test_base::{CompareOptions, TestBase};
use ferroui_base::layout::Orientation;
use ferroui_base::media::effects::{DropShadowEffect, IEffect, ImmutableDropShadowEffect};
use ferroui_base::media::{BitmapCache, Brushes, Color, Colors, SolidColorBrush};
use ferroui_base::Thickness;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Border, Panel, StackPanel};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\Effects")
}

#[test]
fn drop_shadow_effect() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    let child = Border::new();
    child.set_background(None);
    child.set_margin(Thickness::uniform(40.0));
    let effect: Rc<dyn IEffect> = Rc::new(ImmutableDropShadowEffect::new(20.0, 30.0, 5.0, Colors::GREEN, 1.0));
    child.set_effect(Some(effect));
    let inner = Border::new();
    inner.set_background(Some(SolidColorBrush::with_color(Color::from_argb(128, 0, 0, 255)).into()));
    inner.set_border_brush(Some(Brushes::red()));
    inner.set_border_thickness(Thickness::uniform(5.0));
    child.set_child(inner);
    target.set_child(child);

    t.render_to_file(&target, "DropShadowEffect");
    t.compare_images_with("DropShadowEffect", CompareOptions { skip_immediate: true, ..Default::default() });
}

#[test]
fn effect_followed_by_non_effect() {
    let t = base();
    let target = Border::new();
    target.set_background(Some(Brushes::white()));
    target.set_width(200.0);
    target.set_height(200.0);
    let panel = Panel::new();
    panel.set_margin(Thickness::uniform(25.0));
    let first = Rectangle::new();
    first.set_fill(Some(Brushes::yellow()));
    let effect = DropShadowEffect::new();
    effect.set_opacity(1.0);
    effect.set_offset_x(0.0);
    effect.set_offset_y(0.0);
    effect.set_color(Colors::BLACK);
    effect.set_blur_radius(50.0);
    first.set_effect(Some(effect.into()));
    panel.children().add(first);
    let second = Rectangle::new();
    second.set_fill(Some(SolidColorBrush::with_color(Color::from_uint32(0x7F007FFF)).into()));
    panel.children().add(second);
    target.set_child(panel);

    t.render_to_file(&target, "EffectFollowedByNonEffect");
    t.compare_images_with("EffectFollowedByNonEffect", CompareOptions { skip_immediate: true, ..Default::default() });
}

#[test]
fn cached_sibling_followed_by_effect() {
    let t = base();
    let target = Border::new();
    target.set_background(Some(Brushes::white()));
    target.set_width(200.0);
    target.set_height(200.0);
    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Horizontal);
    panel.set_spacing(10.0);
    panel.set_margin(Thickness::uniform(15.0));
    let first = Rectangle::new();
    first.set_width(50.0);
    first.set_height(50.0);
    first.set_fill(Some(Brushes::yellow()));
    first.set_cache_mode(BitmapCache::new());
    panel.children().add(first);
    let second = Rectangle::new();
    second.set_width(50.0);
    second.set_height(50.0);
    second.set_fill(Some(Brushes::blue()));
    let effect = DropShadowEffect::new();
    effect.set_opacity(1.0);
    effect.set_offset_x(0.0);
    effect.set_offset_y(0.0);
    effect.set_color(Colors::BLACK);
    effect.set_blur_radius(20.0);
    second.set_effect(Some(effect.into()));
    panel.children().add(second);
    let third = Rectangle::new();
    third.set_width(50.0);
    third.set_height(50.0);
    third.set_fill(Some(Brushes::green()));
    panel.children().add(third);
    target.set_child(panel);

    t.render_to_file(&target, "CachedSiblingFollowedByEffect");
    t.compare_images_with(
        "CachedSiblingFollowedByEffect",
        CompareOptions { skip_immediate: true, ..Default::default() },
    );
}
