//! Tests for the visual members that depend on the media object model:
//! effects, cache modes, render options and invalidation by changing media
//! objects.

use crate::layout::ILayoutRoot;
use crate::media::effects::{BlurEffect, IEffect, ImmutableBlurEffect};
use crate::media::{
    BitmapCache, CacheMode, Colors, EdgeMode, IBrush, ITransform, RectangleGeometry, RenderOptions, SolidColorBrush,
    TextHintingMode, TextOptions, TextRenderingMode, TranslateTransform,
};
use crate::rendering::{IPresentationSource, IRenderer};
use crate::{Rect, Ref, Size, Visual};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Default)]
struct TestRenderer {
    dirty: Cell<u32>,
}

impl IRenderer for TestRenderer {
    fn diagnostics(&self) -> Rc<crate::rendering::RendererDiagnostics> {
        crate::rendering::RendererDiagnostics::new()
    }
    fn scene_invalidated(
        &self,
        _handler: Rc<dyn Fn(&crate::rendering::SceneInvalidatedEventArgs)>,
    ) -> Rc<dyn crate::reactive::IDisposable> {
        crate::reactive::Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: std::any::TypeId) -> Option<crate::rendering::composition::RenderInterfaceFeature> {
        None
    }
    fn add_dirty(&self, _visual: &Visual) {
        self.dirty.set(self.dirty.get() + 1);
    }
    fn recalculate_children(&self, _visual: &Visual) {}
    fn resized(&self, _size: Size) {}
    fn paint(&self, _rect: Rect) {}
    fn start(&self) {}
    fn stop(&self) {}
    fn dispose(&self) {}
}

struct TestSource {
    root: Ref<Visual>,
    renderer: Rc<TestRenderer>,
}

impl IPresentationSource for TestSource {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        Some(self.root.clone())
    }
    fn render_scaling(&self) -> f64 {
        1.0
    }
    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.clone()
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        unimplemented!("not used by these tests")
    }
    fn hit_tester(&self) -> Rc<dyn crate::rendering::IHitTester> {
        unimplemented!("not used by these tests")
    }
    fn input_root(&self) -> Rc<dyn crate::input::IInputRoot> {
        unimplemented!("not used by these tests")
    }
    fn client_size(&self) -> Size {
        Size::new(100.0, 100.0)
    }
}

/// An attached visual and the renderer that counts its invalidations.
fn attached() -> (Ref<Visual>, Rc<TestRenderer>) {
    let visual = Visual::new();
    let renderer = Rc::new(TestRenderer::default());
    let source: Rc<dyn IPresentationSource> = Rc::new(TestSource { root: visual.clone(), renderer: renderer.clone() });
    visual.set_presentation_source_for_root_visual(Some(source));
    renderer.dirty.set(0);
    (visual, renderer)
}

#[test]
fn effect_affects_render() {
    let (visual, renderer) = attached();
    assert!(visual.effect().is_none());

    let effect: Rc<dyn IEffect> = Rc::new(ImmutableBlurEffect::new(4.0));
    visual.set_effect(Some(effect.clone()));
    assert_eq!(1, renderer.dirty.get());
    assert!(*visual.effect().unwrap() == *effect);

    visual.set_effect(None);
    assert_eq!(2, renderer.dirty.get());
}

#[test]
fn changing_a_mutable_effect_invalidates_the_visual() {
    let (visual, renderer) = attached();
    let effect = BlurEffect::new();
    visual.set_effect(Some(effect.clone().into()));
    assert_eq!(1, renderer.dirty.get());

    effect.set_radius(10.0);
    assert_eq!(2, renderer.dirty.get());

    // The subscription follows the property value.
    let other = BlurEffect::new();
    visual.set_effect(Some(other.clone().into()));
    assert_eq!(3, renderer.dirty.get());
    effect.set_radius(20.0);
    assert_eq!(3, renderer.dirty.get());
    other.set_radius(20.0);
    assert_eq!(4, renderer.dirty.get());

    visual.set_effect(None);
    assert_eq!(5, renderer.dirty.get());
    other.set_radius(30.0);
    assert_eq!(5, renderer.dirty.get());
}

#[test]
fn changing_a_mutable_opacity_mask_or_clip_invalidates_the_visual() {
    let (visual, renderer) = attached();

    let brush = SolidColorBrush::with_color(Colors::RED);
    let mask: Rc<dyn IBrush> = brush.clone().into();
    visual.set_opacity_mask(Some(mask));
    assert_eq!(1, renderer.dirty.get());
    brush.set_color(Colors::BLUE);
    assert_eq!(2, renderer.dirty.get());

    let clip = RectangleGeometry::new();
    visual.set_clip(clip.clone().upcast::<crate::media::Geometry>());
    assert_eq!(3, renderer.dirty.get());
    clip.set_rect(Rect::new(0.0, 0.0, 5.0, 5.0));
    assert_eq!(4, renderer.dirty.get());

    // Both subscriptions stay independent.
    brush.set_opacity(0.5);
    assert_eq!(5, renderer.dirty.get());
    visual.set_clip(None);
    assert_eq!(6, renderer.dirty.get());
    clip.set_rect(Rect::new(0.0, 0.0, 6.0, 6.0));
    assert_eq!(6, renderer.dirty.get());
    brush.set_opacity(0.25);
    assert_eq!(7, renderer.dirty.get());
}

#[test]
fn a_dropped_visual_is_not_kept_alive_by_its_media_values() {
    let effect = BlurEffect::new();
    let weak = {
        let (visual, _renderer) = attached();
        visual.set_effect(Some(effect.clone().into()));
        visual.set_presentation_source_for_root_visual(None);
        visual.downgrade()
    };
    assert!(weak.upgrade().is_none());
    // Invalidation of the effect after the visual is gone is harmless.
    effect.set_radius(1.0);
}

#[test]
fn cache_mode_property() {
    let (visual, renderer) = attached();
    assert!(visual.cache_mode().is_none());
    let cache = BitmapCache::new();
    visual.set_cache_mode(cache.clone().upcast::<CacheMode>());
    assert!(visual.cache_mode().unwrap().is::<BitmapCache>());
    // The cache mode does not affect rendering by itself.
    assert_eq!(0, renderer.dirty.get());
    visual.set_cache_mode(None);
    assert!(visual.cache_mode().is_none());
}

#[test]
fn render_options_invalidate_the_visual() {
    let (visual, renderer) = attached();
    RenderOptions::set_edge_mode(&visual, EdgeMode::Aliased);
    assert_eq!(1, renderer.dirty.get());
    assert_eq!(EdgeMode::Aliased, RenderOptions::get_edge_mode(&visual));
}

#[test]
fn text_options_invalidate_the_visual() {
    let (visual, renderer) = attached();
    assert_eq!(TextOptions::default(), TextOptions::get_text_options(&visual));

    TextOptions::set_text_hinting_mode(&visual, TextHintingMode::Light);
    assert_eq!(1, renderer.dirty.get());
    assert_eq!(TextHintingMode::Light, TextOptions::get_text_hinting_mode(&visual));

    TextOptions::set_text_rendering_mode(&visual, TextRenderingMode::Antialias);
    assert_eq!(2, renderer.dirty.get());
    assert_eq!(
        TextOptions {
            text_rendering_mode: TextRenderingMode::Antialias,
            text_hinting_mode: TextHintingMode::Light,
            ..Default::default()
        },
        TextOptions::get_text_options(&visual)
    );

    TextOptions::set_text_options(&visual, TextOptions::default());
    assert_eq!(3, renderer.dirty.get());
    assert_eq!(TextRenderingMode::Unspecified, TextOptions::get_text_rendering_mode(&visual));
    // The render options are separate.
    assert_eq!(RenderOptions::default().text_rendering_mode, RenderOptions::get_text_rendering_mode(&visual));
}

#[test]
fn changing_the_render_transform_of_an_attached_visual_invalidates_it() {
    let (visual, renderer) = attached();
    let first = TranslateTransform::new();
    let second = TranslateTransform::new();
    let handle = |transform: &Ref<TranslateTransform>| -> Option<Rc<dyn ITransform>> { Some(transform.into()) };

    visual.set_render_transform(handle(&first));
    assert_eq!(1, renderer.dirty.get());
    first.set_x(5.0);
    assert_eq!(2, renderer.dirty.get());

    // Replacing the transform moves the subscription to the new value.
    visual.set_render_transform(handle(&second));
    assert_eq!(3, renderer.dirty.get());
    first.set_x(6.0);
    assert_eq!(3, renderer.dirty.get());
    second.set_x(1.0);
    assert_eq!(4, renderer.dirty.get());

    visual.set_render_transform(None);
    assert_eq!(5, renderer.dirty.get());
    second.set_x(2.0);
    assert_eq!(5, renderer.dirty.get());
}

#[test]
fn the_render_transform_is_only_observed_while_the_visual_is_attached() {
    let visual = Visual::new();
    let transform = TranslateTransform::new();
    visual.set_render_transform(Some((&transform).into()));

    // Attaching subscribes to the transform that is already set.
    let renderer = Rc::new(TestRenderer::default());
    let source: Rc<dyn IPresentationSource> = Rc::new(TestSource { root: visual.clone(), renderer: renderer.clone() });
    visual.set_presentation_source_for_root_visual(Some(source));
    renderer.dirty.set(0);
    transform.set_x(1.0);
    assert_eq!(1, renderer.dirty.get());

    // Detaching unsubscribes.
    visual.set_presentation_source_for_root_visual(None);
    renderer.dirty.set(0);
    transform.set_x(2.0);
    assert_eq!(0, renderer.dirty.get());

    // A transform set while detached is not observed until the visual is
    // attached again.
    let other = TranslateTransform::new();
    visual.set_render_transform(Some((&other).into()));
    other.set_x(1.0);
    assert_eq!(0, renderer.dirty.get());

    let source: Rc<dyn IPresentationSource> = Rc::new(TestSource { root: visual.clone(), renderer: renderer.clone() });
    visual.set_presentation_source_for_root_visual(Some(source));
    renderer.dirty.set(0);
    transform.set_x(3.0);
    assert_eq!(0, renderer.dirty.get());
    other.set_x(2.0);
    assert_eq!(1, renderer.dirty.get());
    visual.set_presentation_source_for_root_visual(None);
}

#[test]
fn rooted_visual_children_count_follows_attachment() {
    let before = Visual::rooted_visual_children_count();
    let (visual, _renderer) = attached();
    assert_eq!(before + 1, Visual::rooted_visual_children_count());

    let child = Visual::new();
    let grandchild = Visual::new();
    child.visual_children().add(grandchild.clone());
    assert_eq!(before + 1, Visual::rooted_visual_children_count());
    visual.visual_children().add(child.clone());
    assert_eq!(before + 3, Visual::rooted_visual_children_count());

    child.visual_children().remove(&grandchild);
    assert_eq!(before + 2, Visual::rooted_visual_children_count());

    visual.set_presentation_source_for_root_visual(None);
    assert_eq!(before, Visual::rooted_visual_children_count());
}
