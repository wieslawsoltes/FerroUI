//! Tests of the backdrop effect of a visual (`Visual::backdrop_effect`): the tree behind the
//! visual drawn again by the backdrop replay into an effect layer, cut to the backdrop bounds,
//! with no read-back of the target. A child module of the compositor tests, whose fixture
//! renders the composition visuals into a recording drawing context.

use super::{CompositorCanvas, TestBorder};
use crate::media::{
    BackdropEffectBoundsSource, BackdropEffectCache, EffectExtensions, IEffect, ImmutableBlurEffect,
};
use crate::platform::LtrbRect;
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::{Rect, Ref};
use std::rc::Rc;

fn blur(radius: f64) -> Rc<dyn IEffect> {
    Rc::new(ImmutableBlurEffect::new(radius))
}

fn reach(radius: f64) -> f64 {
    EffectExtensions::get_backdrop_sampling_radius(Some(
        &ImmutableBlurEffect::new(radius) as &dyn IEffect
    ))
}

/// Renders the whole tree of `s` on a recording drawing context, with or without effects.
fn render(s: &CompositorCanvas, effects: bool) -> Vec<String> {
    s.run_jobs();
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = effects;
    context.log_effects = true;
    s.server_visual(&s.root)
        .render(&mut context, LtrbRect::INFINITE, None, true, false, false);
    log.entries()
}

fn position(entries: &[String], prefix: &str) -> usize {
    entries
        .iter()
        .position(|entry| entry.starts_with(prefix))
        .unwrap_or_else(|| panic!("no {prefix}: {entries:?}"))
}

/// A red square under a visual without content of its own that has a backdrop effect.
fn glass_over_red(s: &CompositorCanvas) -> Ref<TestBorder> {
    s.canvas.add(&TestBorder::filled(0.0, 0.0, 100.0, 100.0));
    let glass = TestBorder::new();
    glass.set_bounds(Rect::new(20.0, 20.0, 50.0, 30.0));
    glass.set_backdrop_effect(Some(blur(10.0)));
    s.canvas.add(&glass);
    glass
}

#[test]
fn a_backdrop_effect_draws_the_tree_behind_the_visual_through_the_effect_in_its_layout_bounds() {
    let s = CompositorCanvas::new();
    glass_over_red(&s);
    let entries = render(&s, true);
    // The visual has no content: its layout rect is what it draws, and the walk reaches it.
    let reach = reach(10.0);
    let input = Rect::new(0.0, 0.0, 50.0, 30.0).inflate(reach);
    let push = position(&entries, &format!("PushEffect {input} blur(10)"));
    let pop = position(&entries, "PopEffect");
    // The output is cut to the layout bounds, in the visual's own space.
    assert_eq!(
        entries[push - 1],
        format!("PushClip {}", Rect::new(0.0, 0.0, 50.0, 30.0)),
        "{entries:?}"
    );
    // The red square is drawn for itself and again inside the effect.
    let red: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.starts_with("DrawRectangle Red"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(red.len(), 2, "{entries:?}");
    assert!(
        red[0] < push && push < red[1] && red[1] < pop,
        "{entries:?}"
    );
    assert_eq!(
        entries.iter().filter(|e| e.starts_with("PushClip")).count(),
        entries.iter().filter(|e| e.starts_with("PopClip")).count(),
        "{entries:?}"
    );
}

#[test]
fn a_backdrop_effect_records_its_area_for_the_dirty_rectangles() {
    let s = CompositorCanvas::new();
    let glass = glass_over_red(&s);
    crate::rendering::composition::server::take_backdrop_regions();
    render(&s, true);
    let regions = crate::rendering::composition::server::take_backdrop_regions();
    let visual = s.server_visual(&glass);
    let region = regions
        .iter()
        .find(|(v, _)| v.upgrade().is_some_and(|v| Rc::ptr_eq(&v, &visual)))
        .unwrap_or_else(|| panic!("no backdrop area: {regions:?}"))
        .1;
    let expected = Rect::new(20.0, 20.0, 50.0, 30.0).inflate(reach(10.0));
    assert_eq!(
        region,
        LtrbRect::new(
            expected.x.floor(),
            expected.y.floor(),
            expected.right().ceil(),
            expected.bottom().ceil()
        )
    );
}

#[test]
fn a_context_without_effects_draws_no_backdrop_effect() {
    let s = CompositorCanvas::new();
    glass_over_red(&s);
    let entries = render(&s, false);
    assert!(
        !entries.iter().any(|e| e.starts_with("PushEffect")),
        "{entries:?}"
    );
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.starts_with("DrawRectangle Red"))
            .count(),
        1,
        "{entries:?}"
    );
}

#[test]
fn subtree_bounds_of_an_empty_visual_draw_no_backdrop_effect() {
    let s = CompositorCanvas::new();
    let glass = glass_over_red(&s);
    glass.set_backdrop_effect_bounds_source(BackdropEffectBoundsSource::Subtree);
    let entries = render(&s, true);
    assert!(
        !entries.iter().any(|e| e.starts_with("PushEffect")),
        "{entries:?}"
    );
}

#[test]
fn subtree_bounds_follow_the_content_of_the_visual() {
    let s = CompositorCanvas::new();
    s.canvas.add(&TestBorder::filled(0.0, 0.0, 100.0, 100.0));
    let glass = TestBorder::layer(20.0, 20.0, 50.0, 30.0);
    let child = TestBorder::filled(5.0, 5.0, 10.0, 10.0);
    child.set_background(Some(crate::media::Brushes::green()));
    glass.add(&child);
    glass.set_backdrop_effect(Some(blur(4.0)));
    glass.set_backdrop_effect_bounds_source(BackdropEffectBoundsSource::Subtree);
    s.canvas.add(&glass);
    let entries = render(&s, true);
    let bounds = s
        .server_visual(&glass)
        .sub_tree_bounds()
        .expect("the visual has content")
        .to_rect();
    let push = position(
        &entries,
        &format!("PushEffect {} blur(4)", bounds.inflate(reach(4.0))),
    );
    assert_eq!(
        entries[push - 1],
        format!("PushClip {bounds}"),
        "{entries:?}"
    );
    // The backdrop is drawn before the content of the visual.
    assert!(
        push < position(&entries, "DrawRectangle Green"),
        "{entries:?}"
    );
}

#[test]
fn removing_the_backdrop_effect_stops_drawing_it() {
    let s = CompositorCanvas::new();
    let glass = glass_over_red(&s);
    assert!(render(&s, true).iter().any(|e| e.starts_with("PushEffect")));
    glass.set_backdrop_effect(None);
    let entries = render(&s, true);
    assert!(
        !entries.iter().any(|e| e.starts_with("PushEffect")),
        "{entries:?}"
    );
}

#[test]
fn the_backdrop_cache_mode_reaches_the_server_visual() {
    let s = CompositorCanvas::new();
    let glass = glass_over_red(&s);
    glass.set_backdrop_effect_cache(BackdropEffectCache::Retained);
    s.run_jobs();
    let visual = s.server_visual(&glass);
    assert_eq!(
        visual.backdrop_effect_cache(),
        BackdropEffectCache::Retained
    );
    assert_eq!(
        visual.backdrop_effect_bounds_source(),
        BackdropEffectBoundsSource::Layout
    );
    assert!(visual.backdrop_effect().is_some());
}

#[test]
fn backdrop_cache_names_parse() {
    assert_eq!(
        BackdropEffectCache::parse("Default"),
        Ok(BackdropEffectCache::Default)
    );
    assert_eq!(
        BackdropEffectCache::parse("Retained"),
        Ok(BackdropEffectCache::Retained)
    );
    assert_eq!(
        BackdropEffectCache::parse("Volatile"),
        Ok(BackdropEffectCache::Volatile)
    );
    assert!(BackdropEffectCache::parse("Other").is_err());
}
