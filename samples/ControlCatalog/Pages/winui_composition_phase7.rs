//! The sections of the page "Composition (WinUI)" for phase 7 of the port (not from upstream;
//! `docs/porting/winui-composition.md`): the shadow of an alpha mask (`GetAlphaMask` of a shape,
//! a text block and an image as the `Mask` of a `DropShadow`, with the trace pointer of the
//! Windows Calculator), the `Offset.Z` of a drop shadow, `ThemeShadow` at the elevations of the
//! WinUI controls in both themes, and the zoom of a scroll viewer driving visuals through its
//! manipulation property set.

use super::winui_composition_page::{brush, compositor_of, text};
use super::winui_composition_phases::{lazy_section, Demo, SmokeAction};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Color, FontWeight, Geometry, IImage, ScaleTransform, Shadow, ThemeShadow};
use ferroui_base::numerics::{Quaternion, Vector3};
use ferroui_base::platform::{AlphaFormat, PixelFormats};
use ferroui_base::rendering::composition::{
    CompositionVisual, Compositor, DropShadow, ICompositionObjectAnimations, SpriteVisual,
};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, CornerRadius, PixelSize, Ref, Thickness, Vector, Vector3D, Visual};
use ferroui_controls::hosting::ElementCompositionPreview;
use ferroui_controls::shapes::Path;
use ferroui_controls::{
    Border, Button, Canvas, Control, Image, Panel, ScrollViewer, Slider, StackPanel, TextBlock, ThemeVariantScope,
    WrapPanel, ZoomMode,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The sections of phase 7, for the page.
pub fn phase7_sections() -> Vec<Ref<StackPanel>> {
    vec![
        lazy_section(
            "Shadows of an alpha mask",
            "Shape.GetAlphaMask, TextBlock.GetAlphaMask and Image.GetAlphaMask as the Mask of a DropShadow on a SpriteVisual set with SetElementChildVisual: the shadow follows the alpha of the element, not its rectangle. The first tile is the trace pointer of the Windows Calculator (DropShadow BlurRadius 6, Opacity 0.33, Offset (2, 2, 0)); the slider scales it. The last tile turns a sprite out of its plane: the Z of the Offset of its shadow moves the shadow along the tilt.",
            alpha_mask_demo,
        ),
        lazy_section(
            "ThemeShadow",
            "UIElement.Shadow set to a ThemeShadow, the elevation from Translation.Z: 16 (a tool tip), 32 (a flyout), 40 (a submenu), 64 and 128 (a content dialog). An ambient and a directional shadow drawn around the rounded corners, darker in the dark theme, none in high contrast.",
            theme_shadow_demo,
        ),
        lazy_section(
            "Zoom and the manipulation property set",
            "A ScrollViewer with ZoomMode Enabled (Ctrl and the wheel, or the buttons calling ChangeView). The marker over it is a composition visual whose Offset is an expression of the Translation and Scale of GetScrollViewerManipulationPropertySet: it stays on the dot of the content at every zoom and offset.",
            zoom_demo,
        ),
    ]
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

/// A tile: a surface of the given size and a caption under it.
fn tile(content: &Control, width: f64, about: &str) -> Ref<StackPanel> {
    let tile = column(6.0);
    tile.set_margin(Thickness::new(0.0, 0.0, 16.0, 16.0));
    tile.children().add(content.to_ref());
    let about = caption(about);
    about.set_max_width(width);
    about.set_horizontal_alignment(HorizontalAlignment::Left);
    tile.children().add(about);
    tile
}

/// A light surface of the given size, holding a canvas.
fn surface(width: f64, height: f64) -> (Ref<Border>, Ref<Canvas>) {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border.set_corner_radius(CornerRadius::uniform(8.0));
    border.set_clip_to_bounds(true);
    border.set_horizontal_alignment(HorizontalAlignment::Left);
    border.set_vertical_alignment(VerticalAlignment::Top);
    border.set_background(Some(brush(Color::from_rgb(0xEE, 0xF0, 0xF4))));
    let canvas = Canvas::new();
    border.set_child(canvas.clone());
    (border, canvas)
}

/// Runs `build` once `element` is in the tree and has its composition visual, on every attach.
fn on_composition<T: ferroui_base::ObjectType + ferroui_base::Upcast<Visual>>(
    element: &Ref<T>,
    build: impl Fn(&Rc<Compositor>) + 'static,
) {
    let build = Rc::new(build);
    let weak = element.clone().upcast::<Visual>().downgrade();
    let _ = element.attached_to_visual_tree(move |_| {
        let (weak, build) = (weak.clone(), build.clone());
        let _ = ferroui_base::threading::Dispatcher::ui_thread().invoke_async_local_with_priority(
            move || {
                let Some(element) = weak.upgrade() else { return };
                let Some(compositor) = compositor_of(&element) else { return };
                build(&compositor);
            },
            ferroui_base::threading::DispatcherPriority::BACKGROUND,
        );
    });
}

/// `AddTracePointerShadow` of the Windows Calculator
/// (`Views/GraphingCalculator/GraphingCalculator.xaml.cs`): a drop shadow of the alpha of the
/// cursor path, on a sprite visual shown by the element under the path.
fn add_trace_pointer_shadow(compositor: &Rc<Compositor>, cursor_path: &Path, cursor_shadow: &Visual) -> Option<Rc<DropShadow>> {
    let mask = cursor_path.get_alpha_mask()?;
    let shadow = compositor.create_drop_shadow();
    shadow.set_blur_radius(6.0);
    shadow.set_opacity(0.33);
    shadow.set_offset(Vector3D::new(2.0, 2.0, 0.0));
    shadow.set_mask(Some(mask));
    let sprite = compositor.create_sprite_visual();
    let bounds = cursor_path.bounds();
    sprite.set_size(Vector::new(bounds.width, bounds.height));
    sprite.set_shadow(Some(shadow.clone()));
    ElementCompositionPreview::set_element_child_visual(cursor_shadow, Some((*sprite).clone()));
    Some(shadow)
}

/// The drop shadow of `mask` on a sprite of `size` shown by `host`.
fn masked_shadow(
    compositor: &Rc<Compositor>,
    host: &Visual,
    mask: Rc<ferroui_base::rendering::composition::CompositionBrush>,
    size: Vector,
    blur: f32,
    offset: Vector3D,
    color: Color,
) -> SpriteVisual {
    let shadow = compositor.create_drop_shadow();
    shadow.set_blur_radius(blur);
    shadow.set_offset(offset);
    shadow.set_color(color);
    shadow.set_mask(Some(mask));
    let sprite = compositor.create_sprite_visual();
    sprite.set_size(size);
    sprite.set_shadow(Some(shadow));
    ElementCompositionPreview::set_element_child_visual(host, Some((*sprite).clone()));
    sprite
}

/// An element under `element` in `canvas`, at the same place and size, that shows the shadow
/// visual (as `CursorShadow` under `CursorPath` in the Calculator).
fn shadow_host(canvas: &Canvas, left: f64, top: f64, width: f64, height: f64) -> Ref<Border> {
    let host = Border::new();
    host.set_width(width);
    host.set_height(height);
    Canvas::set_left(&host, left);
    Canvas::set_top(&host, top);
    canvas.children().add(host.clone());
    host
}

/// The cursor of the Calculator's trace pointer: an arrow, white with a dark outline.
const CURSOR: &str = "M 1,1 L 1,17 L 5,13 L 8,19 L 10.5,18 L 7.5,12 L 13,12 Z";
/// The second shape of the demo's button: a ring.
const RING: &str = "M 7,1 A 6,6 0 1 1 6.99,1 Z M 7,4 A 3,3 0 1 0 7.01,4 Z";

/// A 64 x 64 image of a filled circle on a transparent ground (premultiplied).
fn circle_bitmap() -> Bitmap {
    let size = 64;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f64 - 31.5, y as f64 - 31.5);
            let coverage = (30.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let alpha = (coverage * 255.0) as u8;
            let scale = |channel: u8| ((channel as f64) * coverage) as u8;
            data.extend_from_slice(&[scale(0xE8), scale(0x5D), scale(0x3A), alpha]);
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

// --- the shadows of alpha masks --------------------------------------------------------------

fn alpha_mask_demo() -> Demo {
    let tiles = WrapPanel::new();
    tiles.set_orientation(Orientation::Horizontal);

    // The trace pointer of the Calculator, scaled by the slider (a render transform of the canvas:
    // the mask is drawn at the scale of the path in the window).
    let (pointer_surface, pointer_canvas) = surface(220.0, 160.0);
    let scaled = Canvas::new();
    scaled.set_width(220.0);
    scaled.set_height(160.0);
    let scale = ScaleTransform::with_scale(4.0, 4.0);
    scaled.set_render_transform(Some((&scale).into()));
    scaled.set_render_transform_origin(ferroui_base::RelativePoint::new(0.0, 0.0, ferroui_base::RelativeUnit::Absolute));
    pointer_canvas.children().add(scaled.clone());
    let cursor_shadow = shadow_host(&scaled, 20.0, 10.0, 14.0, 20.0);
    let cursor_path = Path::new();
    cursor_path.set_data(Geometry::parse(CURSOR).ok());
    cursor_path.set_fill(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
    cursor_path.set_stroke(Some(brush(Color::from_rgb(0x20, 0x24, 0x30))));
    cursor_path.set_stroke_thickness(1.0);
    cursor_path.set_width(14.0);
    cursor_path.set_height(20.0);
    Canvas::set_left(&cursor_path, 20.0);
    Canvas::set_top(&cursor_path, 10.0);
    scaled.children().add(cursor_path.clone());
    {
        let (path, host) = (cursor_path.clone(), cursor_shadow.clone());
        on_composition(&cursor_path, move |compositor| {
            add_trace_pointer_shadow(compositor, &path, &host);
        });
    }
    let pointer_controls = column(6.0);
    pointer_controls.children().add(pointer_surface);
    let scale_label = caption("Scale: 4.0");
    let scale_slider = Slider::new();
    scale_slider.set_minimum(1.0);
    scale_slider.set_maximum(6.0);
    scale_slider.set_range_value(4.0);
    scale_slider.set_width(200.0);
    scale_slider.set_horizontal_alignment(HorizontalAlignment::Left);
    {
        let (scale, scale_label) = (scale.clone(), scale_label.downgrade());
        let _ = scale_slider.value_changed(move |_, e| {
            scale.set_scale_x(e.new_value());
            scale.set_scale_y(e.new_value());
            if let Some(scale_label) = scale_label.upgrade() {
                scale_label.set_text(Some(&format!("Scale: {:.1}", e.new_value())));
            }
        });
    }
    pointer_controls.children().add(scale_label);
    pointer_controls.children().add(scale_slider.clone());
    let shape_toggle = {
        let path = cursor_path.downgrade();
        let ring = RefCell::new(false);
        button("Change the shape", move || {
            let Some(path) = path.upgrade() else { return };
            let next = !*ring.borrow();
            *ring.borrow_mut() = next;
            path.set_data(Geometry::parse(if next { RING } else { CURSOR }).ok());
        })
    };
    pointer_controls.children().add(shape_toggle.clone());
    tiles.children().add(tile(
        &pointer_controls,
        220.0,
        "Path.GetAlphaMask as the mask of the trace pointer's shadow; the button changes the geometry and the mask follows",
    ));

    // A text block.
    let (text_surface, text_canvas) = surface(220.0, 160.0);
    let text_shadow_host = shadow_host(&text_canvas, 16.0, 40.0, 190.0, 80.0);
    let headline = text("Alpha", 64.0, FontWeight::Bold);
    headline.set_foreground(Some(brush(Color::from_rgb(0x2E, 0x86, 0xDE))));
    headline.set_text_wrapping(ferroui_base::media::TextWrapping::NoWrap);
    Canvas::set_left(&headline, 16.0);
    Canvas::set_top(&headline, 40.0);
    headline.set_width(190.0);
    headline.set_height(80.0);
    text_canvas.children().add(headline.clone());
    {
        let (headline, host) = (headline.clone(), text_shadow_host.clone());
        on_composition(&headline, move |compositor| {
            let Some(mask) = headline.get_alpha_mask() else { return };
            let bounds = headline.bounds();
            masked_shadow(
                compositor,
                &host,
                mask,
                Vector::new(bounds.width, bounds.height),
                10.0,
                Vector3D::new(4.0, 6.0, 0.0),
                Color::from_argb(0x90, 0, 0, 0),
            );
        });
    }
    tiles.children().add(tile(&text_surface, 220.0, "TextBlock.GetAlphaMask: the shadow of the glyphs"));

    // An image of a circle on a transparent ground.
    let (image_surface, image_canvas) = surface(160.0, 160.0);
    let image_shadow_host = shadow_host(&image_canvas, 40.0, 40.0, 64.0, 64.0);
    let image = Image::new();
    image.set_source(Some(Rc::new(circle_bitmap()) as Rc<dyn IImage>));
    image.set_width(64.0);
    image.set_height(64.0);
    Canvas::set_left(&image, 40.0);
    Canvas::set_top(&image, 40.0);
    image_canvas.children().add(image.clone());
    {
        let (image, host) = (image.clone(), image_shadow_host.clone());
        on_composition(&image, move |compositor| {
            let Some(mask) = image.get_alpha_mask() else { return };
            masked_shadow(
                compositor,
                &host,
                mask,
                Vector::new(64.0, 64.0),
                12.0,
                Vector3D::new(8.0, 8.0, 0.0),
                Color::from_argb(0xA0, 0x10, 0x20, 0x60),
            );
        });
    }
    tiles.children().add(tile(&image_surface, 160.0, "Image.GetAlphaMask: the shadow of a round image"));

    // Offset.Z on a sprite turned about the X axis.
    let (tilt_surface, _tilt_canvas) = surface(220.0, 160.0);
    let tilted: Rc<RefCell<Option<SpriteVisual>>> = Rc::new(RefCell::new(None));
    {
        let tilted = tilted.clone();
        let host = tilt_surface.clone();
        on_composition(&tilt_surface, move |compositor| {
            let sprite = compositor.create_sprite_visual();
            sprite.set_size(Vector::new(120.0, 70.0));
            sprite.set_offset(Vector3D::new(50.0, 45.0, 0.0));
            sprite.set_center_point(Vector3D::new(60.0, 35.0, 0.0));
            sprite.set_brush(Some((*compositor.create_color_brush_with_color(Color::from_rgb(0x3F, 0x9E, 0x5A))).clone()));
            let shadow = compositor.create_drop_shadow();
            shadow.set_blur_radius(8.0);
            shadow.set_offset(Vector3D::new(0.0, 0.0, 30.0));
            shadow.set_color(Color::from_argb(0xA0, 0, 0, 0));
            sprite.set_shadow(Some(shadow));
            sprite.set_orientation(Quaternion::create_from_axis_angle(Vector3::new(1.0, 0.0, 0.0), 0.9));
            ElementCompositionPreview::set_element_child_visual(&host, Some((*sprite).clone()));
            *tilted.borrow_mut() = Some(sprite);
        });
    }
    let tilt_controls = column(6.0);
    tilt_controls.children().add(tilt_surface);
    let tilt_slider = Slider::new();
    tilt_slider.set_minimum(-1.2);
    tilt_slider.set_maximum(1.2);
    tilt_slider.set_range_value(0.9);
    tilt_slider.set_width(200.0);
    tilt_slider.set_horizontal_alignment(HorizontalAlignment::Left);
    {
        let tilted = tilted.clone();
        let _ = tilt_slider.value_changed(move |_, e| {
            if let Some(sprite) = tilted.borrow().as_ref() {
                sprite.set_orientation(Quaternion::create_from_axis_angle(Vector3::new(1.0, 0.0, 0.0), e.new_value() as f32));
            }
        });
    }
    tilt_controls.children().add(tilt_slider.clone());
    tiles.children().add(tile(
        &tilt_controls,
        220.0,
        "DropShadow.Offset (0, 0, 30) under a sprite turned about the X axis (the slider): the shadow plane lies behind the sprite",
    ));

    let smoke: SmokeAction = Rc::new(move || {
        scale_slider.set_range_value(2.5);
        tilt_slider.set_range_value(-0.9);
    });
    Demo { content: tiles.upcast(), smoke }
}

// --- ThemeShadow ---------------------------------------------------------------------------

fn theme_shadow_demo() -> Demo {
    let scope = ThemeVariantScope::new();
    scope.set_requested_theme_variant(Some(ThemeVariant::light()));
    let ground = Border::new();
    ground.set_padding(Thickness::new(24.0, 16.0, 24.0, 40.0));
    ground.set_corner_radius(CornerRadius::uniform(8.0));
    ground.set_horizontal_alignment(HorizontalAlignment::Left);
    ground.set_background(Some(brush(Color::from_rgb(0xF3, 0xF3, 0xF3))));
    let cards = WrapPanel::new();
    cards.set_orientation(Orientation::Horizontal);
    let mut card_borders = Vec::new();
    for (z, name) in [(16.0, "16: tool tip"), (32.0, "32: flyout"), (40.0, "40: submenu"), (64.0, "64"), (128.0, "128: dialog")] {
        let card = Border::new();
        card.set_width(120.0);
        card.set_height(80.0);
        card.set_margin(Thickness::new(0.0, 0.0, 40.0, 48.0));
        card.set_corner_radius(CornerRadius::uniform(8.0));
        card.set_background(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
        card.set_translation(Vector3D::new(0.0, 0.0, z));
        let shadow: Ref<Shadow> = ThemeShadow::new().upcast();
        card.set_shadow(Some(shadow));
        let title = text(&format!("Translation.Z {name}"), 12.0, FontWeight::Normal);
        title.set_margin(Thickness::uniform(10.0));
        title.set_foreground(Some(brush(Color::from_rgb(0x20, 0x20, 0x20))));
        card.set_child(title);
        cards.children().add(card.clone());
        card_borders.push(card);
    }
    ground.set_child(cards);
    scope.set_child(ground.clone());

    let dark = Rc::new(RefCell::new(false));
    let toggle = {
        let (scope, ground, dark) = (scope.downgrade(), ground.downgrade(), dark.clone());
        let cards: Vec<_> = card_borders.iter().map(|card| card.downgrade()).collect();
        move || {
            let next = !*dark.borrow();
            *dark.borrow_mut() = next;
            if let Some(scope) = scope.upgrade() {
                scope.set_requested_theme_variant(Some(if next { ThemeVariant::dark() } else { ThemeVariant::light() }));
            }
            if let Some(ground) = ground.upgrade() {
                ground.set_background(Some(brush(if next { Color::from_rgb(0x20, 0x20, 0x20) } else { Color::from_rgb(0xF3, 0xF3, 0xF3) })));
            }
            for card in cards.iter().filter_map(|card| card.upgrade()) {
                card.set_background(Some(brush(if next { Color::from_rgb(0x2C, 0x2C, 0x2C) } else { Color::from_rgb(0xFF, 0xFF, 0xFF) })));
                if let Some(title) = card.child().and_then(|child| child.cast::<TextBlock>()) {
                    title.set_foreground(Some(brush(if next { Color::from_rgb(0xF0, 0xF0, 0xF0) } else { Color::from_rgb(0x20, 0x20, 0x20) })));
                }
            }
        }
    };
    let toggle = Rc::new(toggle);
    let content = column(10.0);
    {
        let toggle = toggle.clone();
        content.children().add(button("Light / dark theme", move || toggle()));
    }
    content.children().add(scope);
    let smoke: SmokeAction = Rc::new(move || toggle());
    Demo { content: content.upcast(), smoke }
}

// --- zoom ----------------------------------------------------------------------------------

/// The point of the content the marker follows.
const MARKED: (f64, f64) = (300.0, 200.0);

fn zoom_demo() -> Demo {
    let content = Canvas::new();
    content.set_width(600.0);
    content.set_height(400.0);
    content.set_background(Some(brush(Color::from_rgb(0xF6, 0xF7, 0xFA))));
    let colors = [(0x2E, 0x86, 0xDE), (0xE8, 0x5D, 0x3A), (0x3F, 0x9E, 0x5A), (0xD9, 0xA4, 0x41), (0x9B, 0x59, 0xB6)];
    for index in 0..15 {
        let (r, g, b) = colors[index % colors.len()];
        let tile = Border::new();
        tile.set_width(100.0);
        tile.set_height(70.0);
        tile.set_corner_radius(CornerRadius::uniform(6.0));
        tile.set_background(Some(brush(Color::from_rgb(r, g, b))));
        Canvas::set_left(&tile, 20.0 + (index % 5) as f64 * 115.0);
        Canvas::set_top(&tile, 20.0 + (index / 5) as f64 * 125.0);
        let name = text(&format!("Tile {}", index + 1), 13.0, FontWeight::SemiBold);
        name.set_margin(Thickness::uniform(8.0));
        name.set_foreground(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
        tile.set_child(name);
        content.children().add(tile);
    }
    let dot = Border::new();
    dot.set_width(12.0);
    dot.set_height(12.0);
    dot.set_corner_radius(CornerRadius::uniform(6.0));
    dot.set_background(Some(brush(Color::from_rgb(0x10, 0x10, 0x10))));
    Canvas::set_left(&dot, MARKED.0 - 6.0);
    Canvas::set_top(&dot, MARKED.1 - 6.0);
    content.children().add(dot);

    let scroller = ScrollViewer::new();
    scroller.set_width(420.0);
    scroller.set_height(260.0);
    scroller.set_zoom_mode(ZoomMode::Enabled);
    scroller.set_min_zoom_factor(0.5);
    scroller.set_max_zoom_factor(4.0);
    scroller.set_horizontal_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Auto);
    scroller.set_vertical_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Auto);
    scroller.set_content(Some(Control::boxed(content)));

    // The overlay over the scroll viewer that shows the marker visual.
    let overlay = Border::new();
    overlay.set_is_hit_test_visible(false);
    overlay.set_clip_to_bounds(true);
    let layered = Panel::new();
    layered.set_width(420.0);
    layered.set_height(260.0);
    layered.set_horizontal_alignment(HorizontalAlignment::Left);
    layered.children().add(scroller.clone());
    layered.children().add(overlay.clone());

    let marker: Rc<RefCell<Option<Rc<CompositionVisual>>>> = Rc::new(RefCell::new(None));
    {
        let (scroller, overlay, marker) = (scroller.downgrade(), overlay.downgrade(), marker.clone());
        on_composition(&layered, move |compositor| {
            let (Some(scroller), Some(overlay)) = (scroller.upgrade(), overlay.upgrade()) else { return };
            let Some(set) = ElementCompositionPreview::get_scroll_viewer_manipulation_property_set(&scroller) else { return };
            let ring = compositor.create_sprite_visual();
            ring.set_size(Vector::new(28.0, 28.0));
            ring.set_brush(Some((*compositor.create_color_brush_with_color(Color::from_argb(0x60, 0xE8, 0x5D, 0x3A))).clone()));
            let follow = compositor.create_expression_animation_with(&format!(
                "Vector3({} * s.Scale.X + s.Translation.X - 14, {} * s.Scale.Y + s.Translation.Y - 14, 0)",
                MARKED.0, MARKED.1
            ));
            follow.set_reference_parameter("s", set.clone());
            ring.start_animation("Offset", &*follow);
            // The ring grows with the zoom too.
            let grow = compositor.create_expression_animation_with("Vector3(s.Scale.X, s.Scale.Y, 1)");
            grow.set_reference_parameter("s", set);
            ring.set_center_point(Vector3D::new(14.0, 14.0, 0.0));
            ring.start_animation("Scale", &*grow);
            ElementCompositionPreview::set_element_child_visual(&overlay, Some((*ring).clone()));
            *marker.borrow_mut() = Some((*ring).clone());
        });
    }

    let zoom_label = caption("ZoomFactor: 1.00");
    {
        let zoom_label = zoom_label.downgrade();
        let _ = scroller.property_changed(move |e| {
            if e.property() != ScrollViewer::zoom_factor_property().as_property() {
                return;
            }
            let (Some(zoom_label), Some(scroller)) = (zoom_label.upgrade(), e.sender().downcast_ref::<ScrollViewer>())
            else {
                return;
            };
            zoom_label.set_text(Some(&format!("ZoomFactor: {:.2}", scroller.zoom_factor())));
        });
    }
    let buttons = row(6.0);
    for factor in [0.5f32, 1.0, 2.0, 3.0] {
        let scroller = scroller.downgrade();
        buttons.children().add(button(&format!("ChangeView zoom {factor}"), move || {
            if let Some(scroller) = scroller.upgrade() {
                scroller.change_view(None, None, Some(factor));
            }
        }));
    }
    let panel = column(8.0);
    panel.children().add(layered);
    panel.children().add(zoom_label);
    panel.children().add(buttons);
    let smoke_scroller = scroller.downgrade();
    let smoke: SmokeAction = Rc::new(move || {
        if let Some(scroller) = smoke_scroller.upgrade() {
            scroller.change_view(Some(260.0), Some(170.0), Some(2.0));
        }
    });
    let _ = marker;
    Demo { content: panel.upcast(), smoke }
}
