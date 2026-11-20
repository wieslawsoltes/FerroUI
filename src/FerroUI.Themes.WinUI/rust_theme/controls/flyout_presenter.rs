//! `Controls/FlyoutPresenter.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/FlyoutPresenter_themeresources.xaml (the styles; its resources are
//!   Resources/FlyoutPresenter_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts: the ContentPresenter of the source is FerroUI's PART_ContentPresenter, which
//!   hosts the content of the flyout. The ScrollViewer keeps its name.
//! - The setters of the scroll and zoom modes (ScrollViewer.HorizontalScrollMode,
//!   ScrollViewer.VerticalScrollMode, ScrollViewer.ZoomMode) and the ContentTransitions binding
//!   have no counterpart and are left out. The scroll bar visibilities are Auto both ways, as in the
//!   source: content wider than the flyout (text that does not wrap in the unbounded width of the
//!   scroll viewer) scrolls horizontally, and the bar is hidden until the pointer is over the
//!   flyout (the indicator states of Controls/ScrollViewer.xaml), as WinUI's NoIndicator.
//! - The shadow WinUI's FlyoutPresenter gives its child in code (ApplyElevationEffect,
//!   FlyoutPresenter_partial.cpp: a ThemeShadow at a Translation.Z of 32) is a ThemeShadow (the shared
//!   WinUIThemeShadow, Translation.Z as in WinUI) on the border ThemeShadowWrapper around the border of the
//!   presenter (a border of its own), and the margin ThemeShadowFlyoutMargin is the room for it in the
//!   popup (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24). The opening animation
//!   (PopupThemeTransition) is drawn by neither (DEVIATIONS.md, FlyoutPresenter).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::FlyoutPresenter;
use ferroui_controls::ScrollViewer;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultFlyoutPresenterStyle", default_flyout_presenter_style);
    resources.control_theme(type_key::<FlyoutPresenter>(), flyout_presenter_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `FlyoutPresenter` of `default_flyout_presenter_style`.
fn default_flyout_presenter_style_template(t: &Template<FlyoutPresenter>) -> Result<Ref<Control>> {
    // The theme shadow on a border of its own (G-24).
    Ok(t.make(Border::new())
        .name("ThemeShadowWrapper")?
        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
        .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                .child(
                    Decorator::child_property(),
                    t.make(ScrollViewer::new())
                        .name("ScrollViewer")?
                        .template_bind(
                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                        )
                        .template_bind(
                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                        )
                        .content(
                            ContentControl::content_property(),
                            t.make(ContentPresenter::new())
                                .name("PART_ContentPresenter")?
                                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    ContentControl::content_template_property(),
                                )
                                .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                                .template_bind(
                                    Layoutable::horizontal_alignment_property(),
                                    ContentControl::horizontal_content_alignment_property(),
                                )
                                .template_bind(
                                    Layoutable::vertical_alignment_property(),
                                    ContentControl::vertical_content_alignment_property(),
                                ),
                        )?,
                ),
        )
        .control())
}

/// `DefaultFlyoutPresenterStyle`: a control theme of `FlyoutPresenter`.
fn default_flyout_presenter_style(theme: &StyleBuilder) -> Result {
    theme.target::<FlyoutPresenter>();
    // The shadow is drawn outside the bounds of the control: a templated control clips to its
    //         bounds unless its theme says otherwise (G-24).
    theme.set(Visual::clip_to_bounds_property(), false);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Stretch,
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::background_property(), "FlyoutPresenterBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "FlyoutBorderThemeBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "FlyoutBorderThemeThickness",
    );
    theme.static_resource(TemplatedControl::padding_property(), "FlyoutContentPadding")?;
    theme.dynamic(Layoutable::min_width_property(), "FlyoutThemeMinWidth");
    theme.dynamic(Layoutable::max_width_property(), "FlyoutThemeMaxWidth");
    theme.dynamic(Layoutable::min_height_property(), "FlyoutThemeMinHeight");
    theme.dynamic(Layoutable::max_height_property(), "FlyoutThemeMaxHeight");
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    // The room for the shadow in the popup (G-24).
    theme.static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?;
    theme.template(default_flyout_presenter_style_template);
    Ok(())
}

/// The default control theme of `FlyoutPresenter`.
fn flyout_presenter_theme(theme: &StyleBuilder) -> Result {
    theme.target::<FlyoutPresenter>();
    theme.based_on("DefaultFlyoutPresenterStyle")?;
    Ok(())
}
