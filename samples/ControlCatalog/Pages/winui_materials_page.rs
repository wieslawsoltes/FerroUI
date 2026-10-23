//! The page of the WinUI materials (not from the managed original; `docs/porting/winui-materials.md`):
//! `AcrylicBrush` over a busy scene, the blend and colour effects the materials are built from,
//! the Reveal brushes and lights, the system backdrops of the window (`MicaBackdrop`,
//! `DesktopAcrylicBackdrop`) and the menus whose background is the desktop acrylic backdrop.

use super::winui_composition_page::{brush, compositor_of, section, text};
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{AcrylicBrush, Color, ColorMatrix, FontWeight, MaterialPolicy};
use ferroui_base::platform::{AlphaFormat, PixelFormats};
use ferroui_base::rendering::composition::effects::{
    ArithmeticCompositeEffect, BlendEffect, BlendEffectMode, CanvasComposite, ColorMatrixEffect, CompositeEffect,
    GraphicsEffect, GrayscaleEffect, InvertEffect, SaturationEffect, SceneLightingEffect, ShadowEffect, TintEffect,
};
use ferroui_base::rendering::composition::system_backdrops::{ICompositionSupportsSystemBackdrop, MicaKind};
use ferroui_base::rendering::composition::{
    CompositionBrush, CompositionEffectSourceParameter, Compositor, ICompositionSurface, LoadedImageSurface,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{BoxedValue, CornerRadius, PixelSize, Ref, Thickness, Vector, WeakRef};
use ferroui_controls::flyouts::MenuFlyout;
use ferroui_controls::hosting::ElementCompositionPreview;
use ferroui_controls::media::{
    DesktopAcrylicBackdrop, MicaBackdrop, RevealBackgroundBrush, RevealBorderBrush, RevealBrush, RevealBrushState,
    SystemBackdrop,
};
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::{
    Border, Button, Canvas, ContentPage, Control, MenuItem, Panel, ScrollViewer, Slider, StackPanel, TextBlock, TopLevel,
    WrapPanel,
};
use std::cell::RefCell;
use std::rc::Rc;

fn label(value: &str) -> BoxedValue {
    Rc::new(value.to_owned()) as BoxedValue
}

fn caption(value: &str) -> Ref<TextBlock> {
    let block = text(value, 12.0, FontWeight::Normal);
    block.set_opacity(0.75);
    block
}

fn row(spacing: f64) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Horizontal);
    panel.set_spacing(spacing);
    panel
}

fn column(spacing: f64) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(spacing);
    panel
}

fn button(content: &str, click: impl Fn() + 'static) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(label(content)));
    button.click(move |_, _| click());
    button
}

// --- AcrylicBrush --------------------------------------------------------------------------

/// A busy scene for the acrylic to blur: coloured discs and stripes on a dark ground.
fn scene(width: f64, height: f64) -> Ref<Canvas> {
    let canvas = Canvas::new();
    canvas.set_width(width);
    canvas.set_height(height);
    canvas.set_background(Some(brush(Color::from_rgb(0x1B, 0x1F, 0x2A))));
    let shapes: [(f64, f64, f64, (u8, u8, u8)); 7] = [
        (20.0, 10.0, 120.0, (0x2E, 0x86, 0xDE)),
        (150.0, 60.0, 90.0, (0xE8, 0x5D, 0x3A)),
        (240.0, -20.0, 110.0, (0x3F, 0x9E, 0x5A)),
        (360.0, 70.0, 130.0, (0xD9, 0xA4, 0x41)),
        (470.0, 0.0, 80.0, (0x9B, 0x59, 0xB6)),
        (540.0, 90.0, 100.0, (0x1A, 0xA6, 0xA6)),
        (80.0, 120.0, 70.0, (0xF5, 0xF5, 0xF5)),
    ];
    for (x, y, size, (r, g, b)) in shapes {
        let disc = Ellipse::new();
        disc.set_width(size);
        disc.set_height(size);
        disc.set_fill(Some(brush(Color::from_rgb(r, g, b))));
        Canvas::set_left(&disc, x);
        Canvas::set_top(&disc, y);
        canvas.children().add(disc.upcast());
    }
    for index in 0..8 {
        let stripe = Border::new();
        stripe.set_width(6.0);
        stripe.set_height(height);
        stripe.set_background(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
        stripe.set_opacity(0.5);
        Canvas::set_left(&stripe, 30.0 + index as f64 * 80.0);
        canvas.children().add(stripe.upcast());
    }
    let words = text("FerroUI materials", 34.0, FontWeight::Bold);
    words.set_foreground(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
    Canvas::set_left(&words, 170.0);
    Canvas::set_top(&words, 20.0);
    canvas.children().add(words.upcast());
    canvas
}

/// A card whose background is `acrylic`, with its name on it.
fn acrylic_card(acrylic: &Ref<AcrylicBrush>, name: &str, dark_text: bool) -> Ref<Border> {
    let card = Border::new();
    card.set_width(140.0);
    card.set_height(120.0);
    card.set_corner_radius(CornerRadius::uniform(8.0));
    card.set_background(Some(acrylic.clone().into()));
    card.set_border_brush(Some(brush(Color::from_argb(0x40, 0xFF, 0xFF, 0xFF))));
    card.set_border_thickness(Thickness::uniform(1.0));
    let name = text(name, 13.0, FontWeight::SemiBold);
    name.set_margin(Thickness::uniform(10.0));
    let foreground = if dark_text { Color::from_rgb(0x1A, 0x1A, 0x1A) } else { Color::from_rgb(0xFF, 0xFF, 0xFF) };
    name.set_foreground(Some(brush(foreground)));
    card.set_child(name);
    card
}

fn acrylic(tint: Color, tint_opacity: f64, luminosity: Option<f64>, fallback: Color) -> Ref<AcrylicBrush> {
    let acrylic = AcrylicBrush::new();
    acrylic.set_tint_color(tint);
    acrylic.set_tint_opacity(tint_opacity);
    acrylic.set_tint_luminosity_opacity(luminosity);
    acrylic.set_fallback_color(fallback);
    acrylic
}

fn acrylic_section() -> Ref<StackPanel> {
    let light = acrylic(Color::from_rgb(0xFC, 0xFC, 0xFC), 0.0, Some(0.85), Color::from_rgb(0xF9, 0xF9, 0xF9));
    let dark = acrylic(Color::from_rgb(0x2C, 0x2C, 0x2C), 0.15, Some(0.96), Color::from_rgb(0x2C, 0x2C, 0x2C));
    let accent = acrylic(Color::from_rgb(0x00, 0x5F, 0xB8), 0.8, Some(0.8), Color::from_rgb(0x00, 0x5F, 0xB8));
    let tinted = acrylic(Color::from_rgb(0xE8, 0x5D, 0x3A), 0.4, None, Color::from_rgb(0xE8, 0x5D, 0x3A));
    let fallback = acrylic(Color::from_rgb(0x2C, 0x2C, 0x2C), 0.15, Some(0.96), Color::from_rgb(0x2C, 0x2C, 0x2C));
    fallback.set_always_use_fallback(true);
    let host = acrylic(Color::from_rgb(0x2C, 0x2C, 0x2C), 0.3, Some(0.6), Color::from_rgb(0x40, 0x40, 0x40));
    host.set_background_source(ferroui_base::media::AcrylicBackgroundSource::HostBackdrop);

    let cards = row(16.0);
    cards.set_margin(Thickness::new(16.0, 36.0, 16.0, 16.0));
    cards.children().add(acrylic_card(&light, "Light: tint 0, luminosity 0.85", true).upcast());
    cards.children().add(acrylic_card(&dark, "Dark: tint 0.15, luminosity 0.96", false).upcast());
    cards.children().add(acrylic_card(&accent, "Accent: tint 0.8", false).upcast());
    cards.children().add(acrylic_card(&tinted, "Tint 0.4, luminosity from the tint", false).upcast());
    cards.children().add(acrylic_card(&fallback, "AlwaysUseFallback", false).upcast());
    cards.children().add(acrylic_card(&host, "BackgroundSource HostBackdrop", false).upcast());

    let stage = Panel::new();
    stage.set_horizontal_alignment(HorizontalAlignment::Left);
    stage.set_clip_to_bounds(true);
    stage.children().add(scene(940.0, 190.0).upcast());
    stage.children().add(cards.upcast());

    let controls = row(24.0);
    let tint_title = caption("TintOpacity of the tinted card: 0.40");
    let tint_slider = Slider::new();
    tint_slider.set_minimum(0.0);
    tint_slider.set_maximum(1.0);
    tint_slider.set_range_value(0.4);
    tint_slider.set_width(200.0);
    {
        let tinted = tinted.clone();
        let title = tint_title.downgrade();
        let _ = tint_slider.value_changed(move |_, e| {
            tinted.set_tint_opacity(e.new_value());
            if let Some(title) = title.upgrade() {
                title.set_text(Some(&format!("TintOpacity of the tinted card: {:.2}", e.new_value())));
            }
        });
    }
    let tint_panel = column(2.0);
    tint_panel.children().add(tint_title.upcast());
    tint_panel.children().add(tint_slider.upcast());
    controls.children().add(tint_panel.upcast());

    let policy = caption("");
    let update_policy: Rc<dyn Fn()> = {
        let policy = policy.downgrade();
        Rc::new(move || {
            if let Some(policy) = policy.upgrade() {
                let settings = MaterialPolicy::platform_settings();
                policy.set_text(Some(&format!(
                    "Material policy: {} (transparency effects {}, energy saver {}, simulated {})",
                    if MaterialPolicy::is_disabled_by_policy() { "fallback" } else { "materials" },
                    if settings.transparency_effects_enabled { "on" } else { "off" },
                    if settings.energy_saver_on { "on" } else { "off" },
                    if MaterialPolicy::simulate_disabled_by_policy() { "yes" } else { "no" },
                )));
            }
        })
    };
    update_policy();
    let simulate = {
        let update_policy = update_policy.clone();
        button("Simulate the policy", move || {
            MaterialPolicy::set_simulate_disabled_by_policy(!MaterialPolicy::simulate_disabled_by_policy());
            update_policy();
        })
    };
    controls.children().add(simulate.upcast());
    let tint_color = {
        let tinted = tinted.clone();
        let index = RefCell::new(0usize);
        button("Next tint colour", move || {
            const TINTS: [Color; 4] = [
                Color::from_rgb(0x3F, 0x9E, 0x5A),
                Color::from_rgb(0x9B, 0x59, 0xB6),
                Color::from_rgb(0xD9, 0xA4, 0x41),
                Color::from_rgb(0xE8, 0x5D, 0x3A),
            ];
            let mut index = index.borrow_mut();
            tinted.set_tint_color(TINTS[*index % TINTS.len()]);
            *index += 1;
        })
    };
    controls.children().add(tint_color.upcast());

    let content = column(10.0);
    content.children().add(stage.upcast());
    content.children().add(controls.upcast());
    content.children().add(policy.upcast());
    section(
        "AcrylicBrush",
        "The acrylic recipe of WinUI over what is drawn behind each card: the backdrop over the opaque fallback colour, blurred by 30, its luminosity from TintLuminosityOpacity (or from the tint), its colour from TintColor and TintOpacity, and 2 % noise. TintColor changes cross-fade over TintTransitionDuration; the material policy (transparency effects off, energy saver, the simulation) cross-fades to the FallbackColor. The HostBackdrop card (WinUI 2's window acrylic) shows what is behind the window where the window is transparent with a blur (Settings, transparency AcrylicBlur), its fallback colour in an opaque window and while no window of the application is active.",
        &content.upcast(),
    )
}

// --- effects -------------------------------------------------------------------------------

/// A gradient image of `width` x `height` pixels: hue across, light down.
fn gradient_bitmap(width: i32, height: i32) -> Bitmap {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let t = x as f32 / width as f32;
            let v = 1.0 - 0.6 * y as f32 / height as f32;
            let (r, g, b) = hue(t);
            data.extend_from_slice(&[(r * v * 255.0) as u8, (g * v * 255.0) as u8, (b * v * 255.0) as u8, 255]);
        }
    }
    Bitmap::from_pixels(
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
        &data,
        PixelSize::new(width, height),
        Vector::new(96.0, 96.0),
        width * 4,
    )
}

fn hue(t: f32) -> (f32, f32, f32) {
    let h = t * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    match h as i32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    }
}

/// Rings of grey for the foreground of the blends.
fn rings_bitmap(size: i32) -> Bitmap {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let center = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let distance = ((x as f32 - center).powi(2) + (y as f32 - center).powi(2)).sqrt();
            let level = (((distance / 6.0).sin() * 0.5 + 0.5) * 255.0) as u8;
            data.extend_from_slice(&[level, level, level, 255]);
        }
    }
    Bitmap::from_pixels(
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
        &data,
        PixelSize::new(size, size),
        Vector::new(96.0, 96.0),
        size * 4,
    )
}

fn surface_brush(compositor: &Rc<Compositor>, image: &Bitmap) -> Rc<CompositionBrush> {
    let surface: Rc<dyn ICompositionSurface> = LoadedImageSurface::from_bitmap(compositor, image);
    let brush = compositor.create_surface_brush_with_surface(surface);
    brush.set_stretch(ferroui_base::rendering::composition::CompositionStretch::Fill);
    (*brush).clone()
}

fn parameter(name: &str) -> Option<ferroui_base::rendering::composition::effects::GraphicsEffectSource> {
    Some(CompositionEffectSourceParameter::new(name).into())
}

/// A tile that shows `effect` over the gradient (`image`) and the rings (`rings`).
fn effect_tile(name: &str, effect: GraphicsEffect) -> Ref<StackPanel> {
    let host = Border::new();
    host.set_width(150.0);
    host.set_height(100.0);
    host.set_corner_radius(CornerRadius::uniform(8.0));
    host.set_clip_to_bounds(true);
    host.set_horizontal_alignment(HorizontalAlignment::Left);
    host.set_background(Some(brush(Color::from_rgb(0x20, 0x24, 0x30))));
    let weak = host.downgrade();
    let effect = Rc::new(effect);
    let _ = host.attached_to_visual_tree(move |_| {
        let (weak, effect) = (weak.clone(), effect.clone());
        let _ = Dispatcher::ui_thread().invoke_async_local_with_priority(
            move || {
                let Some(host) = weak.upgrade() else { return };
                let Some(compositor) = compositor_of(&host) else { return };
                let factory = compositor.create_effect_factory((*effect).clone());
                let effect_brush = factory.create_brush();
                effect_brush.set_source_parameter("image", Some(surface_brush(&compositor, &gradient_bitmap(150, 100))));
                effect_brush.set_source_parameter("rings", Some(surface_brush(&compositor, &rings_bitmap(100))));
                let sprite = compositor.create_sprite_visual();
                sprite.set_size(Vector::new(150.0, 100.0));
                sprite.set_brush(Some((*effect_brush).clone()));
                ElementCompositionPreview::set_element_child_visual(&host, Some(sprite.visual().clone()));
            },
            DispatcherPriority::BACKGROUND,
        );
    });
    let tile = column(6.0);
    tile.set_margin(Thickness::new(0.0, 0.0, 12.0, 12.0));
    tile.children().add(host.upcast());
    let about = caption(name);
    about.set_max_width(150.0);
    about.set_horizontal_alignment(HorizontalAlignment::Left);
    tile.children().add(about.upcast());
    tile
}

fn blend(mode: BlendEffectMode) -> GraphicsEffect {
    BlendEffect { name: "Blend".into(), mode, background: parameter("image"), foreground: parameter("rings") }.into()
}

fn effects_section() -> Ref<StackPanel> {
    let grid = WrapPanel::new();
    grid.set_orientation(Orientation::Horizontal);
    let tiles: Vec<(&str, GraphicsEffect)> = vec![
        ("BlendEffect Multiply", blend(BlendEffectMode::Multiply)),
        ("BlendEffect Screen", blend(BlendEffectMode::Screen)),
        ("BlendEffect Overlay", blend(BlendEffectMode::Overlay)),
        ("BlendEffect Difference", blend(BlendEffectMode::Difference)),
        ("BlendEffect Luminosity (acrylic)", blend(BlendEffectMode::Luminosity)),
        ("BlendEffect Color (acrylic)", blend(BlendEffectMode::Color)),
        (
            "SaturationEffect 0.2",
            SaturationEffect { name: "Saturation".into(), saturation: 0.2, source: parameter("image") }.into(),
        ),
        ("GrayscaleEffect", GrayscaleEffect { name: "Grayscale".into(), source: parameter("image") }.into()),
        ("InvertEffect", InvertEffect { name: "Invert".into(), source: parameter("image") }.into()),
        (
            "TintEffect (teal)",
            TintEffect {
                name: "Tint".into(),
                color: Color::from_rgb(0x1A, 0xA6, 0xA6),
                clamp_output: false,
                source: parameter("image"),
            }
            .into(),
        ),
        ("BlendEffect PinLight", blend(BlendEffectMode::PinLight)),
        ("BlendEffect VividLight", blend(BlendEffectMode::VividLight)),
        ("BlendEffect Subtract", blend(BlendEffectMode::Subtract)),
        ("BlendEffect Dissolve", blend(BlendEffectMode::Dissolve)),
        (
            "ArithmeticCompositeEffect of two images",
            ArithmeticCompositeEffect {
                name: "Arithmetic".into(),
                multiply_amount: 1.0,
                source1_amount: 0.3,
                source2_amount: 0.3,
                offset: 0.0,
                source1: parameter("image"),
                source2: parameter("rings"),
            }
            .into(),
        ),
        (
            "CompositeEffect MaskInvert",
            CompositeEffect {
                name: String::new(),
                mode: CanvasComposite::MaskInvert,
                sources: vec![
                    CompositionEffectSourceParameter::new("image").into(),
                    CompositionEffectSourceParameter::new("rings").into(),
                ],
            }
            .into(),
        ),
        (
            "ShadowEffect (blur 4, teal)",
            ShadowEffect {
                name: "Shadow".into(),
                blur_amount: 4.0,
                shadow_color: Color::from_rgb(0x1A, 0xA6, 0xA6),
                source: parameter("rings"),
                ..Default::default()
            }
            .into(),
        ),
        (
            "ColorMatrixEffect (sepia)",
            ColorMatrixEffect {
                name: "Sepia".into(),
                color_matrix: ColorMatrix::new([
                    [0.393, 0.349, 0.272, 0.0],
                    [0.769, 0.686, 0.534, 0.0],
                    [0.189, 0.168, 0.131, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                    [0.0, 0.0, 0.0, 0.0],
                ]),
                source: parameter("image"),
                ..Default::default()
            }
            .into(),
        ),
    ];
    for (name, effect) in tiles {
        grid.children().add(effect_tile(name, effect).upcast());
    }
    section(
        "Blend and colour effects",
        "The effects of the materials of WinUI in the effect graph of the compositor: BlendEffect (the 25 modes of Win2D; the acrylic uses Luminosity and Color), SaturationEffect, GrayscaleEffect, InvertEffect, TintEffect and ColorMatrixEffect, drawn as blend layers and colour matrix layers of Skia and Vello.",
        &grid.upcast(),
    )
}

// --- lighting ------------------------------------------------------------------------------

/// A host of composition visuals, `width` by `height`, with a caption.
fn light_tile(name: &str, width: f64, height: f64, build: impl Fn(&Ref<Border>, &Rc<Compositor>) + 'static) -> Ref<StackPanel> {
    let host = Border::new();
    host.set_width(width);
    host.set_height(height);
    host.set_corner_radius(CornerRadius::uniform(8.0));
    host.set_clip_to_bounds(true);
    host.set_horizontal_alignment(HorizontalAlignment::Left);
    host.set_background(Some(brush(Color::from_rgb(0x10, 0x12, 0x18))));
    let weak = host.downgrade();
    let build = Rc::new(build);
    let _ = host.attached_to_visual_tree(move |_| {
        let (weak, build) = (weak.clone(), build.clone());
        let _ = Dispatcher::ui_thread().invoke_async_local_with_priority(
            move || {
                let Some(host) = weak.upgrade() else { return };
                let Some(compositor) = compositor_of(&host) else { return };
                build(&host, &compositor);
            },
            DispatcherPriority::BACKGROUND,
        );
    });
    let tile = column(6.0);
    tile.set_margin(Thickness::new(0.0, 0.0, 16.0, 12.0));
    tile.children().add(host.upcast());
    let about = caption(name);
    about.set_max_width(width);
    about.set_horizontal_alignment(HorizontalAlignment::Left);
    tile.children().add(about.upcast());
    tile
}

fn lighting_section() -> Ref<StackPanel> {
    let grid = WrapPanel::new();
    grid.set_orientation(Orientation::Horizontal);

    // A coloured light brighter than full on coloured content: each channel
    // of the content times the light, brightened where the light is above 1.
    grid.children().add(
        light_tile("PointLight (orange, intensity 1.8) following the pointer over a gradient", 300.0, 180.0, |host, compositor| {
            let container = compositor.create_container_visual();
            container.set_size(Vector::new(300.0, 180.0));
            let sprite = compositor.create_sprite_visual();
            sprite.set_size(Vector::new(300.0, 180.0));
            sprite.set_brush(Some(surface_brush(compositor, &gradient_bitmap(300, 180))));
            container.children().insert_at_top((*sprite).clone());
            let ambient = compositor.create_ambient_light();
            ambient.set_intensity(0.25);
            ambient.targets().add((*sprite).clone());
            let light = compositor.create_point_light();
            light.set_color(Color::from_rgb(0xFF, 0xB0, 0x60));
            light.set_intensity(1.8);
            light.set_constant_attenuation(1.0);
            light.set_offset(ferroui_base::Vector3D::new(150.0, 90.0, 80.0));
            light.set_coordinate_space(Some(container.clone()));
            light.targets().add((*sprite).clone());
            if let Some(pointer) = ElementCompositionPreview::get_pointer_position_property_set(host) {
                let follow = compositor.create_expression_animation_with("Vector3(pointer.Position.X, pointer.Position.Y, 80)");
                follow.set_reference_parameter("pointer", pointer);
                light.start_animation("Offset", &*follow);
            }
            ElementCompositionPreview::set_element_child_visual(host, Some(container.visual().clone()));
        })
        .upcast(),
    );

    // A spot light whose coordinate space is a visual that moves: the light
    // moves with it and lights its target again.
    grid.children().add(
        light_tile("SpotLight in the coordinate space of a moving visual", 300.0, 180.0, |host, compositor| {
            let container = compositor.create_container_visual();
            container.set_size(Vector::new(300.0, 180.0));
            let sprite = compositor.create_sprite_visual();
            sprite.set_size(Vector::new(300.0, 180.0));
            sprite.set_brush(Some((*compositor.create_color_brush_with_color(Color::from_rgb(0xE0, 0xE4, 0xF0))).clone()));
            container.children().insert_at_top((*sprite).clone());
            let space = compositor.create_container_visual();
            space.set_size(Vector::new(1.0, 1.0));
            container.children().insert_at_top((*space).clone());
            let light = compositor.create_spot_light();
            light.set_offset(ferroui_base::Vector3D::new(0.0, 90.0, 120.0));
            light.set_outer_cone_angle_in_degrees(30.0);
            light.set_inner_cone_angle_in_degrees(15.0);
            light.set_coordinate_space(Some(space.clone()));
            light.targets().add((*sprite).clone());
            let travel = compositor.create_scalar_key_frame_animation();
            travel.insert_key_frame(0.0, 20.0);
            travel.insert_key_frame(1.0, 280.0);
            travel.set_duration(std::time::Duration::from_millis(2400));
            travel.set_iteration_behavior(ferroui_base::rendering::composition::animations::AnimationIterationBehavior::Forever);
            travel.set_direction(ferroui_base::animation::PlaybackDirection::Alternate);
            space.start_animation("Offset.X", &*travel);
            ElementCompositionPreview::set_element_child_visual(host, Some(container.visual().clone()));
        })
        .upcast(),
    );

    // The scene lighting effect with a specular highlight.
    grid.children().add(
        light_tile("SceneLightingEffect: diffuse 0.6, specular 1, shine 24", 300.0, 180.0, |host, compositor| {
            let factory = compositor.create_effect_factory(SceneLightingEffect {
                name: "Lighting".into(),
                ambient_amount: 0.1,
                diffuse_amount: 0.6,
                specular_amount: 1.0,
                specular_shine: 24.0,
                ..Default::default()
            });
            let lighting = factory.create_brush();
            let sprite = compositor.create_sprite_visual();
            sprite.set_size(Vector::new(300.0, 180.0));
            sprite.set_brush(Some(lighting.brush().clone()));
            let light = compositor.create_point_light();
            light.set_offset(ferroui_base::Vector3D::new(150.0, 90.0, 120.0));
            light.targets().add((*sprite).clone());
            if let Some(pointer) = ElementCompositionPreview::get_pointer_position_property_set(host) {
                let follow = compositor.create_expression_animation_with("Vector3(pointer.Position.X, pointer.Position.Y, 120)");
                follow.set_reference_parameter("pointer", pointer);
                light.start_animation("Offset", &*follow);
            }
            ElementCompositionPreview::set_element_child_visual(host, Some(sprite.visual().clone()));
        })
        .upcast(),
    );

    section(
        "Lights",
        "The content a light reaches is multiplied by the light, channel by channel, in a multiply layer of the drawing context: a coloured light tints coloured content exactly, a light above full brightens it, and the light map is sampled every second device pixel. A light in the coordinate space of a moving visual moves with it. The scene lighting effect adds a specular highlight.",
        &grid.upcast(),
    )
}

// --- Reveal --------------------------------------------------------------------------------

/// An element with a Reveal background and border whose Reveal state follows the pointer, as
/// the visual states of a WinUI control set it.
pub(super) fn reveal_tile(name: &str) -> Ref<Border> {
    let tile = Border::new();
    tile.set_width(150.0);
    tile.set_height(64.0);
    tile.set_corner_radius(CornerRadius::uniform(4.0));
    let background = RevealBackgroundBrush::new();
    background.set_color(Color::from_argb(0x33, 0xFF, 0xFF, 0xFF));
    background.set_fallback_color(Color::from_argb(0x33, 0xFF, 0xFF, 0xFF));
    background.set_target_theme(ferroui_controls::media::ApplicationTheme::Dark);
    tile.set_background(Some(background.into()));
    let border = RevealBorderBrush::new();
    border.set_color(Color::from_argb(0x66, 0xFF, 0xFF, 0xFF));
    border.set_fallback_color(Color::from_argb(0x33, 0xFF, 0xFF, 0xFF));
    border.set_target_theme(ferroui_controls::media::ApplicationTheme::Dark);
    tile.set_border_brush(Some(border.into()));
    tile.set_border_thickness(Thickness::uniform(2.0));
    let name = text(name, 13.0, FontWeight::SemiBold);
    name.set_foreground(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
    name.set_horizontal_alignment(HorizontalAlignment::Center);
    name.set_vertical_alignment(VerticalAlignment::Center);
    tile.set_child(name);

    let set = |tile: &Ref<Border>, state: RevealBrushState| RevealBrush::set_state(tile, state);
    let weak: WeakRef<Border> = tile.downgrade();
    {
        let weak = weak.clone();
        tile.add_handler(InputElement::pointer_entered_event(), move |_, _| {
            if let Some(tile) = weak.upgrade() {
                set(&tile, RevealBrushState::PointerOver);
            }
        });
    }
    {
        let weak = weak.clone();
        tile.add_handler(InputElement::pointer_exited_event(), move |_, _| {
            if let Some(tile) = weak.upgrade() {
                set(&tile, RevealBrushState::Normal);
            }
        });
    }
    {
        let weak = weak.clone();
        tile.add_handler(InputElement::pointer_pressed_event(), move |_, _| {
            if let Some(tile) = weak.upgrade() {
                set(&tile, RevealBrushState::Pressed);
            }
        });
    }
    tile.add_handler(InputElement::pointer_released_event(), move |_, _| {
        if let Some(tile) = weak.upgrade() {
            set(&tile, RevealBrushState::PointerOver);
        }
    });
    tile
}

fn reveal_section() -> Ref<StackPanel> {
    let surface = Border::new();
    surface.set_padding(Thickness::uniform(16.0));
    surface.set_corner_radius(CornerRadius::uniform(8.0));
    surface.set_horizontal_alignment(HorizontalAlignment::Left);
    surface.set_background(Some(brush(Color::from_rgb(0x1F, 0x1F, 0x1F))));
    let grid = WrapPanel::new();
    grid.set_orientation(Orientation::Horizontal);
    grid.set_max_width(680.0);
    for name in ["Home", "Documents", "Pictures", "Music", "Videos", "Downloads", "Settings", "Help"] {
        let tile = reveal_tile(name);
        tile.set_margin(Thickness::uniform(4.0));
        grid.children().add(tile.upcast());
    }
    surface.set_child(grid);
    let focus_kind = {
        let toggle = Button::new();
        let set_label = |toggle: &Ref<Button>| {
            let on = ferroui_controls::media::RevealFocus::focus_visual_kind() == ferroui_controls::media::FocusVisualKind::Reveal;
            toggle.set_content(Some(label(if on { "Reveal focus: on (Tab to see it)" } else { "Reveal focus: off" })));
        };
        set_label(&toggle);
        let weak = toggle.downgrade();
        toggle.click(move |_, _| {
            use ferroui_controls::media::{FocusVisualKind, RevealFocus};
            let next = if RevealFocus::focus_visual_kind() == FocusVisualKind::Reveal {
                FocusVisualKind::HighVisibility
            } else {
                FocusVisualKind::Reveal
            };
            RevealFocus::set_focus_visual_kind(next);
            if let Some(toggle) = weak.upgrade() {
                set_label(&toggle);
            }
        });
        toggle
    };
    let buttons = row(8.0);
    for (theme, name) in [("ButtonRevealStyle", "ButtonRevealStyle"), ("ToggleButtonRevealStyle", "ToggleButtonRevealStyle"), ("RepeatButtonRevealStyle", "RepeatButtonRevealStyle")] {
        let _ = theme;
        let button: Ref<Control> = match name {
            "ToggleButtonRevealStyle" => {
                let toggle = ferroui_controls::primitives::ToggleButton::new();
                toggle.set_content(Some(label(name)));
                toggle.upcast()
            }
            "RepeatButtonRevealStyle" => {
                let repeat = ferroui_controls::RepeatButton::new();
                repeat.set_content(Some(label(name)));
                repeat.upcast()
            }
            _ => button(name, || {}).upcast(),
        };
        apply_theme(&button, theme);
        buttons.children().add(button);
    }
    let content = column(10.0);
    content.children().add(surface.upcast());
    content.children().add(buttons.upcast());
    content.children().add(focus_kind.upcast());
    let surface = content;
    section(
        "Reveal",
        "RevealBackgroundBrush and RevealBorderBrush (TargetTheme Dark) with the Reveal states of WinUI set by the pointer: RevealBrush.State adds a RevealHoverLight and a RevealPressLight to the element (XamlLight, its Lights); the border lights of the window (RevealBorderLight, with the XamlAmbientLight) light the borders near the pointer through the scene lighting effect of the brush, across adjacent tiles. The buttons use ButtonRevealStyle, ToggleButtonRevealStyle and RepeatButtonRevealStyle of the WinUI theme. With Reveal focus (FocusVisualKind.Reveal), the element keyboard focus moves to glows, lit by a spot light that travels in the direction focus moved, a pulsing light and a press light (Space or Enter).",
        &surface.upcast(),
    )
}

// --- system backdrops ----------------------------------------------------------------------

ferroui_base::ferro_thread_local! {
    /// The function of the page last created that sets the backdrop of the window, for the
    /// smoke run of the desktop entry point.
    static SET_BACKDROP: RefCell<Option<Rc<dyn Fn(usize)>>> = const { RefCell::new(None) };
}

/// Sets the system backdrop of the window of the page last created, as its buttons do:
/// 0 none, 1 Mica, 2 Mica Alt, 3 desktop acrylic (for the smoke run). Returns whether the
/// page exists.
pub fn smoke_set_materials_backdrop(index: usize) -> bool {
    let set = SET_BACKDROP.with(|set| set.borrow().clone());
    match set {
        Some(set) => {
            set(index);
            true
        }
        None => false,
    }
}

fn backdrop_section() -> Ref<StackPanel> {
    let status = caption("The window has no system backdrop.");
    let anchor = row(8.0);
    let set: Rc<dyn Fn(usize)> = {
        let anchor = anchor.downgrade();
        let status = status.downgrade();
        Rc::new(move |index: usize| {
            let (Some(anchor), Some(status)) = (anchor.upgrade(), status.upgrade()) else { return };
            let Some(top_level) = TopLevel::get_top_level(Some(&anchor)) else { return };
            let backdrop: Option<Ref<SystemBackdrop>> = match index {
                1 => Some(MicaBackdrop::new().upcast()),
                2 => {
                    let mica = MicaBackdrop::new();
                    mica.set_kind(MicaKind::BaseAlt);
                    Some(mica.upcast())
                }
                3 => Some(DesktopAcrylicBackdrop::new().upcast()),
                _ => None,
            };
            top_level.set_system_backdrop(backdrop.clone());
            let text = match backdrop {
                None => "The window has no system backdrop.".to_owned(),
                Some(backdrop) => {
                    let target: Rc<dyn ICompositionSupportsSystemBackdrop> = top_level.system_backdrop_target();
                    let controller = backdrop
                        .downcast_ref::<MicaBackdrop>()
                        .and_then(|mica| mica.controller_of(&target))
                        .or_else(|| backdrop.downcast_ref::<DesktopAcrylicBackdrop>().and_then(|acrylic| acrylic.controller_of(&target)));
                    let state = controller.map_or_else(|| "no controller".to_owned(), |controller| format!("{:?}", controller.state()));
                    format!(
                        "{}: controller {state}; drawn by the platform: {}",
                        ["", "MicaBackdrop", "MicaBackdrop Kind=BaseAlt", "DesktopAcrylicBackdrop"][index.min(3)],
                        if top_level.system_backdrop_target().is_platform_active() { "yes" } else { "no (the fallback colour or in-app material)" },
                    )
                }
            };
            status.set_text(Some(&text));
        })
    };
    SET_BACKDROP.with(|slot| *slot.borrow_mut() = Some(set.clone()));
    for (index, name) in ["None", "Mica", "Mica Alt", "Desktop acrylic"].into_iter().enumerate() {
        let set = set.clone();
        anchor.children().add(button(name, move || set(index)).upcast());
    }
    let content = column(8.0);
    content.children().add(anchor.upcast());
    content.children().add(status.upcast());
    section(
        "System backdrops",
        "Window.SystemBackdrop with MicaBackdrop (Kind Base and BaseAlt) and DesktopAcrylicBackdrop: a MicaController or DesktopAcrylicController per window, configured from the theme of the window, high contrast and whether the window is active. macOS draws them with an NSVisualEffectView behind the window and Windows with the system backdrop types of DWM; an inactive window, high contrast, the energy saver or transparency effects turned off show the fallback colour.",
        &content.upcast(),
    )
}

// --- menus ---------------------------------------------------------------------------------

fn menu_section() -> Ref<StackPanel> {
    let flyout = MenuFlyout::new();
    for name in ["Open", "Save", "Save as", "Print", "Close"] {
        let item = MenuItem::new();
        item.set_header(Some(label(name)));
        flyout.items().add(Some(Control::boxed(item)));
    }
    let open = Button::new();
    open.set_content(Some(label("Open a menu flyout")));
    open.set_flyout(&flyout);

    let context = ferroui_controls::ContextMenu::new();
    for name in ["Cut", "Copy", "Paste"] {
        let item = MenuItem::new();
        item.set_header(Some(label(name)));
        context.items().add(Some(Control::boxed(item)));
    }
    let target = Border::new();
    target.set_padding(Thickness::uniform(12.0));
    target.set_corner_radius(CornerRadius::uniform(4.0));
    target.set_background(Some(brush(Color::from_argb(0x30, 0x80, 0x80, 0x80))));
    target.set_child(text("Right-click for a context menu", 13.0, FontWeight::Normal));
    target.set_context_menu(Some(context));

    let content = row(16.0);
    content.children().add(open.upcast());
    content.children().add(target.upcast());
    section(
        "Menus",
        "MenuFlyoutPresenter.SystemBackdrop is the theme resource MenuFlyoutSystemBackdrop, a DesktopAcrylicBackdrop: in a popup of the window the menu shows the acrylic of what is behind it; in a popup window of its own, the fallback colour of the acrylic. The context menu shares the property.",
        &content.upcast(),
    )
}

/// Creates the page.
pub fn winui_materials_page() -> Ref<ContentPage> {
    let page = ContentPage::new();
    page.set_header(Some(label("Materials (WinUI)")));
    let panel = StackPanel::new();
    panel.set_spacing(28.0);
    panel.set_margin(Thickness::uniform(16.0));
    panel.set_vertical_alignment(VerticalAlignment::Top);
    panel.children().add(acrylic_section().upcast());
    panel.children().add(effects_section().upcast());
    panel.children().add(lighting_section().upcast());
    panel.children().add(reveal_section().upcast());
    panel.children().add(backdrop_section().upcast());
    panel.children().add(menu_section().upcast());
    let scroller = ScrollViewer::new();
    scroller.set_content(Some(Control::boxed(panel)));
    page.set_content(Some(Control::boxed(scroller)));
    page
}

/// Applies the control theme `key` of the application's resources (the
/// Reveal styles of the WinUI theme) to `control`, when the theme has it.
fn apply_theme(control: &Ref<Control>, key: &str) {
    let key = ferroui_base::controls::ResourceKey::from(key);
    let weak = control.downgrade();
    let _ = control.attached_to_visual_tree(move |_| {
        let Some(control) = weak.upgrade() else { return };
        if let Some(theme) = control
            .try_find_resource(&key, None)
            .flatten()
            .and_then(|value| {
                let value: &dyn ferroui_base::AnyValue = &*value;
                value.downcast_ref::<Ref<ferroui_base::styling::ControlTheme>>().cloned()
            })
        {
            control.set_theme(Some(theme));
        }
    });
}
