//! The pages of the section "Composition" of the catalog (not from upstream;
//! `docs/porting/winui-composition.md`, `docs/porting/winui-materials.md`): one page per feature
//! of the port of the WinUI composition API and of the WinUI materials, each with its demo shown
//! at once. The demos are in `winui_composition_page.rs` (connected and implicit animations, the
//! pointer position and the parallax of the property sets), `winui_composition_phases.rs`
//! (visuals, brushes, animations, shadows and lights, the interaction tracker, effect brushes),
//! `winui_composition_phase7.rs` (alpha mask shadows, `ThemeShadow`, zoom) and
//! `winui_materials_page.rs` (acrylic, blend and colour effects, lights, Reveal, system backdrops,
//! menus).
//!
//! The headers here are the ones of the entries of the page list
//! (`ViewModels/main_window_view_model_page_list.rs`).

use super::winui_composition_page::{
    connected_animation_section, implicit_animations_section, parallax_section, pointer_section, section,
};
use super::winui_composition_phase7::{alpha_mask_demo, theme_shadow_demo, zoom_demo};
use super::winui_composition_phases::{
    animations_demo, brushes_demo, effects_demo, interaction_tracker_demo, shadows_and_lights_demo, visuals_demo, Demo,
    SmokeAction,
};
use super::winui_materials_page::{
    acrylic_section, backdrop_section, effects_section, lighting_section, menu_section, reveal_section,
};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::{ContentPage, Control, ScrollViewer, StackPanel};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

const CONNECTED_ANIMATIONS: &str = "Connected animations";
const IMPLICIT_ANIMATIONS: &str = "Implicit animations";
const POINTER_POSITION: &str = "Pointer position";
const PARALLAX: &str = "Parallax";
const VISUALS: &str = "Visuals";
const BRUSHES: &str = "Composition brushes";
const ANIMATIONS: &str = "Spring and key frame animations";
const SHADOWS_AND_LIGHTS: &str = "Shadows and lights";
const INTERACTION_TRACKER: &str = "InteractionTracker";
const EFFECT_BRUSHES: &str = "Effect brushes";
const ALPHA_MASK_SHADOWS: &str = "Alpha mask shadows";
const THEME_SHADOW: &str = "ThemeShadow";
const ZOOM: &str = "Zoom and manipulation";
const ACRYLIC_BRUSH: &str = "Acrylic brush";
const BLEND_EFFECTS: &str = "Blend and colour effects";
const LIGHTING: &str = "Lighting";
const REVEAL: &str = "Reveal";
const SYSTEM_BACKDROPS: &str = "System backdrops";
const ACRYLIC_MENUS: &str = "Acrylic menus";

/// The pages whose demos the smoke run of the desktop entry point shows one after the other
/// (`smoke_show_section` and `smoke_run_section`), by index: the order of the sections of the
/// earlier single page, so that `section-N-<ms>.png` keeps its numbers.
const SMOKE_SECTIONS: [&str; 10] = [
    VISUALS,
    BRUSHES,
    ANIMATIONS,
    SHADOWS_AND_LIGHTS,
    INTERACTION_TRACKER,
    EFFECT_BRUSHES,
    REVEAL,
    ALPHA_MASK_SHADOWS,
    THEME_SHADOW,
    ZOOM,
];

ferroui_base::ferro_thread_local! {
    /// The header of the page with a smoke action last created, and the action. The page holds
    /// the action; the record does not keep the page alive.
    static PAGE_SMOKE: RefCell<Option<(&'static str, Weak<dyn Fn()>)>> = const { RefCell::new(None) };
}

/// How many pages the smoke run shows one after the other (for the smoke run of the desktop
/// entry point).
pub fn smoke_section_count() -> usize {
    SMOKE_SECTIONS.len()
}

/// The header of the page the smoke run shows as section `index`.
pub fn smoke_section_header(index: usize) -> Option<&'static str> {
    SMOKE_SECTIONS.get(index).copied()
}

/// Navigates to the page of section `index` (for the smoke run of the desktop entry point).
/// Returns whether there is such a page; its demo is built when the navigation creates the page,
/// and [`smoke_run_section`] runs it once the page is shown.
pub fn smoke_show_section(index: usize) -> bool {
    smoke_section_header(index).is_some_and(crate::select_page)
}

/// Runs the demo of the page of section `index`, as its buttons do (for the smoke run). Returns
/// whether that page is the one with a smoke action last created and is still alive.
pub fn smoke_run_section(index: usize) -> bool {
    let Some(header) = smoke_section_header(index) else { return false };
    let smoke = PAGE_SMOKE.with(|current| match &*current.borrow() {
        Some((page, smoke)) if *page == header => smoke.upgrade(),
        _ => None,
    });
    let Some(smoke) = smoke else { return false };
    smoke();
    true
}

/// A page of the section: `header`, and `content` (a section: its title, its description and its
/// demo) in a scroll viewer.
fn feature_page(header: &str, content: Ref<StackPanel>) -> Ref<ContentPage> {
    let page = ContentPage::new();
    page.set_header(Some(Rc::new(header.to_owned()) as BoxedValue));
    content.set_margin(Thickness::uniform(16.0));
    content.set_vertical_alignment(VerticalAlignment::Top);
    let scroller = ScrollViewer::new();
    scroller.set_content(Some(Control::boxed(content)));
    page.set_content(Some(Control::boxed(scroller)));
    page
}

/// Records `smoke` as the smoke action of the page `header`; `content` holds it.
fn register_smoke(header: &'static str, content: &Ref<StackPanel>, smoke: SmokeAction) {
    PAGE_SMOKE.with(|current| *current.borrow_mut() = Some((header, Rc::downgrade(&smoke))));
    let _ = content.detached_from_visual_tree(move |_| {
        let _ = &smoke;
    }).detach();
}

/// A page whose demo is built with the page.
fn demo_page(header: &'static str, title: &str, description: &str, build: fn() -> Demo) -> Ref<ContentPage> {
    let demo = build();
    demo.content.set_horizontal_alignment(HorizontalAlignment::Left);
    let content = section(title, description, &demo.content);
    register_smoke(header, &content, demo.smoke);
    feature_page(header, content)
}

// --- the property sets and the animations of ElementCompositionPreview ----------------------

/// The page "Connected animations".
pub fn connected_animations_page() -> Ref<ContentPage> {
    feature_page(CONNECTED_ANIMATIONS, connected_animation_section())
}

/// The page "Implicit animations".
pub fn implicit_animations_page() -> Ref<ContentPage> {
    feature_page(IMPLICIT_ANIMATIONS, implicit_animations_section())
}

/// The page "Pointer position".
pub fn pointer_position_page() -> Ref<ContentPage> {
    feature_page(POINTER_POSITION, pointer_section())
}

/// The page "Parallax".
pub fn parallax_page() -> Ref<ContentPage> {
    feature_page(PARALLAX, parallax_section())
}

// --- the composition API ----------------------------------------------------------------------

/// The page "Visuals".
pub fn visuals_page() -> Ref<ContentPage> {
    demo_page(
        VISUALS,
        "Visuals",
        "SpriteVisual with a CompositionColorBrush; ShapeVisual with rounded rectangle, ellipse and path geometries (the path animates TrimEnd); LayerVisual with an effect over its children; RedirectVisual drawing the visual of another element.",
        visuals_demo,
    )
}

/// The page "Composition brushes".
pub fn composition_brushes_page() -> Ref<ContentPage> {
    demo_page(
        BRUSHES,
        "Brushes",
        "CompositionLinearGradientBrush and CompositionRadialGradientBrush with their WinUI members, CompositionNineGridBrush over a LoadedImageSurface, and CompositionMaskBrush.",
        brushes_demo,
    )
}

/// The page "Spring and key frame animations".
pub fn composition_animations_page() -> Ref<ContentPage> {
    demo_page(
        ANIMATIONS,
        "Animations",
        "SpringScalarNaturalMotionAnimation and SpringVector3NaturalMotionAnimation with DampingRatio and Period; key frame animations with the CompositionEasingFunction curves side by side; a CompositionAnimationGroup whose CompositionScopedBatch reports its completion.",
        animations_demo,
    )
}

/// The page "Shadows and lights".
pub fn shadows_and_lights_page() -> Ref<ContentPage> {
    demo_page(
        SHADOWS_AND_LIGHTS,
        "Shadows and lights",
        "DropShadow with BlurRadius, Offset and Color; AmbientLight, DistantLight, PointLight and SpotLight lighting the visuals of their Targets.",
        shadows_and_lights_demo,
    )
}

/// The page "InteractionTracker".
pub fn interaction_tracker_page() -> Ref<ContentPage> {
    demo_page(
        INTERACTION_TRACKER,
        "InteractionTracker",
        "InteractionTracker with a VisualInteractionSource redirected from the pointer: drag the surface, release to coast in inertia within MinPosition and MaxPosition. An expression animation reads tracker.Position.",
        interaction_tracker_demo,
    )
}

/// The page "Effect brushes".
pub fn effect_brushes_page() -> Ref<ContentPage> {
    demo_page(
        EFFECT_BRUSHES,
        "Effects",
        "CompositionEffectFactory and CompositionEffectBrush for the effects of the port: GaussianBlurEffect, OpacityEffect, CrossFadeEffect, CompositeEffect and ColorSourceEffect, over a LoadedImageSurface. The blend and colour effects of the materials are on the page Blend and colour effects.",
        effects_demo,
    )
}

/// The page "Alpha mask shadows".
pub fn alpha_mask_shadows_page() -> Ref<ContentPage> {
    demo_page(
        ALPHA_MASK_SHADOWS,
        "Shadows of an alpha mask",
        "Shape.GetAlphaMask, TextBlock.GetAlphaMask and Image.GetAlphaMask as the Mask of a DropShadow on a SpriteVisual set with SetElementChildVisual: the shadow follows the alpha of the element, not its rectangle. The first tile is the trace pointer of the Windows Calculator (DropShadow BlurRadius 6, Opacity 0.33, Offset (2, 2, 0)); the slider scales it. The last tile turns a sprite out of its plane: the Z of the Offset of its shadow moves the shadow along the tilt.",
        alpha_mask_demo,
    )
}

/// The page "ThemeShadow".
pub fn theme_shadow_page() -> Ref<ContentPage> {
    demo_page(
        THEME_SHADOW,
        "ThemeShadow",
        "UIElement.Shadow set to a ThemeShadow, the elevation from Translation.Z: 16 (a tool tip), 32 (a flyout), 40 (a submenu), 64 and 128 (a content dialog). An ambient and a directional shadow drawn around the rounded corners, darker in the dark theme, none in high contrast.",
        theme_shadow_demo,
    )
}

/// The page "Zoom and manipulation".
pub fn zoom_page() -> Ref<ContentPage> {
    demo_page(
        ZOOM,
        "Zoom and the manipulation property set",
        "A ScrollViewer with ZoomMode Enabled (Ctrl and the wheel, or the buttons calling ChangeView). The marker over it is a composition visual whose Offset is an expression of the Translation and Scale of GetScrollViewerManipulationPropertySet: it stays on the dot of the content at every zoom and offset.",
        zoom_demo,
    )
}

// --- the materials ----------------------------------------------------------------------------

/// The page "Acrylic brush".
pub fn acrylic_brush_page() -> Ref<ContentPage> {
    feature_page(ACRYLIC_BRUSH, acrylic_section())
}

/// The page "Blend and colour effects".
pub fn blend_effects_page() -> Ref<ContentPage> {
    feature_page(BLEND_EFFECTS, effects_section())
}

/// The page "Lighting".
pub fn lighting_page() -> Ref<ContentPage> {
    feature_page(LIGHTING, lighting_section())
}

/// The page "Reveal": the Reveal brushes lit by the XAML lights of WinUI. Its demo has nothing
/// for the smoke run to press.
pub fn reveal_page() -> Ref<ContentPage> {
    let content = reveal_section();
    register_smoke(REVEAL, &content, Rc::new(|| {}));
    feature_page(REVEAL, content)
}

/// The page "System backdrops".
pub fn system_backdrops_page() -> Ref<ContentPage> {
    feature_page(SYSTEM_BACKDROPS, backdrop_section())
}

/// The page "Acrylic menus".
pub fn acrylic_menus_page() -> Ref<ContentPage> {
    feature_page(ACRYLIC_MENUS, menu_section())
}
