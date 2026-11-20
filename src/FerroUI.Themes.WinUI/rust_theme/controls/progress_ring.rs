//! `Controls/ProgressRing.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/ProgressRing/ProgressRing.xaml (the implicit ProgressRing style; the resources of
//!   ProgressRing_themeresources.xaml are Resources/ProgressRing_themeresources.xaml)
//!   controls/dev/ProgressRing/AnimatedVisuals/ProgressRingIndeterminate.cpp (the geometry of the
//!   ring)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The template of WinUI's ProgressRing, which FerroUI ports as a control (ferroui-controls,
//! docs/porting/winui-controls.md); the Fluent theme has no document for it.
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The AnimatedVisualPlayer LottiePlayer of WinUI plays the Lottie compositions of the control
//!   (ProgressRingIndeterminate, ProgressRingDeterminate) with the Foreground and the Background of
//!   the ring. FerroUI has no player: LottiePlayer is a Viewbox (Stretch Fill, as the player) of the
//!   80 x 80 shape visual of the indeterminate composition, its ellipse of radius 35 drawn with a
//!   stroke of 7.5 with round caps: the track, PART_Track, in the Background, and the arc of the
//!   progress, PART_Indicator, in the Foreground, whose angles the control moves as WinUI plays the
//!   compositions. The determinate composition (a ring of radius 14.16 and stroke 2.655 in 32 x 32)
//!   is drawn with the same geometry.
//! - CommonStates are the pseudo-classes the control sets (ProgressRing.cpp calls GoToState):
//!   Active is :active, DeterminateActive is :determinate-active, Inactive is :inactive. The
//!   AccessibilityView of LottiePlayer in the state Inactive is left out (the control sets the view
//!   of the ring).
//! - IsTabStop is IsTabStop.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::PenLineCap;
use ferroui_base::media::Stretch;
use ferroui_base::Visual;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Arc;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Control;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::ProgressRing;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<ProgressRing>(), progress_ring_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `ProgressRing` of `progress_ring_theme`.
fn progress_ring_theme_template(t: &Template<ProgressRing>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("LayoutRoot")?
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        // The AnimatedVisualPlayer for Lottie.
        .children([t
            .make(Viewbox::new())
            .name("LottiePlayer")?
            .set(Viewbox::stretch_property(), Stretch::Fill)
            .child(
                Viewbox::child_property(),
                t.make(Grid::new())
                    .set(Layoutable::width_property(), 80.0)
                    .set(Layoutable::height_property(), 80.0)
                    .children([
                        t.make(Arc::new())
                            .name("PART_Track")?
                            .set(Layoutable::width_property(), 77.5)
                            .set(Layoutable::height_property(), 77.5)
                            .set(Arc::start_angle_property(), -90.0)
                            .set(Arc::sweep_angle_property(), 360.0)
                            .template_bind(Shape::stroke_property(), TemplatedControl::background_property())
                            .set(Shape::stroke_thickness_property(), 7.5)
                            .set(Shape::stroke_line_cap_property(), PenLineCap::Round)
                            .control(),
                        t.make(Arc::new())
                            .name("PART_Indicator")?
                            .set(Layoutable::width_property(), 77.5)
                            .set(Layoutable::height_property(), 77.5)
                            .set(Arc::start_angle_property(), -90.0)
                            .set(Arc::sweep_angle_property(), 0.0)
                            .template_bind(Shape::stroke_property(), TemplatedControl::foreground_property())
                            .set(Shape::stroke_thickness_property(), 7.5)
                            .set(Shape::stroke_line_cap_property(), PenLineCap::Round)
                            .control(),
                    ]),
            )
            .control()])
        .control())
}

/// The default control theme of `ProgressRing`.
fn progress_ring_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ProgressRing>();
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "ProgressRingForegroundThemeBrush",
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "ProgressRingBackgroundThemeBrush",
    );
    theme.set(InputElement::is_hit_test_visible_property(), false);
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.set(Layoutable::min_height_property(), 16.0);
    theme.set(Layoutable::min_width_property(), 16.0);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(Layoutable::width_property(), 32.0);
    theme.set(Layoutable::height_property(), 32.0);
    theme.set(ProgressRing::maximum_property(), 100.0);
    theme.template(progress_ring_theme_template);
    // CommonStates: Inactive (DeterminateActive and Active set nothing).
    theme.style(
        nesting().class(":inactive").template().of::<Grid>().name("LayoutRoot"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    Ok(())
}
