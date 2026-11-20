//! `Controls/TransitioningContentControl.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   dxaml/xcp/dxaml/themes/generic.xaml (the template of the implicit style of ContentControl)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WinUI has no TransitioningContentControl: a WinUI application animates a change of content
//! with the ContentTransitions of a ContentControl (or of a ContentPresenter), whose template is
//! a ContentPresenter of the content at the padding. FerroUI's TransitioningContentControl runs its
//! PageTransition between two presenters, so this template is that presenter twice, one over the
//! other: PART_ContentPresenter and PART_ContentPresenter2, which the control swaps.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The presenters take the padding, the content alignments, the background and the border, as
//!   FerroUI's ContentControl does (WinUI's template places the presenter at the padding with the
//!   content alignments as its own alignments and draws no background).
//! - The content alignments Left and Top of the implicit style of ContentControl are not set: the
//!   control hosts pages and views that fill it (FerroUI's default, Stretch), as WinUI's Frame
//!   does.
//! - ContentTransitions (empty by default in WinUI) is FerroUI's PageTransition, which keeps the
//!   default of the control (a cross-fade).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Panel;
use ferroui_controls::TransitioningContentControl;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(
        type_key::<TransitioningContentControl>(),
        transitioning_content_control_theme,
    );
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `TransitioningContentControl` of `transitioning_content_control_theme`.
fn transitioning_content_control_theme_template(t: &Template<TransitioningContentControl>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .children([
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
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
                .template_bind(
                    ContentPresenter::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    ContentPresenter::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind(
                    ContentPresenter::horizontal_content_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::vertical_content_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                )
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter2")?
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
                .template_bind(
                    ContentPresenter::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    ContentPresenter::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind(
                    ContentPresenter::horizontal_content_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::vertical_content_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `TransitioningContentControl`.
fn transitioning_content_control_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TransitioningContentControl>();
    theme.template(transitioning_content_control_theme_template);
    Ok(())
}
