//! `Controls/ToolTip.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/ToolTip_themeresources.xaml (the styles; its resources are
//!   Resources/ToolTip_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template part: the ContentPresenter LayoutRoot of the source is FerroUI's
//!   PART_ContentPresenter, the presenter a content control hosts its content in.
//! - OpenStates: Opened runs FadeInThemeAnimation, Closed FadeOutThemeAnimation on LayoutRoot.
//!   The fade is the opacity of the tool tip, 0 by default and 1 under FerroUI's :open, with a
//!   linear transition of 250 ms, the duration of the fade-in animation of the Windows animation
//!   library (the timing of the theme animations comes from the system, not from the sources).
//!   The fade-out is not seen: FerroUI closes the popup of a tool tip at once.
//! - ContentTransitions has no counterpart. The drop shadow WinUI's ToolTip applies to LayoutRoot
//!   in code when it opens (ApplyElevationEffect with a base elevation of 16 under drop shadows,
//!   ToolTip_Partial.cpp) is a ThemeShadow (the shared WinUIThemeShadow, Translation.Z as in WinUI) on the border
//!   ThemeShadowWrapper around PART_ContentPresenter under :open (a border of its own),
//!   with the margin ThemeShadowToolTipMargin as the room for it in the popup
//!   (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::TextWrapping;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ToolTip;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultToolTipStyle", default_tool_tip_style);
    resources.control_theme(type_key::<ToolTip>(), tool_tip_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`: the document included
/// (`ResourceInclude`) rather than merged.
#[allow(dead_code)]
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `ToolTip` of `default_tool_tip_style`.
fn default_tool_tip_style_template(t: &Template<ToolTip>) -> Result<Ref<Control>> {
    // The theme shadow on a border of its own, at the elevation of a tool tip (G-24).
    Ok(t.make(Border::new())
        .name("ThemeShadowWrapper")?
        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,16"))?)
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .template_bind(
                    ContentPresenter::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    ContentPresenter::background_property(),
                    TemplatedControl::background_property(),
                )
                .template_bind(
                    ContentPresenter::background_sizing_property(),
                    TemplatedControl::background_sizing_property(),
                )
                .template_bind(
                    ContentPresenter::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(Layoutable::max_width_property(), Layoutable::max_width_property())
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                .template_bind(
                    ContentPresenter::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                ),
        )
        .control())
}

/// `DefaultToolTipStyle`: a control theme of `ToolTip`.
fn default_tool_tip_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToolTip>();
    // The shadow is drawn outside the bounds of the control: a templated control clips to its
    // bounds unless its theme says otherwise (G-24).
    theme.set(Visual::clip_to_bounds_property(), false);
    theme.dynamic(TemplatedControl::foreground_property(), "ToolTipForegroundBrush");
    theme.dynamic(TemplatedControl::background_property(), "ToolTipBackgroundBrush");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "ToolTipBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ToolTipBorderThemeThickness",
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ToolTipContentThemeFontSize");
    theme.static_resource(TemplatedControl::padding_property(), "ToolTipBorderPadding")?;
    theme.static_resource(Layoutable::max_width_property(), "ToolTipMaxWidth")?;
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // Closed (FadeOutThemeAnimation) and Opened (FadeInThemeAnimation) of OpenStates.
    theme.set(Visual::opacity_property(), 0.0);
    theme.set_value(
        Animatable::transitions_property(),
        value({
            let object = Transitions::new();
            object.add(
                theme
                    .scope()
                    .make(DoubleTransition::new())
                    .set(
                        TransitionBase::property_property(),
                        Some(Visual::opacity_property().property()),
                    )
                    .set(
                        TransitionBase::duration_property(),
                        SignedDuration::from_milliseconds(250.0),
                    )
                    .end()
                    .into(),
            );
            object
        }),
    );
    theme.template(default_tool_tip_style_template);
    // Opened; the shadow of the open tool tip and the room for it in the popup (G-24).
    theme.style(nesting().class(":open"), |style| {
        style.set(Visual::opacity_property(), 1.0);
        style.static_resource(Layoutable::margin_property(), "ThemeShadowToolTipMargin")?;
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":open")
            .template()
            .of::<Border>()
            .name("ThemeShadowWrapper"),
        |style| {
            style.static_resource(Visual::shadow_property(), "WinUIThemeShadow")?;
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `ToolTip`.
fn tool_tip_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ToolTip>();
    theme.based_on("DefaultToolTipStyle")?;
    Ok(())
}
