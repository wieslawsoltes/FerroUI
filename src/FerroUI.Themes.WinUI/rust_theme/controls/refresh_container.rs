//! `Controls/RefreshContainer.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/PullToRefresh/RefreshContainer/RefreshContainer.xaml (the styles; the resources
//!   of controls/dev/PullToRefresh/RefreshContainer/RefreshContainer_themeresources.xaml are
//!   Resources/RefreshContainer_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4): ContentPresenter is
//! PART_ContentPresenter and RefreshVisualizerPresenter PART_RefreshVisualizerPresenter, the parts
//! of FerroUI's RefreshContainer (the control puts its visualizer into the presenter grid). The
//! FerroUI contract: the content presenter also takes the ContentTemplate, the padding, the border
//! and the content alignments of the control, as in the Fluent theme.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::RefreshContainer;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultRefreshContainerStyle", default_refresh_container_style);
    resources.control_theme(type_key::<RefreshContainer>(), refresh_container_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `RefreshContainer` of `default_refresh_container_style`.
fn default_refresh_container_style_template(t: &Template<RefreshContainer>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("Root")?
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind(
                    ContentPresenter::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .set(
                    ContentPresenter::background_sizing_property(),
                    BackgroundSizing::InnerBorderEdge,
                )
                .template_bind(
                    ContentPresenter::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    ContentPresenter::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(
                    ContentPresenter::horizontal_content_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::vertical_content_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                )
                .set(
                    ContentPresenter::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                )
                .control(),
            t.make(Grid::new()).name("PART_RefreshVisualizerPresenter")?.control(),
        ])
        .control())
}

/// `DefaultRefreshContainerStyle`: a control theme of `RefreshContainer`.
fn default_refresh_container_style(theme: &StyleBuilder) -> Result {
    theme.target::<RefreshContainer>();
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "RefreshContainerForegroundBrush",
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "RefreshContainerBackgroundBrush",
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.template(default_refresh_container_style_template);
    Ok(())
}

/// The default control theme of `RefreshContainer`.
fn refresh_container_theme(theme: &StyleBuilder) -> Result {
    theme.target::<RefreshContainer>();
    theme.based_on("DefaultRefreshContainerStyle")?;
    Ok(())
}
