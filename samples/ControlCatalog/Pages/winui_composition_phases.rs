//! The sections of the page "Composition (WinUI)" for phases 2 to 6 of the port (not from
//! upstream; `docs/porting/winui-composition.md`): visuals, brushes, animations, shadows and
//! lights, the interaction tracker and the effect brushes. Each section builds its demo the first
//! time its expander opens, so the page stays light until a demo is looked at.

use super::winui_composition_page::{brush, compositor_of, scalar, section, text, vector};
use ferroui_base::animation::PlaybackDirection;
use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::collections::FerroList;
use ferroui_base::media::{Color, FontWeight, IGradientStop};
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::platform::{AlphaFormat, PixelFormats};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::animations::{
    AnimationIterationBehavior, CompositionEasingFunction, CompositionEasingFunctionMode, ICompositionAnimation,
};
use ferroui_base::rendering::composition::effects::{
    ColorSourceEffect, CompositeEffect, CrossFadeEffect, GaussianBlurEffect, GraphicsEffectSource, OpacityEffect,
};
use ferroui_base::rendering::composition::interactions::{
    IInteractionTrackerOwner, InteractionSourceMode, InteractionTracker, InteractionTrackerIdleStateEnteredArgs,
    InteractionTrackerInertiaStateEnteredArgs, InteractionTrackerInteractingStateEnteredArgs,
    InteractionTrackerValuesChangedArgs, VisualInteractionSource,
};
use ferroui_base::rendering::composition::{
    CompositionBatchTypes, CompositionBrush, CompositionEffectBrush, CompositionEffectSourceParameter, CompositionPath,
    CompositionScopedBatch, CompositionVisual, Compositor, DropShadow, ICompositionObjectAnimations,
    ICompositionSurface, LoadedImageSurface, SpriteVisual,
};
use ferroui_base::{BoxedValue, CornerRadius, PixelSize, Ref, Thickness, Vector, Vector3D, WeakRef};
use ferroui_controls::hosting::ElementCompositionPreview;
use ferroui_controls::{Border, Button, Control, Expander, Slider, StackPanel, TextBlock, WrapPanel};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// What the smoke run does in an open section: presses its buttons.
type SmokeAction = Rc<dyn Fn()>;

/// A demo: its content, and what the smoke run does in it.
struct Demo {
    content: Ref<Control>,
    smoke: SmokeAction,
}

ferroui_base::ferro_thread_local! {
    /// The sections of the page last created, with their expanders and the smoke action of the
    /// demo once it is built, for the smoke run of the desktop entry point.
    static SECTIONS: RefCell<Vec<SectionEntry>> = const { RefCell::new(Vec::new()) };
}

struct SectionEntry {
    section: WeakRef<StackPanel>,
    expander: WeakRef<Expander>,
    smoke: Rc<RefCell<Option<SmokeAction>>>,
}

/// Opens the expander of every section of the page last created (for the smoke run of the
/// desktop entry point). Returns how many sections there are.
pub fn smoke_expand_sections() -> usize {
    SECTIONS.with(|sections| {
        let sections = sections.borrow();
        for entry in sections.iter() {
            if let Some(expander) = entry.expander.upgrade() {
                expander.set_is_expanded(true);
            }
        }
        sections.len()
    })
}

/// Brings section `index` of the page last created into view and runs its demo, as its buttons
/// do (for the smoke run). Returns whether the section exists and its demo is built.
pub fn smoke_show_section(index: usize) -> bool {
    let entry = SECTIONS.with(|sections| {
        sections.borrow().get(index).map(|entry| (entry.section.upgrade(), entry.smoke.borrow().clone()))
    });
    let Some((Some(section), smoke)) = entry else { return false };
    section.bring_into_view();
    let Some(smoke) = smoke else { return false };
    smoke();
    true
}

/// The sections of phases 2 to 6, for the page.
pub fn phase_sections() -> Vec<Ref<StackPanel>> {
    SECTIONS.with(|sections| sections.borrow_mut().clear());
    vec![
        lazy_section(
            "Visuals",
            "SpriteVisual with a CompositionColorBrush; ShapeVisual with rounded rectangle, ellipse and path geometries (the path animates TrimEnd); LayerVisual with an effect over its children; RedirectVisual drawing the visual of another element.",
            visuals_demo,
        ),
        lazy_section(
            "Brushes",
            "CompositionLinearGradientBrush and CompositionRadialGradientBrush with their WinUI members, CompositionNineGridBrush over a LoadedImageSurface, and CompositionMaskBrush.",
            brushes_demo,
        ),
        lazy_section(
            "Animations",
            "SpringScalarNaturalMotionAnimation and SpringVector3NaturalMotionAnimation with DampingRatio and Period; key frame animations with the CompositionEasingFunction curves side by side; a CompositionAnimationGroup whose CompositionScopedBatch reports its completion.",
            animations_demo,
        ),
        lazy_section(
            "Shadows and lights",
            "DropShadow with BlurRadius, Offset and Color; AmbientLight, DistantLight, PointLight and SpotLight lighting the visuals of their Targets.",
            shadows_and_lights_demo,
        ),
        lazy_section(
            "InteractionTracker",
            "InteractionTracker with a VisualInteractionSource redirected from the pointer: drag the surface, release to coast in inertia within MinPosition and MaxPosition. An expression animation reads tracker.Position.",
            interaction_tracker_demo,
        ),
        lazy_section(
            "Effects",
            "CompositionEffectFactory and CompositionEffectBrush for the effects of the port: GaussianBlurEffect, OpacityEffect, CrossFadeEffect, CompositeEffect and ColorSourceEffect, over a LoadedImageSurface.",
            effects_demo,
        ),
    ]
}

/// A section whose demo is built when its expander first opens.
fn lazy_section(title: &str, description: &str, build: fn() -> Demo) -> Ref<StackPanel> {
    let expander = Expander::new();
    expander.set_header(Some(label("Show the demo")));
    expander.set_horizontal_alignment(HorizontalAlignment::Stretch);
    let smoke: Rc<RefCell<Option<SmokeAction>>> = Rc::new(RefCell::new(None));
    {
        let weak = expander.downgrade();
        let smoke = smoke.clone();
        let _ = expander.expanding(move |_, _| {
            let Some(expander) = weak.upgrade() else { return };
            if expander.content().is_some() {
                return;
            }
            let demo = build();
            demo.content.set_horizontal_alignment(HorizontalAlignment::Left);
            *smoke.borrow_mut() = Some(demo.smoke);
            expander.set_content(Some(Control::boxed(demo.content)));
        });
    }
    let panel = section(title, description, &expander.clone().upcast());
    SECTIONS.with(|sections| {
        sections.borrow_mut().push(SectionEntry { section: panel.downgrade(), expander: expander.downgrade(), smoke })
    });
    panel
}

// --- helpers -------------------------------------------------------------------------------

fn label(value: &str) -> BoxedValue {
    Rc::new(value.to_owned()) as BoxedValue
}

fn caption(value: &str) -> Ref<TextBlock> {
    let block = text(value, 12.0, FontWeight::Normal);
    block.set_opacity(0.75);
    block
}

fn button(content: &str, click: impl Fn() + 'static) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(label(content)));
    button.click(move |_, _| click());
    button
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

/// A dark surface of the given size that composition visuals are shown on.
fn host(width: f64, height: f64, background: Color) -> Ref<Border> {
    let host = Border::new();
    host.set_width(width);
    host.set_height(height);
    host.set_corner_radius(CornerRadius::uniform(8.0));
    host.set_clip_to_bounds(true);
    host.set_horizontal_alignment(HorizontalAlignment::Left);
    host.set_vertical_alignment(VerticalAlignment::Top);
    host.set_background(Some(brush(background)));
    host
}

const DARK: Color = Color::from_rgb(0x20, 0x24, 0x30);
const LIGHT: Color = Color::from_rgb(0xEE, 0xF0, 0xF4);

/// A host with a caption under it.
fn tile(host: &Ref<Border>, text: &str) -> Ref<StackPanel> {
    let tile = column(6.0);
    tile.set_margin(Thickness::new(0.0, 0.0, 12.0, 12.0));
    tile.children().add(host.clone());
    let about = caption(text);
    about.set_max_width(host.width());
    about.set_horizontal_alignment(HorizontalAlignment::Left);
    tile.children().add(about);
    tile
}

/// Runs `build` once `host` is in the tree and the renderer has created its composition
/// visual, and again each time it is attached.
fn on_composition(host: &Ref<Border>, build: impl Fn(&Ref<Border>, &Rc<Compositor>) + 'static) {
    let build = Rc::new(build);
    let weak = host.downgrade();
    let _ = host.attached_to_visual_tree(move |_| {
        let (weak, build) = (weak.clone(), build.clone());
        let _ = ferroui_base::threading::Dispatcher::ui_thread().invoke_async_local_with_priority(
            move || {
                let Some(host) = weak.upgrade() else { return };
                let Some(compositor) = compositor_of(&host) else { return };
                build(&host, &compositor);
            },
            ferroui_base::threading::DispatcherPriority::BACKGROUND,
        );
    });
}

fn show(host: &Ref<Border>, visual: &Rc<CompositionVisual>) {
    ElementCompositionPreview::set_element_child_visual(host, Some(visual.clone()));
}

fn color_brush(compositor: &Rc<Compositor>, color: Color) -> Rc<CompositionBrush> {
    (*compositor.create_color_brush_with_color(color)).clone()
}

fn sprite(compositor: &Rc<Compositor>, offset: (f64, f64), size: (f64, f64), color: Color) -> SpriteVisual {
    let sprite = compositor.create_sprite_visual();
    sprite.set_offset(Vector3D::new(offset.0, offset.1, 0.0));
    sprite.set_size(Vector::new(size.0, size.1));
    sprite.set_brush(Some(color_brush(compositor, color)));
    sprite
}

/// A slider with its name and value above it; `changed` gets the new value. Returns the panel
/// and the slider.
fn slider(
    name: &'static str,
    range: (f64, f64),
    value: f64,
    changed: impl Fn(f64) + 'static,
) -> (Ref<StackPanel>, Ref<Slider>) {
    let panel = column(2.0);
    let title = caption(&format!("{name}: {value:.2}"));
    let slider = Slider::new();
    slider.set_minimum(range.0);
    slider.set_maximum(range.1);
    slider.set_range_value(value);
    slider.set_width(200.0);
    slider.set_horizontal_alignment(HorizontalAlignment::Left);
    let weak = title.downgrade();
    let _ = slider.value_changed(move |_, e| {
        if let Some(title) = weak.upgrade() {
            title.set_text(Some(&format!("{name}: {:.2}", e.new_value())));
        }
        changed(e.new_value());
    });
    panel.children().add(title);
    panel.children().add(slider.clone());
    (panel, slider)
}

/// A bitmap of `width` x `height` pixels at 96 DPI whose premultiplied RGBA pixels are given by
/// `pixel`.
fn bitmap(width: i32, height: i32, pixel: impl Fn(i32, i32) -> [u8; 4]) -> Bitmap {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            data.extend_from_slice(&pixel(x, y));
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

/// A checkerboard of blue squares and transparent ones.
fn checkerboard(compositor: &Rc<Compositor>) -> Rc<CompositionBrush> {
    let image = bitmap(64, 64, |x, y| if (x / 16 + y / 16) % 2 == 0 { [0x2E, 0x86, 0xDE, 0xFF] } else { [0, 0, 0, 0] });
    let surface: Rc<dyn ICompositionSurface> = LoadedImageSurface::from_bitmap(compositor, &image);
    (*compositor.create_surface_brush_with_surface(surface)).clone()
}

/// A key frame animation of `target` that plays back and forth forever.
fn forever(compositor: &Rc<Compositor>, from: f32, to: f32, ms: u64) -> Rc<dyn ICompositionAnimation> {
    let animation = compositor.create_scalar_key_frame_animation();
    animation.insert_key_frame(0.0, from);
    animation.insert_key_frame(1.0, to);
    animation.set_duration(Duration::from_millis(ms));
    animation.set_iteration_behavior(AnimationIterationBehavior::Forever);
    animation.set_direction(PlaybackDirection::Alternate);
    animation
}

// --- visuals -------------------------------------------------------------------------------

fn visuals_demo() -> Demo {
    // A sprite visual with a colour brush, and a smaller one turned inside it.
    let sprite_host = host(150.0, 150.0, DARK);
    on_composition(&sprite_host, |host, compositor| {
        let outer = sprite(compositor, (15.0, 15.0), (120.0, 120.0), Color::from_rgb(0x2E, 0x86, 0xDE));
        let inner = sprite(compositor, (30.0, 30.0), (60.0, 60.0), Color::from_rgb(0xF5, 0xC5, 0x42));
        inner.set_center_point(Vector3D::new(30.0, 30.0, 0.0));
        inner.set_rotation_angle_in_degrees(30.0);
        outer.children().insert_at_top(inner.visual().clone());
        show(host, outer.visual());
    });

    // A shape visual: a rounded rectangle, an ellipse and a path whose TrimEnd animates.
    let shape_host = host(220.0, 150.0, DARK);
    on_composition(&shape_host, |host, compositor| {
        let visual = compositor.create_shape_visual();
        visual.set_size(Vector::new(220.0, 150.0));
        let rounded = compositor.create_rounded_rectangle_geometry();
        rounded.set_offset(Vector::new(14.0, 14.0));
        rounded.set_size(Vector::new(90.0, 56.0));
        rounded.set_corner_radius(Vector::new(14.0, 14.0));
        let rounded_shape = compositor.create_sprite_shape_with_geometry(Some((*rounded).clone()));
        rounded_shape.set_fill_brush(Some(color_brush(compositor, Color::from_rgb(0xE8, 0x5D, 0x3A))));
        visual.shapes().append((*rounded_shape).clone());

        let ellipse = compositor.create_ellipse_geometry();
        ellipse.set_center(Vector::new(160.0, 42.0));
        ellipse.set_radius(Vector::new(40.0, 28.0));
        let ellipse_shape = compositor.create_sprite_shape_with_geometry(Some((*ellipse).clone()));
        ellipse_shape.set_fill_brush(Some(color_brush(compositor, Color::from_rgb(0x3F, 0x9E, 0x5A))));
        ellipse_shape.set_stroke_brush(Some(color_brush(compositor, Color::from_rgb(0xE0, 0xF0, 0xE4))));
        ellipse_shape.set_stroke_thickness(2.0);
        visual.shapes().append((*ellipse_shape).clone());

        if let Ok(path) = CompositionPath::parse("M 16,128 C 60,70 120,160 204,96") {
            let geometry = compositor.create_path_geometry_with_path(Some(path));
            let path_shape = compositor.create_sprite_shape_with_geometry(Some((*geometry).clone()));
            path_shape.set_stroke_brush(Some(color_brush(compositor, Color::from_rgb(0xF5, 0xC5, 0x42))));
            path_shape.set_stroke_thickness(5.0);
            path_shape.set_stroke_start_cap(ferroui_base::rendering::composition::CompositionStrokeCap::Round);
            path_shape.set_stroke_end_cap(ferroui_base::rendering::composition::CompositionStrokeCap::Round);
            visual.shapes().append((*path_shape).clone());
            geometry.start_animation("TrimEnd", &*forever(compositor, 0.0, 1.0, 1600));
        }
        show(host, &visual);
    });

    // A layer visual: three overlapping sprites composed into a layer, blurred as one.
    let layer_host = host(150.0, 150.0, DARK);
    on_composition(&layer_host, |host, compositor| {
        let layer = compositor.create_layer_visual();
        layer.set_size(Vector::new(150.0, 150.0));
        for (offset, color) in [
            ((20.0, 20.0), Color::from_rgb(0x9B, 0x59, 0xB6)),
            ((50.0, 45.0), Color::from_rgb(0x1A, 0xA6, 0xA6)),
            ((80.0, 70.0), Color::from_rgb(0xD9, 0xA4, 0x41)),
        ] {
            layer.children().insert_at_top(sprite(compositor, offset, (50.0, 50.0), color).visual().clone());
        }
        let factory = compositor.create_effect_factory(GaussianBlurEffect {
            name: "Blur".into(),
            blur_amount: 3.0,
            source: Some(CompositionEffectSourceParameter::new("layer").into()),
            ..Default::default()
        });
        layer.set_effect(Some(factory.create_brush()));
        show(host, &layer);
    });

    // A redirect visual: the visual of a live element, drawn again as a reflection.
    let clicks = Rc::new(Cell::new(0u32));
    let source = Border::new();
    source.set_width(150.0);
    source.set_height(90.0);
    source.set_corner_radius(CornerRadius::uniform(8.0));
    source.set_background(Some(brush(Color::from_rgb(0xC0, 0x4A, 0x8C))));
    source.set_horizontal_alignment(HorizontalAlignment::Left);
    let counter = text("Clicks: 0", 14.0, FontWeight::SemiBold);
    counter.set_foreground(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
    counter.set_margin(Thickness::uniform(12.0));
    source.set_child(counter.clone());
    let click = {
        let counter = counter.downgrade();
        let clicks = clicks.clone();
        move || {
            clicks.set(clicks.get() + 1);
            if let Some(counter) = counter.upgrade() {
                counter.set_text(Some(&format!("Clicks: {}", clicks.get())));
            }
        }
    };
    let click: SmokeAction = Rc::new(click);
    let mirror_host = host(150.0, 90.0, DARK);
    {
        let source = source.downgrade();
        on_composition(&mirror_host, move |host, _| {
            let Some(source) = source.upgrade() else { return };
            let Some(source_visual) = ElementCompositionPreview::get_element_visual(&source) else { return };
            let compositor = source_visual.compositor().clone();
            let redirect = compositor.create_redirect_visual_with_source_visual(Some(source_visual));
            redirect.set_size(Vector::new(150.0, 90.0));
            // Flipped about the bottom edge of the host, and faded.
            redirect.set_scale(Vector3D::new(1.0, -1.0, 1.0));
            redirect.set_offset(Vector3D::new(0.0, 90.0, 0.0));
            redirect.set_opacity(0.55);
            show(host, &redirect);
        });
    }
    let redirect_tile = column(6.0);
    redirect_tile.set_margin(Thickness::new(0.0, 0.0, 12.0, 12.0));
    redirect_tile.children().add(source);
    redirect_tile.children().add(mirror_host);
    {
        let click = click.clone();
        redirect_tile.children().add(button("Click the element", move || click()));
    }
    redirect_tile.children().add(caption("RedirectVisual: its Source is the element visual above"));

    let tiles = WrapPanel::new();
    tiles.set_orientation(Orientation::Horizontal);
    tiles.children().add(tile(&sprite_host, "SpriteVisual, a child turned by RotationAngleInDegrees"));
    tiles.children().add(tile(&shape_host, "ShapeVisual: rounded rectangle, ellipse, a path animating TrimEnd"));
    tiles.children().add(tile(&layer_host, "LayerVisual with a GaussianBlurEffect brush as Effect"));
    tiles.children().add(redirect_tile);
    Demo { content: tiles.upcast(), smoke: click }
}

// --- brushes -------------------------------------------------------------------------------

fn gradient_stops(compositor: &Rc<Compositor>, stops: &[(f32, Color)]) -> FerroList<Rc<dyn IGradientStop>> {
    FerroList::from_items(
        stops
            .iter()
            .map(|&(offset, color)| compositor.create_color_gradient_stop(offset, color) as Rc<dyn IGradientStop>),
    )
}

/// A rounded frame of 24 x 24 pixels: a 4 pixel border around a lighter centre, with round
/// corners, for the nine-grid brush.
fn frame_bitmap() -> Bitmap {
    bitmap(24, 24, |x, y| {
        let (cx, cy) = (x.clamp(8, 15), y.clamp(8, 15));
        let away = |d: i32| d as f64 + 0.5 * d.signum() as f64;
        let (dx, dy) = (away(x - cx), away(y - cy));
        let distance = (dx * dx + dy * dy).sqrt();
        if distance > 8.0 {
            [0, 0, 0, 0]
        } else if distance > 4.0 || x < 4 || y < 4 || x > 19 || y > 19 {
            [0xF5, 0xC5, 0x42, 0xFF]
        } else {
            [0x40, 0x34, 0x18, 0xFF]
        }
    })
}

fn brushes_demo() -> Demo {
    let linear_host = host(170.0, 110.0, DARK);
    on_composition(&linear_host, |host, compositor| {
        let linear = compositor.create_linear_gradient_brush();
        linear.set_start_point_vector(Vector::new(0.0, 0.0));
        linear.set_end_point_vector(Vector::new(1.0, 1.0));
        linear.set_color_stops(gradient_stops(
            compositor,
            &[
                (0.0, Color::from_rgb(0x2E, 0x86, 0xDE)),
                (0.5, Color::from_rgb(0x9B, 0x59, 0xB6)),
                (1.0, Color::from_rgb(0xE8, 0x5D, 0x3A)),
            ],
        ));
        let visual = compositor.create_sprite_visual();
        visual.set_size(Vector::new(170.0, 110.0));
        visual.set_brush(Some((**linear).clone()));
        show(host, visual.visual());
    });

    let radial_host = host(170.0, 110.0, DARK);
    on_composition(&radial_host, |host, compositor| {
        let radial = compositor.create_radial_gradient_brush();
        radial.set_ellipse_center(Vector::new(0.5, 0.5));
        radial.set_ellipse_radius(Vector::new(0.5, 0.6));
        radial.set_gradient_origin_offset(Vector::new(-0.2, -0.2));
        radial.set_color_stops(gradient_stops(
            compositor,
            &[
                (0.0, Color::from_rgb(0xFF, 0xF4, 0xC0)),
                (0.45, Color::from_rgb(0xD9, 0xA4, 0x41)),
                (1.0, Color::from_rgb(0x40, 0x20, 0x10)),
            ],
        ));
        let visual = compositor.create_sprite_visual();
        visual.set_size(Vector::new(170.0, 110.0));
        visual.set_brush(Some((**radial).clone()));
        show(host, visual.visual());
    });

    let nine_grid_host = host(170.0, 110.0, DARK);
    on_composition(&nine_grid_host, |host, compositor| {
        let surface: Rc<dyn ICompositionSurface> = LoadedImageSurface::from_bitmap(compositor, &frame_bitmap());
        let source = compositor.create_surface_brush_with_surface(surface.clone());
        let nine_grid = compositor.create_nine_grid_brush();
        nine_grid.set_source(Some((*source).clone()));
        nine_grid.set_insets(8.0);
        let framed = compositor.create_sprite_visual();
        framed.set_offset(Vector3D::new(10.0, 10.0, 0.0));
        framed.set_size(Vector::new(150.0, 52.0));
        framed.set_brush(Some((*nine_grid).clone()));
        // The same image stretched by a surface brush, for comparison.
        let stretched = compositor.create_sprite_visual();
        stretched.set_offset(Vector3D::new(10.0, 70.0, 0.0));
        stretched.set_size(Vector::new(150.0, 30.0));
        stretched.set_brush(Some((*compositor.create_surface_brush_with_surface(surface)).clone()));
        let root = compositor.create_container_visual();
        root.set_size(Vector::new(170.0, 110.0));
        root.children().insert_at_top(framed.visual().clone());
        root.children().insert_at_top(stretched.visual().clone());
        show(host, &root);
    });

    let mask_host = host(170.0, 110.0, DARK);
    on_composition(&mask_host, |host, compositor| {
        let source = compositor.create_linear_gradient_brush();
        source.set_end_point_vector(Vector::new(1.0, 0.0));
        source.set_color_stops(gradient_stops(
            compositor,
            &[(0.0, Color::from_rgb(0x1A, 0xA6, 0xA6)), (1.0, Color::from_rgb(0x3F, 0x9E, 0x5A))],
        ));
        let mask = compositor.create_radial_gradient_brush();
        mask.set_ellipse_center(Vector::new(0.5, 0.5));
        mask.set_ellipse_radius(Vector::new(0.5, 0.5));
        mask.set_color_stops(gradient_stops(
            compositor,
            &[
                (0.0, Color::from_argb(0xFF, 0, 0, 0)),
                (0.7, Color::from_argb(0xC0, 0, 0, 0)),
                (1.0, Color::from_argb(0, 0, 0, 0)),
            ],
        ));
        let masked = compositor.create_mask_brush();
        masked.set_source(Some((**source).clone()));
        masked.set_mask(Some((**mask).clone()));
        let visual = compositor.create_sprite_visual();
        visual.set_size(Vector::new(170.0, 110.0));
        visual.set_brush(Some((*masked).clone()));
        show(host, visual.visual());
    });

    let tiles = WrapPanel::new();
    tiles.set_orientation(Orientation::Horizontal);
    tiles.children().add(tile(&linear_host, "CompositionLinearGradientBrush, StartPoint (0, 0) to EndPoint (1, 1)"));
    tiles.children().add(tile(&radial_host, "CompositionRadialGradientBrush with a GradientOriginOffset"));
    tiles.children().add(tile(&nine_grid_host, "CompositionNineGridBrush (insets 8), and the image stretched"));
    tiles.children().add(tile(&mask_host, "CompositionMaskBrush: a gradient through a radial mask"));
    Demo { content: tiles.upcast(), smoke: Rc::new(|| {}) }
}

// --- animations ----------------------------------------------------------------------------

/// The easing curves of `CompositionEasingFunction`, with their names.
fn easings(compositor: &Compositor) -> Vec<(&'static str, ferroui_base::animation::easings::Easing)> {
    use CompositionEasingFunctionMode::{InOut, Out};
    vec![
        ("Linear", CompositionEasingFunction::create_linear_easing_function(compositor)),
        (
            "CubicBezier",
            CompositionEasingFunction::create_cubic_bezier_easing_function(
                compositor,
                Vector2::new(0.42, 0.0),
                Vector2::new(0.58, 1.0),
            ),
        ),
        ("Step (5)", CompositionEasingFunction::create_step_easing_function_with_step_count(compositor, 5)),
        ("Back", CompositionEasingFunction::create_back_easing_function(compositor, Out, 0.6)),
        ("Bounce", CompositionEasingFunction::create_bounce_easing_function(compositor, Out, 3, 2.0)),
        ("Circle", CompositionEasingFunction::create_circle_easing_function(compositor, InOut)),
        ("Elastic", CompositionEasingFunction::create_elastic_easing_function(compositor, Out, 3, 4.0)),
        ("Exponential", CompositionEasingFunction::create_exponential_easing_function(compositor, InOut, 6.0)),
        ("Power (3)", CompositionEasingFunction::create_power_easing_function(compositor, InOut, 3.0)),
        ("Sine", CompositionEasingFunction::create_sine_easing_function(compositor, InOut)),
    ]
}

/// The state of the animations demo: its visuals once they exist, the parameters of the
/// springs and the batch of the group.
#[derive(Default)]
struct Animations {
    springs: RefCell<Option<(SpriteVisual, SpriteVisual)>>,
    damping_ratio: Cell<f32>,
    period_ms: Cell<f64>,
    sprung: Cell<bool>,
    eased: RefCell<Vec<SpriteVisual>>,
    eased_out: Cell<bool>,
    grouped: RefCell<Option<SpriteVisual>>,
    batch: RefCell<Option<(Rc<CompositionScopedBatch>, Rc<dyn IDisposable>)>>,
}

impl Animations {
    fn spring(&self) {
        let Some((scalar_target, vector_target)) = self.springs.borrow().clone() else { return };
        let compositor = scalar_target.compositor().clone();
        let sprung = !self.sprung.get();
        self.sprung.set(sprung);
        let period = Duration::from_secs_f64(self.period_ms.get() / 1000.0);

        let x = compositor.create_spring_scalar_animation();
        x.set_final_value(Some(if sprung { 330.0 } else { 16.0 }));
        x.set_damping_ratio(self.damping_ratio.get());
        x.set_period(period);
        scalar_target.start_animation("Offset.X", &*x);

        let offset = compositor.create_spring_vector3_animation();
        let target = if sprung { Vector3::new(330.0, 76.0, 0.0) } else { Vector3::new(16.0, 76.0, 0.0) };
        offset.set_final_value(Some(target));
        offset.set_damping_ratio(self.damping_ratio.get());
        offset.set_period(period);
        vector_target.start_animation("Offset", &*offset);
    }

    fn ease(&self) {
        let eased = self.eased.borrow();
        let Some(first) = eased.first() else { return };
        let compositor = first.compositor().clone();
        let out = !self.eased_out.get();
        self.eased_out.set(out);
        let (from, to) = if out { (0.0f32, 330.0f32) } else { (330.0, 0.0) };
        for (visual, (_, easing)) in eased.iter().zip(easings(&compositor)) {
            let animation = compositor.create_scalar_key_frame_animation();
            animation.insert_key_frame(0.0, from);
            animation.insert_key_frame_with_easing(1.0, to, easing.as_rc().clone());
            animation.set_duration(Duration::from_millis(1400));
            visual.start_animation("Offset.X", &*animation);
        }
    }

    fn group(&self, status: &WeakRef<TextBlock>) {
        let Some(visual) = self.grouped.borrow().clone() else { return };
        let compositor = visual.compositor().clone();
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        let group = compositor.create_animation_group();
        let turn = compositor.create_scalar_key_frame_animation();
        turn.insert_key_frame(0.0, 0.0);
        turn.insert_key_frame(1.0, std::f32::consts::TAU);
        turn.set_duration(Duration::from_millis(900));
        turn.set_target(Some("RotationAngle".to_owned()));
        group.add(turn);
        group.add(scalar(&compositor, "Opacity", 0.3, 1.0, 900));
        group.add(vector(&compositor, "Scale", Vector3D::new(0.5, 0.5, 1.0), Vector3D::new(1.0, 1.0, 1.0), 900));
        visual.start_animation_group(&*group);
        batch.end();
        let started = Instant::now();
        if let Some(status) = status.upgrade() {
            status.set_text(Some("The batch is running"));
        }
        let status = status.clone();
        let subscription = batch.completed(move |_| {
            if let Some(status) = status.upgrade() {
                status.set_text(Some(&format!("Completed raised after {} ms", started.elapsed().as_millis())));
            }
        });
        *self.batch.borrow_mut() = Some((batch, subscription));
    }
}

fn animations_demo() -> Demo {
    let state = Rc::new(Animations { damping_ratio: Cell::new(0.3), period_ms: Cell::new(60.0), ..Default::default() });
    let status = caption("Not run yet");

    // Springs.
    let spring_host = host(400.0, 130.0, DARK);
    {
        let state = Rc::downgrade(&state);
        on_composition(&spring_host, move |host, compositor| {
            let Some(state) = state.upgrade() else { return };
            let scalar_target = sprite(compositor, (16.0, 20.0), (40.0, 40.0), Color::from_rgb(0x2E, 0x86, 0xDE));
            let vector_target = sprite(compositor, (16.0, 76.0), (40.0, 40.0), Color::from_rgb(0xE8, 0x5D, 0x3A));
            let root = compositor.create_container_visual();
            root.set_size(Vector::new(400.0, 130.0));
            root.children().insert_at_top(scalar_target.visual().clone());
            root.children().insert_at_top(vector_target.visual().clone());
            show(host, &root);
            state.sprung.set(false);
            *state.springs.borrow_mut() = Some((scalar_target, vector_target));
        });
    }
    let spring_controls = row(16.0);
    {
        let state = state.clone();
        let (damping, _) = slider("DampingRatio", (0.05, 1.0), 0.3, move |value| state.damping_ratio.set(value as f32));
        spring_controls.children().add(damping);
    }
    {
        let state = state.clone();
        let (period, _) = slider("Period (ms)", (20.0, 200.0), 60.0, move |value| state.period_ms.set(value));
        spring_controls.children().add(period);
    }
    {
        let state = Rc::downgrade(&state);
        spring_controls.children().add(button("Spring", move || {
            if let Some(state) = state.upgrade() {
                state.spring();
            }
        }));
    }
    let springs = column(8.0);
    springs.children().add(spring_controls);
    springs.children().add(tile(
        &spring_host,
        "Blue: SpringScalarNaturalMotionAnimation of Offset.X. Orange: SpringVector3NaturalMotionAnimation of Offset.",
    ));

    // Easing functions side by side.
    const ROW: f64 = 24.0;
    let names = column(0.0);
    let eased_host = host(400.0, ROW * 10.0, DARK);
    {
        let state = Rc::downgrade(&state);
        on_composition(&eased_host, move |host, compositor| {
            let Some(state) = state.upgrade() else { return };
            let root = compositor.create_container_visual();
            root.set_size(Vector::new(400.0, ROW * 10.0));
            let visuals: Vec<SpriteVisual> = (0..easings(compositor).len())
                .map(|index| {
                    let offset = (0.0, index as f64 * ROW + 3.0);
                    let visual = sprite(compositor, offset, (30.0, ROW - 6.0), Color::from_rgb(0x3F, 0x9E, 0x5A));
                    root.children().insert_at_top(visual.visual().clone());
                    visual
                })
                .collect();
            show(host, &root);
            state.eased_out.set(false);
            *state.eased.borrow_mut() = visuals;
        });
    }
    // The names do not need the compositor: the default one gives them.
    if let Some(compositor) = Compositor::try_get_default_compositor() {
        for (name, _) in easings(&compositor) {
            let block = text(name, 12.0, FontWeight::Normal);
            block.set_height(ROW);
            block.set_width(90.0);
            block.set_vertical_alignment(VerticalAlignment::Center);
            names.children().add(block);
        }
    }
    let eased_row = row(8.0);
    eased_row.children().add(names);
    eased_row.children().add(eased_host);
    let eased = column(8.0);
    {
        let state = Rc::downgrade(&state);
        eased.children().add(button("Play the key frame animations", move || {
            if let Some(state) = state.upgrade() {
                state.ease();
            }
        }));
    }
    eased.children().add(eased_row);
    eased.children().add(caption("ScalarKeyFrameAnimation of Offset.X, 1.4 s, one CompositionEasingFunction per row"));

    // An animation group and its scoped batch.
    let group_host = host(200.0, 140.0, DARK);
    {
        let state = Rc::downgrade(&state);
        on_composition(&group_host, move |host, compositor| {
            let Some(state) = state.upgrade() else { return };
            let visual = sprite(compositor, (60.0, 30.0), (80.0, 80.0), Color::from_rgb(0x9B, 0x59, 0xB6));
            visual.set_center_point(Vector3D::new(40.0, 40.0, 0.0));
            show(host, visual.visual());
            *state.grouped.borrow_mut() = Some(visual);
        });
    }
    let group = column(8.0);
    {
        let state = Rc::downgrade(&state);
        let status = status.downgrade();
        group.children().add(button("Run the group in a scoped batch", move || {
            if let Some(state) = state.upgrade() {
                state.group(&status);
            }
        }));
    }
    group.children().add(tile(&group_host, "CompositionAnimationGroup: RotationAngle, Opacity and Scale"));
    group.children().add(status.clone());

    let content = column(20.0);
    content.children().add(springs);
    content.children().add(eased);
    content.children().add(group);
    // The state lives as long as the content: the handlers hold it weakly.
    {
        let keep = state.clone();
        let _ = content.detached_from_visual_tree(move |_| {
            let _ = &keep;
        });
    }
    let smoke: SmokeAction = {
        let state = state.clone();
        let status = status.downgrade();
        Rc::new(move || {
            state.spring();
            state.ease();
            state.group(&status);
        })
    };
    Demo { content: content.upcast(), smoke }
}

// --- shadows and lights --------------------------------------------------------------------

fn shadows_and_lights_demo() -> Demo {
    // A drop shadow with adjustable blur, offset and colour.
    let shadow: Rc<RefCell<Option<Rc<DropShadow>>>> = Rc::new(RefCell::new(None));
    let shadow_host = host(320.0, 190.0, LIGHT);
    {
        let shadow = shadow.clone();
        on_composition(&shadow_host, move |host, compositor| {
            let visual = sprite(compositor, (80.0, 45.0), (160.0, 100.0), Color::from_rgb(0x2E, 0x86, 0xDE));
            let drop_shadow = compositor.create_drop_shadow();
            drop_shadow.set_blur_radius(20.0);
            drop_shadow.set_offset(Vector3D::new(10.0, 10.0, 0.0));
            drop_shadow.set_color(Color::from_argb(0xC0, 0, 0, 0));
            visual.set_shadow(Some(drop_shadow.clone()));
            show(host, visual.visual());
            *shadow.borrow_mut() = Some(drop_shadow);
        });
    }
    let controls = column(8.0);
    let (blur_panel, blur_slider) = {
        let shadow = shadow.clone();
        slider("BlurRadius", (0.0, 60.0), 20.0, move |value| {
            if let Some(shadow) = shadow.borrow().as_ref() {
                shadow.set_blur_radius(value as f32);
            }
        })
    };
    controls.children().add(blur_panel);
    {
        let shadow = shadow.clone();
        let (offset, _) = slider("Offset", (-30.0, 30.0), 10.0, move |value| {
            if let Some(shadow) = shadow.borrow().as_ref() {
                shadow.set_offset(Vector3D::new(value, value, 0.0));
            }
        });
        controls.children().add(offset);
    }
    let colors = row(6.0);
    for (name, color) in [
        ("Black", Color::from_argb(0xC0, 0, 0, 0)),
        ("Blue", Color::from_argb(0xE0, 0x10, 0x40, 0xC0)),
        ("Orange", Color::from_argb(0xE0, 0xE8, 0x5D, 0x3A)),
    ] {
        let shadow = shadow.clone();
        colors.children().add(button(name, move || {
            if let Some(shadow) = shadow.borrow().as_ref() {
                shadow.set_color(color);
            }
        }));
    }
    controls.children().add(colors);
    let shadow_row = row(16.0);
    shadow_row.children().add(tile(&shadow_host, "SpriteVisual.Shadow: a DropShadow"));
    shadow_row.children().add(controls);

    // The lights, each over its own scene.
    let lights = WrapPanel::new();
    lights.set_orientation(Orientation::Horizontal);
    for (index, about) in [
        "AmbientLight: a warm colour over the whole scene",
        "DistantLight: white light from the upper left",
        "PointLight: its Offset moves over the scene",
        "SpotLight: an inner and an outer cone",
    ]
    .into_iter()
    .enumerate()
    {
        let light_host = host(160.0, 120.0, DARK);
        // The light is disposed with its handle: the tile keeps it.
        let light: Rc<RefCell<Option<Box<dyn Any>>>> = Rc::new(RefCell::new(None));
        on_composition(&light_host, move |host, compositor| {
            let scene = light_scene(compositor);
            *light.borrow_mut() = Some(add_light(compositor, index, &scene));
            show(host, &scene);
        });
        lights.children().add(tile(&light_host, about));
    }

    let content = column(20.0);
    content.children().add(shadow_row);
    content.children().add(lights);
    let smoke: SmokeAction = Rc::new(move || {
        blur_slider.set_range_value(40.0);
        if let Some(shadow) = shadow.borrow().as_ref() {
            shadow.set_color(Color::from_argb(0xE0, 0xE8, 0x5D, 0x3A));
        }
    });
    Demo { content: content.upcast(), smoke }
}

/// The scene the lights shine on: a grey ground and three coloured blocks.
fn light_scene(compositor: &Rc<Compositor>) -> Rc<CompositionVisual> {
    let scene = compositor.create_container_visual();
    scene.set_size(Vector::new(160.0, 120.0));
    let ground = sprite(compositor, (0.0, 0.0), (160.0, 120.0), Color::from_rgb(0xB0, 0xB4, 0xC0));
    scene.children().insert_at_top(ground.visual().clone());
    for (offset, color) in [
        ((14.0, 18.0), Color::from_rgb(0xE8, 0x5D, 0x3A)),
        ((60.0, 50.0), Color::from_rgb(0x2E, 0x86, 0xDE)),
        ((106.0, 18.0), Color::from_rgb(0x3F, 0x9E, 0x5A)),
    ] {
        scene.children().insert_at_top(sprite(compositor, offset, (40.0, 52.0), color).visual().clone());
    }
    scene
}

/// Adds light `index` (ambient, distant, point, spot) over `scene` and returns its handle.
fn add_light(compositor: &Rc<Compositor>, index: usize, scene: &Rc<CompositionVisual>) -> Box<dyn Any> {
    match index {
        0 => {
            let light = compositor.create_ambient_light();
            light.set_color(Color::from_rgb(0xFF, 0xB8, 0x70));
            light.set_intensity(1.0);
            light.targets().add(scene.clone());
            Box::new(light)
        }
        1 => {
            let light = compositor.create_distant_light();
            light.set_color(Color::from_rgb(0xFF, 0xFF, 0xFF));
            light.set_direction(Vector3D::new(1.0, 1.0, -1.0));
            light.set_coordinate_space(Some(scene.clone()));
            light.targets().add(scene.clone());
            Box::new(light)
        }
        2 => {
            let light = compositor.create_point_light();
            light.set_color(Color::from_rgb(0xFF, 0xF0, 0xD0));
            light.set_offset(Vector3D::new(20.0, 20.0, 60.0));
            light.set_coordinate_space(Some(scene.clone()));
            light.targets().add(scene.clone());
            let travel = compositor.create_vector3d_key_frame_animation();
            travel.insert_key_frame(0.0, Vector3D::new(20.0, 20.0, 60.0));
            travel.insert_key_frame(0.5, Vector3D::new(140.0, 60.0, 60.0));
            travel.insert_key_frame(1.0, Vector3D::new(20.0, 100.0, 60.0));
            travel.set_duration(Duration::from_millis(2400));
            travel.set_iteration_behavior(AnimationIterationBehavior::Forever);
            travel.set_direction(PlaybackDirection::Alternate);
            light.start_animation("Offset", &*travel);
            Box::new(light)
        }
        _ => {
            let light = compositor.create_spot_light();
            light.set_offset(Vector3D::new(80.0, 60.0, 100.0));
            light.set_direction(Vector3D::new(0.0, 0.0, -1.0));
            light.set_inner_cone_angle_in_degrees(14.0);
            light.set_outer_cone_angle_in_degrees(34.0);
            light.set_inner_cone_color(Color::from_rgb(0xFF, 0xFF, 0xFF));
            light.set_outer_cone_color(Color::from_rgb(0x60, 0x80, 0xFF));
            light.set_coordinate_space(Some(scene.clone()));
            light.targets().add(scene.clone());
            Box::new(light)
        }
    }
}

// --- interaction tracker -------------------------------------------------------------------

/// The owner of the tracker: writes its state and position under the surface.
struct TrackerStatus {
    status: WeakRef<TextBlock>,
}

impl TrackerStatus {
    fn write(&self, value: String) {
        if let Some(status) = self.status.upgrade() {
            status.set_text(Some(&value));
        }
    }
}

impl IInteractionTrackerOwner for TrackerStatus {
    fn idle_state_entered(&self, sender: &Rc<InteractionTracker>, _args: &InteractionTrackerIdleStateEnteredArgs) {
        let position = sender.position();
        self.write(format!("Idle at ({:.0}, {:.0})", position.x, position.y));
    }

    fn inertia_state_entered(
        &self,
        _sender: &Rc<InteractionTracker>,
        args: &InteractionTrackerInertiaStateEnteredArgs,
    ) {
        let rest = args.modified_resting_position.unwrap_or(args.natural_resting_position);
        self.write(format!("Inertia, coming to rest at ({:.0}, {:.0})", rest.x, rest.y));
    }

    fn interacting_state_entered(
        &self,
        _sender: &Rc<InteractionTracker>,
        _args: &InteractionTrackerInteractingStateEnteredArgs,
    ) {
        self.write("Interacting".to_owned());
    }

    fn values_changed(&self, sender: &Rc<InteractionTracker>, args: &InteractionTrackerValuesChangedArgs) {
        let state = format!("{:?}", sender.state());
        self.write(format!("{state}: Position ({:.0}, {:.0})", args.position.x, args.position.y));
    }
}

/// The tracker and the source of the demo, once the surface has its visual.
#[derive(Default)]
struct Tracker {
    tracker: RefCell<Option<Rc<InteractionTracker>>>,
    source: RefCell<Option<Rc<VisualInteractionSource>>>,
}

fn interaction_tracker_demo() -> Demo {
    const VIEW: (f64, f64) = (420.0, 240.0);
    const CONTENT: (f64, f64) = (720.0, 480.0);
    let state = Rc::new(Tracker::default());
    let status = caption("Drag the surface");
    let surface = host(VIEW.0, VIEW.1, DARK);
    {
        let state = Rc::downgrade(&state);
        let status = status.downgrade();
        on_composition(&surface, move |host, compositor| {
            let Some(state) = state.upgrade() else { return };
            let Some(host_visual) = ElementCompositionPreview::get_element_visual(host) else { return };
            // The content: a gradient with a grid of tiles, larger than the surface.
            let content = compositor.create_sprite_visual();
            content.set_size(Vector::new(CONTENT.0, CONTENT.1));
            let gradient = compositor.create_linear_gradient_brush();
            gradient.set_end_point_vector(Vector::new(1.0, 1.0));
            gradient.set_color_stops(gradient_stops(
                compositor,
                &[(0.0, Color::from_rgb(0x1A, 0x3A, 0x6A)), (1.0, Color::from_rgb(0x6A, 0x1A, 0x4A))],
            ));
            content.set_brush(Some((**gradient).clone()));
            for column in 0..9 {
                for row in 0..6 {
                    let shade = 0x60 + ((column * 13 + row * 29) % 0x80) as u8;
                    let tile = sprite(
                        compositor,
                        (20.0 + column as f64 * 78.0, 20.0 + row as f64 * 76.0),
                        (60.0, 56.0),
                        Color::from_argb(0xC0, shade, 0xD0, 0xFF - shade / 2),
                    );
                    content.children().insert_at_top(tile.visual().clone());
                }
            }
            show(host, content.visual());

            let owner = Rc::new(TrackerStatus { status: status.clone() });
            let tracker = InteractionTracker::create_with_owner(compositor, owner);
            tracker.set_min_position(Vector3::new(0.0, 0.0, 0.0));
            tracker.set_max_position(Vector3::new((CONTENT.0 - VIEW.0) as f32, (CONTENT.1 - VIEW.1) as f32, 0.0));
            let source = VisualInteractionSource::create(&host_visual);
            source.set_position_x_source_mode(InteractionSourceMode::EnabledWithInertia);
            source.set_position_y_source_mode(InteractionSourceMode::EnabledWithInertia);
            tracker.interaction_sources().add(source.clone());
            let follow = compositor.create_expression_animation_with("-tracker.Position");
            follow.set_reference_parameter("tracker", tracker.clone());
            content.start_animation("Offset", &*follow);
            *state.tracker.borrow_mut() = Some(tracker);
            *state.source.borrow_mut() = Some(source);
        });
    }
    {
        let state = Rc::downgrade(&state);
        surface.add_handler(InputElement::pointer_pressed_event(), move |_, e: &PointerPressedEventArgs| {
            let Some(state) = state.upgrade() else { return };
            let source = state.source.borrow().clone();
            if let Some(source) = source {
                source.try_redirect_for_manipulation(e);
            }
        });
    }
    let fling = {
        let state = Rc::downgrade(&state);
        move || {
            let Some(state) = state.upgrade() else { return };
            let tracker = state.tracker.borrow().clone();
            if let Some(tracker) = tracker {
                tracker.try_update_position_with_additional_velocity(Vector3::new(900.0, 600.0, 0.0));
            }
        }
    };
    let fling: SmokeAction = Rc::new(fling);
    let buttons = row(8.0);
    {
        let fling = fling.clone();
        buttons.children().add(button("Fling (TryUpdatePositionWithAdditionalVelocity)", move || fling()));
    }
    {
        let state = Rc::downgrade(&state);
        buttons.children().add(button("Back to the origin (TryUpdatePosition)", move || {
            let Some(state) = state.upgrade() else { return };
            let tracker = state.tracker.borrow().clone();
            if let Some(tracker) = tracker {
                tracker.try_update_position(Vector3::new(0.0, 0.0, 0.0));
            }
        }));
    }
    let content = column(8.0);
    content.children().add(buttons);
    content.children().add(surface);
    content.children().add(status);
    {
        let keep = state.clone();
        let _ = content.detached_from_visual_tree(move |_| {
            let _ = &keep;
        });
    }
    Demo { content: content.upcast(), smoke: fling }
}

// --- effects -------------------------------------------------------------------------------

fn parameter(name: &str) -> Option<GraphicsEffectSource> {
    Some(CompositionEffectSourceParameter::new(name).into())
}

/// A sprite of the size of an effect tile painted with `effect`.
fn effect_sprite(compositor: &Rc<Compositor>, effect: &CompositionEffectBrush) -> SpriteVisual {
    let visual = compositor.create_sprite_visual();
    visual.set_size(Vector::new(150.0, 100.0));
    visual.set_brush(Some(effect.brush().clone()));
    visual
}

fn effects_demo() -> Demo {
    let blur: Rc<RefCell<Option<CompositionEffectBrush>>> = Rc::new(RefCell::new(None));
    let mut tiles = Vec::new();

    let blur_host = host(150.0, 100.0, DARK);
    {
        let blur = blur.clone();
        on_composition(&blur_host, move |host, compositor| {
            let factory = compositor.create_effect_factory_with_properties(
                GaussianBlurEffect {
                    name: "Blur".into(),
                    blur_amount: 3.0,
                    source: parameter("image"),
                    ..Default::default()
                },
                &["Blur.BlurAmount"],
            );
            let brush = factory.create_brush();
            brush.set_source_parameter("image", Some(checkerboard(compositor)));
            show(host, effect_sprite(compositor, &brush).visual());
            *blur.borrow_mut() = Some(brush);
        });
    }
    tiles.push((blur_host, "GaussianBlurEffect, BlurAmount from the slider"));

    let opacity_host = host(150.0, 100.0, DARK);
    on_composition(&opacity_host, |host, compositor| {
        let factory = compositor.create_effect_factory(OpacityEffect {
            name: "Opacity".into(),
            opacity: 0.35,
            source: parameter("image"),
        });
        let brush = factory.create_brush();
        brush.set_source_parameter("image", Some(checkerboard(compositor)));
        show(host, effect_sprite(compositor, &brush).visual());
    });
    tiles.push((opacity_host, "OpacityEffect, Opacity 0.35"));

    let cross_fade_host = host(150.0, 100.0, DARK);
    on_composition(&cross_fade_host, |host, compositor| {
        let factory = compositor.create_effect_factory_with_properties(
            CrossFadeEffect {
                name: "Crossfade".into(),
                weight: 0.0,
                source1: parameter("image"),
                source2: parameter("fill"),
            },
            &["Crossfade.Weight"],
        );
        let brush = factory.create_brush();
        brush.set_source_parameter("image", Some(checkerboard(compositor)));
        brush.set_source_parameter("fill", Some(color_brush(compositor, Color::from_rgb(0xE8, 0x5D, 0x3A))));
        brush.start_animation("Crossfade.Weight", &*forever(compositor, 0.0, 1.0, 1800));
        show(host, effect_sprite(compositor, &brush).visual());
    });
    tiles.push((cross_fade_host, "CrossFadeEffect, Weight animated"));

    let composite_host = host(150.0, 100.0, DARK);
    on_composition(&composite_host, |host, compositor| {
        let factory = compositor.create_effect_factory(CompositeEffect {
            name: "Composite".into(),
            sources: vec![
                ColorSourceEffect { name: "Ground".into(), color: Color::from_rgb(0xF5, 0xC5, 0x42) }.into(),
                CompositionEffectSourceParameter::new("image").into(),
            ],
            ..Default::default()
        });
        let brush = factory.create_brush();
        brush.set_source_parameter("image", Some(checkerboard(compositor)));
        show(host, effect_sprite(compositor, &brush).visual());
    });
    tiles.push((composite_host, "CompositeEffect: the image over a colour source"));

    let color_host = host(150.0, 100.0, DARK);
    on_composition(&color_host, |host, compositor| {
        let factory = compositor.create_effect_factory(ColorSourceEffect {
            name: "Fill".into(),
            color: Color::from_rgb(0x3F, 0x9E, 0x5A),
        });
        let brush = factory.create_brush();
        show(host, effect_sprite(compositor, &brush).visual());
    });
    tiles.push((color_host, "ColorSourceEffect"));

    let grid = WrapPanel::new();
    grid.set_orientation(Orientation::Horizontal);
    for (host, about) in &tiles {
        grid.children().add(tile(host, about));
    }
    let content = column(8.0);
    let (blur_panel, blur_slider) = slider("BlurAmount", (0.0, 12.0), 3.0, move |value| {
        if let Some(blur) = blur.borrow().as_ref() {
            blur.properties().insert_scalar("Blur.BlurAmount", value as f32);
        }
    });
    content.children().add(blur_panel);
    content.children().add(grid);
    let smoke: SmokeAction = Rc::new(move || blur_slider.set_range_value(8.0));
    Demo { content: content.upcast(), smoke }
}
