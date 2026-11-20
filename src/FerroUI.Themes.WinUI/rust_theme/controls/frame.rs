//! `Controls/Frame.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   dxaml/xcp/dxaml/themes/generic.xaml (the implicit style of Frame)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The template of WinUI's Frame, which FerroUI ports as a control (ferroui-controls,
//! docs/porting/winui-controls.md, section 7); the Fluent theme has no document for it. The pages of
//! a frame (FramePage, WinUI's Page) are user controls with the theme of UserControl: WinUI's Page
//! has no style.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The ContentPresenter of the template is PART_ContentPresenter: FerroUI's Frame is a
//!   ContentControl, whose content is hosted by the presenter of that name.
//! - ContentTransitions (the NavigationThemeTransition of a navigation) has no counterpart and is
//!   left out: FerroUI's frame has no navigation transitions.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Frame;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<Frame>(), frame_theme);
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

/// The template of `Frame` of `frame_theme`.
fn frame_theme_template(t: &Template<Frame>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
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
        .control())
}

/// The default control theme of `Frame`.
fn frame_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Frame>();
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Stretch,
    );
    theme.template(frame_theme_template);
    Ok(())
}
