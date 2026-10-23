//! Tests of the WinUI materials (winui-materials.md): the effects of the
//! materials (blend, colour matrix, arithmetic composite, alpha mask,
//! border, composite modes, scene lighting), the backdrop and host backdrop
//! brushes, composition brushes as the brushes of elements, the XAML
//! composition brushes and `AcrylicBrush`, XAML lights and the system
//! backdrop controllers. A child module of the compositor tests, whose
//! fixture renders the composition visuals into a recording drawing context.

use super::{CompositorCanvas, TestBorder};
use crate::media::imaging::Bitmap;
use crate::media::{
    AcrylicBrush, Color, ColorMatrix, ColorMatrixAlphaMode, Colors, IBrush, MaterialPolicy, XamlCompositionBrushBase,
    XamlLight,
};
use crate::platform::{AlphaFormat, LtrbRect, PixelFormats};
use crate::rendering::composition::effects::{
    AlphaMaskEffect, ArithmeticCompositeEffect, BlendEffect, BlendEffectMode, BorderEffect, CanvasComposite,
    CanvasEdgeBehavior, ColorMatrixEffect, ColorSourceEffect, CompositeEffect, GaussianBlurEffect, GraphicsEffectSource,
    InvertEffect, SaturationEffect, SceneLightingEffect, TintEffect,
};
use crate::rendering::composition::system_backdrops::{
    DesktopAcrylicController, ICompositionSupportsSystemBackdrop, MicaController, MicaKind, PlatformSystemBackdrop,
    PlatformSystemBackdropKind, SystemBackdropConfiguration, SystemBackdropState, SystemBackdropSupport,
    SystemBackdropTheme,
};
use crate::rendering::composition::{
    CompositionBrush, CompositionEffectBrush, CompositionEffectFactoryError, CompositionEffectFactoryLoadStatus,
    CompositionEffectSourceParameter, CompositionVisual, Compositor, ElementComposition,
    LoadedImageSurface,
};
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::{instantiate, PixelSize, Ref, Vector};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

fn show(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) {
    ElementComposition::set_element_child_visual(&s.canvas, Some(visual.clone()));
    s.run_jobs();
}

fn run_for(s: &CompositorCanvas, duration: Duration) {
    let started = s.compositor.server().server_now();
    while s.compositor.server().server_now() < started + duration {
        s.render_loop.tick();
        crate::threading::Dispatcher::ui_thread().run_jobs(None);
    }
    s.run_jobs();
}

/// Renders the server side of `visual` on a recording drawing context that
/// has effects and blend and colour matrix layers.
fn render_with_layers(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.log_effects = true;
    context.supports_layer_effects = true;
    s.server_object(visual.server()).render(&mut context, LtrbRect::INFINITE, None, true, false, false);
    log.entries()
}

fn position_starting(entries: &[String], prefix: &str) -> usize {
    entries.iter().position(|e| e.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix} is drawn: {entries:?}"))
}

fn parameter(name: &str) -> Option<GraphicsEffectSource> {
    Some(CompositionEffectSourceParameter::new(name).into())
}

fn sprite_with(s: &CompositorCanvas, brush: &Rc<CompositionBrush>) -> crate::rendering::composition::SpriteVisual {
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 50.0));
    sprite.set_brush(Some(brush.clone()));
    sprite
}

fn blue(s: &CompositorCanvas) -> Option<Rc<CompositionBrush>> {
    Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone())
}

// --- effects -------------------------------------------------------------------------

#[test]
fn a_blend_draws_its_foreground_in_a_blend_layer_over_its_background() {
    let s = CompositorCanvas::new();
    let factory = s.compositor.create_effect_factory(BlendEffect {
        name: String::new(),
        mode: BlendEffectMode::Luminosity,
        background: Some(ColorSourceEffect { name: String::new(), color: Colors::RED }.into()),
        foreground: parameter("source"),
    });
    assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
    let brush = factory.create_brush();
    brush.set_source_parameter("source", blue(&s));
    let sprite = sprite_with(&s, brush.brush());
    show(&s, &sprite);
    let entries = render_with_layers(&s, &sprite);
    let layer = position_starting(&entries, "PushLayer 0, 0, 100, 50");
    let red = position_starting(&entries, "DrawRectangle Red");
    let blend = position_starting(&entries, "PushBlendLayer 0, 0, 100, 50 Luminosity");
    let blue = position_starting(&entries, "DrawRectangle Blue");
    let pop_blend = position_starting(&entries, "PopBlendLayer");
    assert!(layer < red && red < blend && blend < blue && blue < pop_blend, "{entries:?}");
}

#[test]
fn colour_matrices_load_and_push_a_matrix_layer() {
    let s = CompositorCanvas::new();
    for effect in [
        SaturationEffect { name: "Saturation".into(), saturation: 0.0, source: parameter("source") }.into(),
        TintEffect { name: String::new(), color: Colors::RED, clamp_output: false, source: parameter("source") }.into(),
        InvertEffect { name: String::new(), source: parameter("source") }.into(),
        ColorMatrixEffect {
            name: String::new(),
            color_matrix: ColorMatrix::luminance_to_alpha(),
            alpha_mode: ColorMatrixAlphaMode::Straight,
            clamp_output: false,
            source: parameter("source"),
        }
        .into(),
    ] {
        let effect: crate::rendering::composition::effects::GraphicsEffect = effect;
        let factory = s.compositor.create_effect_factory(effect);
        assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
        let brush = factory.create_brush();
        brush.set_source_parameter("source", blue(&s));
        let sprite = sprite_with(&s, brush.brush());
        show(&s, &sprite);
        let entries = render_with_layers(&s, &sprite);
        let push = position_starting(&entries, "PushColorMatrixLayer 0, 0, 100, 50");
        let blue = position_starting(&entries, "DrawRectangle Blue");
        let pop = position_starting(&entries, "PopColorMatrixLayer");
        assert!(push < blue && blue < pop, "{entries:?}");
    }
}

#[test]
fn an_animated_saturation_is_an_entry_of_the_properties() {
    let s = CompositorCanvas::new();
    let factory = s.compositor.create_effect_factory_with_properties(
        SaturationEffect { name: "Saturation".into(), saturation: 0.25, source: parameter("source") },
        &["Saturation.Saturation"],
    );
    let brush = factory.create_brush();
    assert_eq!(
        (crate::rendering::composition::CompositionGetValueStatus::Succeeded, 0.25),
        brush.properties().try_get_scalar("Saturation.Saturation")
    );
}

#[test]
fn an_arithmetic_composite_with_a_colour_is_a_premultiplied_matrix() {
    let s = CompositorCanvas::new();
    let factory = s.compositor.create_effect_factory(ArithmeticCompositeEffect {
        name: String::new(),
        multiply_amount: -2.0,
        source1_amount: 2.0,
        source2_amount: 2.0,
        offset: -1.0,
        clamp_output: false,
        source1: parameter("source"),
        source2: Some(ColorSourceEffect { name: "Base".into(), color: Color::default() }.into()),
    });
    assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
    let brush = factory.create_brush();
    brush.set_source_parameter("source", blue(&s));
    let sprite = sprite_with(&s, brush.brush());
    show(&s, &sprite);
    let entries = render_with_layers(&s, &sprite);
    position_starting(&entries, "PushColorMatrixLayer 0, 0, 100, 50 Premultiplied");

    // Two images: not drawn.
    let both = s.compositor.create_effect_factory(ArithmeticCompositeEffect {
        source1: parameter("a"),
        source2: parameter("b"),
        ..Default::default()
    });
    assert_eq!(CompositionEffectFactoryLoadStatus::Other, both.load_status());
    assert_eq!(
        Some(&CompositionEffectFactoryError::EffectTypeNotSupported("ArithmeticCompositeEffect")),
        both.extended_error()
    );
}

#[test]
fn an_alpha_mask_keeps_the_source_where_the_mask_is() {
    let s = CompositorCanvas::new();
    let factory = s.compositor.create_effect_factory(AlphaMaskEffect {
        name: String::new(),
        source: parameter("source"),
        alpha_mask: Some(ColorSourceEffect { name: String::new(), color: Colors::RED }.into()),
    });
    let brush = factory.create_brush();
    brush.set_source_parameter("source", blue(&s));
    let sprite = sprite_with(&s, brush.brush());
    show(&s, &sprite);
    let entries = render_with_layers(&s, &sprite);
    let blue = position_starting(&entries, "DrawRectangle Blue");
    let mask = position_starting(&entries, "PushBlendLayer 0, 0, 100, 50 DestinationIn");
    let red = position_starting(&entries, "DrawRectangle Red");
    assert!(blue < mask && mask < red, "{entries:?}");
}

#[test]
fn composite_modes_compose_in_a_layer() {
    let s = CompositorCanvas::new();
    let factory = s.compositor.create_effect_factory(CompositeEffect {
        name: String::new(),
        mode: CanvasComposite::Xor,
        sources: vec![
            ColorSourceEffect { name: String::new(), color: Colors::RED }.into(),
            CompositionEffectSourceParameter::new("source").into(),
        ],
    });
    assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
    let brush = factory.create_brush();
    brush.set_source_parameter("source", blue(&s));
    let sprite = sprite_with(&s, brush.brush());
    show(&s, &sprite);
    let entries = render_with_layers(&s, &sprite);
    let red = position_starting(&entries, "DrawRectangle Red");
    let xor = position_starting(&entries, "PushBlendLayer 0, 0, 100, 50 Xor");
    let blue = position_starting(&entries, "DrawRectangle Blue");
    assert!(red < xor && xor < blue, "{entries:?}");

    let invert = s.compositor.create_effect_factory(CompositeEffect {
        mode: CanvasComposite::MaskInvert,
        sources: vec![CompositionEffectSourceParameter::new("a").into()],
        ..Default::default()
    });
    assert_eq!(
        Some(&CompositionEffectFactoryError::CompositeModeNotSupported(CanvasComposite::MaskInvert)),
        invert.extended_error()
    );
    let dissolve = s.compositor.create_effect_factory(BlendEffect {
        mode: BlendEffectMode::Dissolve,
        background: parameter("a"),
        foreground: parameter("b"),
        ..Default::default()
    });
    assert_eq!(
        Some(&CompositionEffectFactoryError::BlendModeNotSupported(BlendEffectMode::Dissolve)),
        dissolve.extended_error()
    );
}

#[test]
fn a_border_effect_tiles_a_surface_brush() {
    let s = CompositorCanvas::new();
    let bitmap = Bitmap::from_pixels(
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
        &[0u8; 20 * 10 * 4],
        PixelSize::new(20, 10),
        Vector::new(96.0, 96.0),
        20 * 4,
    );
    let image = LoadedImageSurface::from_bitmap(&s.compositor, &bitmap);
    let surface = s.compositor.create_surface_brush_with_surface(image as Rc<dyn crate::rendering::composition::ICompositionSurface>);
    surface.set_stretch(crate::rendering::composition::CompositionStretch::None);
    let factory = s.compositor.create_effect_factory(BorderEffect {
        name: String::new(),
        extend_x: CanvasEdgeBehavior::Wrap,
        extend_y: CanvasEdgeBehavior::Wrap,
        source: parameter("source"),
    });
    let brush = factory.create_brush();
    brush.set_source_parameter("source", Some((*surface).clone()));
    let sprite = sprite_with(&s, brush.brush());
    show(&s, &sprite);
    let entries = render_with_layers(&s, &sprite);
    // 100 x 50 in tiles of 20 x 10.
    assert_eq!(25, entries.iter().filter(|entry| entry.starts_with("DrawBitmap")).count(), "{entries:?}");
}

#[test]
fn scene_lighting_draws_the_light_of_the_lights_that_target_the_visual() {
    let s = CompositorCanvas::new();
    let factory = s.compositor.create_effect_factory(SceneLightingEffect::default());
    let brush = factory.create_brush();
    let sprite = sprite_with(&s, brush.brush());
    let light = s.compositor.create_ambient_light();
    light.targets().add((*sprite).clone());
    show(&s, &sprite);
    let entries = render_with_layers(&s, &sprite);
    position_starting(&entries, "DrawBitmap 1 0, 0, 25, 13 0, 0, 100, 50");
    assert!(crate::rendering::composition::server::SCENE_LIGHTING_DISTANCE_UNIT > 1.0);
}

// --- the backdrop ----------------------------------------------------------------------

#[test]
fn the_backdrop_brush_draws_what_is_under_the_visual() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    // A red element under the sprite.
    s.canvas.add(&TestBorder::filled(0.0, 0.0, 100.0, 50.0));
    let sprite = sprite_with(&s, &(*s.compositor.create_backdrop_brush()).clone());
    log.clear();
    show(&s, &sprite);
    // The element, then the element again as the backdrop of the sprite.
    assert_eq!(2, log.count("DrawRectangle Red none 0, 0, 100, 50"), "{:?}", log.entries());

    // A backdrop brush in an effect: the backdrop is drawn inside the blur.
    let factory = s.compositor.create_effect_factory(GaussianBlurEffect {
        name: "Blur".into(),
        blur_amount: 10.0,
        source: parameter("backdrop"),
        ..Default::default()
    });
    let blur = factory.create_brush();
    blur.set_source_parameter("backdrop", Some((*s.compositor.create_backdrop_brush()).clone()));
    let blurred = sprite_with(&s, blur.brush());
    log.clear();
    show(&s, &blurred);
    assert_eq!(2, log.count("DrawRectangle Red none 0, 0, 100, 50"), "{:?}", log.entries());
}

#[test]
fn a_backdrop_outside_a_render_is_empty() {
    let s = CompositorCanvas::new();
    s.canvas.add(&TestBorder::filled(0.0, 0.0, 100.0, 50.0));
    let sprite = sprite_with(&s, &(*s.compositor.create_backdrop_brush()).clone());
    show(&s, &sprite);
    // Rendering the sprite alone: it is the root of the render, nothing is
    // under it.
    let entries = render_with_layers(&s, &sprite);
    assert_eq!(0, entries.iter().filter(|entry| entry.starts_with("DrawRectangle Red")).count(), "{entries:?}");
}

#[test]
fn a_dirty_rectangle_under_a_backdrop_redraws_all_of_it() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let under = TestBorder::filled(0.0, 0.0, 10.0, 10.0);
    s.canvas.add(&under);
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(200.0, 100.0));
    sprite.set_brush(Some((*s.compositor.create_backdrop_brush()).clone()));
    show(&s, &sprite);
    log.clear();
    // A change of 10 x 10 pixels under the backdrop.
    under.set_background(Some(crate::media::Brushes::blue()));
    s.run_jobs();
    let entries = log.entries();
    // The clip of the dirty area covers the whole backdrop.
    let covers = entries.iter().filter_map(|entry| entry.strip_prefix("PushClip ")).any(|rect| {
        let values: Vec<f64> = rect.split(", ").filter_map(|value| value.parse().ok()).collect();
        values.len() == 4 && values[0] <= 0.0 && values[1] <= 0.0 && values[0] + values[2] >= 200.0 && values[1] + values[3] >= 100.0
    });
    assert!(covers, "the whole backdrop is drawn again: {entries:?}");
}

#[test]
fn the_host_backdrop_brush_clears_its_area_in_a_transparent_window() {
    let s = CompositorCanvas::new();
    let brush = (*s.compositor.create_host_backdrop_brush()).clone();
    let sprite = sprite_with(&s, &brush);
    show(&s, &sprite);
    let entries = render_with_layers(&s, &sprite);
    assert!(!entries.iter().any(|entry| entry.starts_with("PushBlendLayer")), "{entries:?}");
}

#[test]
fn composition_brushes_are_the_brushes_of_elements() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let factory = s.compositor.create_effect_factory(ColorSourceEffect { name: String::new(), color: Colors::GREEN });
    let brush: Rc<dyn IBrush> = factory.create_brush().brush().clone();
    let element = TestBorder::new();
    element.set_bounds(crate::Rect::new(0.0, 0.0, 30.0, 20.0));
    element.set_background(Some(brush));
    log.clear();
    s.canvas.add(&element);
    s.run_jobs();
    let entries = log.entries();
    // Painted inside a clip of the rectangle the element fills.
    let clip = position_starting(&entries, "PushRoundedClip 0, 0, 30, 20");
    let green = position_starting(&entries, "DrawRectangle Green none 0, 0, 30, 20");
    assert!(clip < green, "{entries:?}");
}

// --- XAML composition brushes and AcrylicBrush -----------------------------------------

#[test]
fn a_xaml_brush_without_a_composition_brush_fills_its_fallback_colour() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let brush = AcrylicBrush::new();
    brush.set_fallback_color(Colors::ORANGE);
    brush.set_always_use_fallback(true);
    let element = TestBorder::new();
    element.set_bounds(crate::Rect::new(0.0, 0.0, 30.0, 20.0));
    element.set_background(Some(brush.clone().into()));
    s.canvas.add(&element);
    s.run_jobs();
    assert!(brush.is_connected());
    assert!(!brush.is_using_acrylic_brush());
    assert!(log.count("DrawRectangle Orange") > 0, "{:?}", log.entries());

    // Without a compositor (the immediate renderer) it is its fallback colour too.
    let xaml: Rc<dyn IBrush> = brush.clone().into();
    assert!(XamlCompositionBrushBase::of(&*xaml).is_some());
}

#[test]
fn acrylic_draws_its_recipe_and_falls_back_with_the_policy() {
    let s = CompositorCanvas::new();
    let brush = AcrylicBrush::new();
    brush.set_tint_color(Color::from_rgb(0x2c, 0x2c, 0x2c));
    brush.set_tint_opacity(0.15);
    brush.set_tint_luminosity_opacity(Some(0.96));
    brush.set_fallback_color(Color::from_rgb(0x2c, 0x2c, 0x2c));
    let element = TestBorder::new();
    element.set_bounds(crate::Rect::new(0.0, 0.0, 100.0, 50.0));
    element.set_background(Some(brush.clone().into()));
    s.canvas.add(&element);
    s.run_jobs();
    assert!(brush.is_connected());
    assert!(brush.is_using_acrylic_brush());
    let composition_brush = brush.composition_brush().expect("the material");
    let effect = CompositionEffectBrush::from_brush(&composition_brush).expect("an effect brush");
    assert_eq!(
        vec!["Backdrop".to_owned(), "Noise".to_owned()],
        {
            let mut names = effect.source_parameter_names();
            names.sort();
            names
        }
    );
    // The recipe: the blur of the backdrop, the luminosity and the colour blends.
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.log_effects = true;
    context.supports_layer_effects = true;
    s.server_object(element.composition_visual().unwrap().server()).render(
        &mut context,
        LtrbRect::INFINITE,
        None,
        true,
        false,
        false,
    );
    let entries = log.entries();
    position_starting(&entries, "PushEffect");
    position_starting(&entries, "PushBlendLayer 0, 0, 100, 50 Luminosity");
    position_starting(&entries, "PushBlendLayer 0, 0, 100, 50 Color");

    // The policy disables materials: a cross-fade, then the fallback colour.
    MaterialPolicy::set_simulate_disabled_by_policy(true);
    assert!(!brush.is_using_acrylic_brush());
    assert!(brush.is_waiting_for_fallback_animation());
    run_for(&s, Duration::from_millis(400));
    assert!(!brush.is_waiting_for_fallback_animation());
    let fallback = brush.composition_brush().expect("the fallback colour");
    assert!(CompositionEffectBrush::from_brush(&fallback).is_none());
    MaterialPolicy::set_simulate_disabled_by_policy(false);
    run_for(&s, Duration::from_millis(400));
    assert!(brush.is_using_acrylic_brush());

    // Leaving the tree disconnects the brush.
    s.canvas.remove(&element);
    s.run_jobs();
    assert!(!brush.is_connected());
    assert!(brush.composition_brush().is_none());
}

#[test]
fn an_opaque_tint_draws_no_backdrop() {
    let s = CompositorCanvas::new();
    let brush = AcrylicBrush::new();
    brush.set_tint_color(Colors::RED);
    brush.set_tint_opacity(1.0);
    brush.set_tint_luminosity_opacity(Some(1.0));
    let element = TestBorder::new();
    element.set_bounds(crate::Rect::new(0.0, 0.0, 10.0, 10.0));
    element.set_background(Some(brush.clone().into()));
    s.canvas.add(&element);
    s.run_jobs();
    let effect = CompositionEffectBrush::from_brush(&brush.composition_brush().unwrap()).unwrap();
    assert_eq!(vec!["Noise".to_owned()], effect.source_parameter_names());
}

// --- XAML lights -----------------------------------------------------------------------

#[test]
fn xaml_lights_connect_to_the_element_that_holds_them() {
    let s = CompositorCanvas::new();
    let host = TestBorder::filled(0.0, 0.0, 100.0, 100.0);
    let light: Ref<XamlLight> = instantiate(XamlLight::construct());
    host.lights().add(light.clone());
    assert!(!light.is_connected());
    s.canvas.add(&host);
    s.run_jobs();
    assert!(light.is_connected());

    // A composition light set while connected is bound to the element.
    let point = s.compositor.create_point_light();
    light.set_composition_light(Some((*point).clone()));
    assert_eq!(Some(String::new()), point.xaml_id());
    assert!(point.coordinate_space().is_some());

    // An element of the subtree that targets the id is a target.
    let target = TestBorder::filled(0.0, 0.0, 10.0, 10.0);
    host.add(&target);
    XamlLight::add_target_element("", &target);
    assert_eq!(1, point.targets().count());

    s.canvas.remove(&host);
    s.run_jobs();
    assert!(!light.is_connected());
    assert_eq!(None, point.xaml_id());
    assert_eq!(0, point.targets().count());
}

#[test]
fn xaml_brushes_record_the_lights_that_target_them() {
    let brush = AcrylicBrush::new();
    let handle: Rc<dyn IBrush> = brush.clone().into();
    XamlLight::add_target_brush("light", &handle);
    XamlLight::add_target_brush("light", &handle);
    assert_eq!(vec!["light".to_owned()], brush.light_target_ids());
    XamlLight::remove_target_brush("light", &handle);
    assert!(brush.light_target_ids().is_empty());
}

// --- system backdrops ------------------------------------------------------------------

struct MockTarget {
    compositor: Rc<Compositor>,
    support: Cell<SystemBackdropSupport>,
    brush: RefCell<Option<Rc<CompositionBrush>>>,
    platform: RefCell<Option<PlatformSystemBackdrop>>,
}

impl ICompositionSupportsSystemBackdrop for MockTarget {
    fn system_backdrop(&self) -> Option<Rc<CompositionBrush>> {
        self.brush.borrow().clone()
    }

    fn set_system_backdrop(&self, brush: Option<Rc<CompositionBrush>>) {
        *self.brush.borrow_mut() = brush;
    }

    fn try_set_platform_system_backdrop(&self, backdrop: Option<&PlatformSystemBackdrop>) -> SystemBackdropSupport {
        *self.platform.borrow_mut() = backdrop.copied();
        if backdrop.is_some() {
            self.support.get()
        } else {
            SystemBackdropSupport::None
        }
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.compositor.clone())
    }
}

fn mock_target(s: &CompositorCanvas, support: SystemBackdropSupport) -> Rc<MockTarget> {
    Rc::new(MockTarget {
        compositor: s.compositor.clone(),
        support: Cell::new(support),
        brush: RefCell::new(None),
        platform: RefCell::new(None),
    })
}

#[test]
fn a_mica_controller_follows_its_configuration() {
    let s = CompositorCanvas::new();
    let target = mock_target(&s, SystemBackdropSupport::Native);
    let as_target: Rc<dyn ICompositionSupportsSystemBackdrop> = target.clone();
    let controller = MicaController::new();
    assert_eq!(SystemBackdropState::Hidden, controller.state());
    let states = Rc::new(RefCell::new(Vec::new()));
    let recorded = states.clone();
    let _subscription = controller.state_changed(move |state| recorded.borrow_mut().push(state));

    let configuration = SystemBackdropConfiguration::new();
    configuration.set_is_input_active(true);
    configuration.set_theme(SystemBackdropTheme::Dark);
    controller.set_system_backdrop_configuration(Some(configuration.clone()));
    assert!(controller.add_system_backdrop_target(as_target.clone()));
    assert!(!controller.add_system_backdrop_target(as_target.clone()));
    // The platform draws it: no brush in the window.
    assert_eq!(SystemBackdropState::Active, controller.state());
    assert!(target.brush.borrow().is_none());
    let platform = target.platform.borrow().unwrap();
    assert_eq!(PlatformSystemBackdropKind::Mica(MicaKind::Base), platform.kind);
    assert_eq!(SystemBackdropTheme::Dark, platform.theme);
    assert_eq!(Color::from_rgb(0x20, 0x20, 0x20), platform.tint_color);

    controller.set_kind(MicaKind::BaseAlt);
    assert_eq!(PlatformSystemBackdropKind::Mica(MicaKind::BaseAlt), target.platform.borrow().unwrap().kind);

    // An inactive window shows the fallback colour.
    configuration.set_is_input_active(false);
    assert_eq!(SystemBackdropState::Fallback, controller.state());
    assert!(target.brush.borrow().is_some());
    assert!(target.platform.borrow().is_none());

    // High contrast shows its background colour.
    configuration.set_is_input_active(true);
    configuration.set_is_high_contrast(true);
    configuration.set_high_contrast_background_color(Some(Colors::BLACK));
    assert_eq!(SystemBackdropState::Fallback, controller.state());

    // The colours set by the application, and back to the defaults.
    configuration.set_is_high_contrast(false);
    controller.set_tint_color(Colors::RED);
    assert_eq!(Colors::RED, target.platform.borrow().unwrap().tint_color);
    controller.reset_properties();
    assert_eq!(Color::from_rgb(0x0a, 0x0a, 0x0a), controller.tint_color());

    // The policy (energy saver, transparency effects off).
    MaterialPolicy::set_simulate_disabled_by_policy(true);
    assert_eq!(SystemBackdropState::Fallback, controller.state());
    MaterialPolicy::set_simulate_disabled_by_policy(false);
    assert_eq!(SystemBackdropState::Active, controller.state());

    controller.close();
    assert_eq!(SystemBackdropState::Hidden, controller.state());
    assert!(target.brush.borrow().is_none());
    assert!(controller.is_closed());
    assert!(states.borrow().contains(&SystemBackdropState::Fallback));
}

#[test]
fn desktop_acrylic_without_platform_support_falls_back_and_over_a_blur_draws_its_tint() {
    let s = CompositorCanvas::new();
    let unsupported = mock_target(&s, SystemBackdropSupport::None);
    let controller = DesktopAcrylicController::new();
    controller.add_system_backdrop_target(unsupported.clone());
    assert_eq!(SystemBackdropState::Fallback, controller.state());
    assert!(unsupported.brush.borrow().is_some());

    let blur = mock_target(&s, SystemBackdropSupport::BlurBehind);
    let over_blur = DesktopAcrylicController::new();
    over_blur.add_system_backdrop_target(blur.clone());
    assert_eq!(SystemBackdropState::Active, over_blur.state());
    let brush = blur.brush.borrow().clone().expect("the tint over the blur");
    assert!(CompositionEffectBrush::from_brush(&brush).is_some());
}
