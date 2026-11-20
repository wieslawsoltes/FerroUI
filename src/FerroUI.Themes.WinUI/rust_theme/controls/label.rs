//! `Controls/Label.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   dxaml/xcp/dxaml/themes/generic.xaml (the implicit style of ContentControl)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WinUI has no Label: a WinUI application labels a control with a TextBlock, or with the Header of
//! the control. FerroUI's Label is a content control, so its theme is the default style of
//! ContentControl: the content aligned to the left and the top, presented by a ContentPresenter
//! at the padding. Its text is the text of its container (the font, size and foreground of the
//! window, ContentControlThemeFontFamily, ControlContentThemeFontSize, TextFillColorPrimaryBrush),
//! as for a TextBlock of WinUI.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's ContentControl has a template of its own (the one of this document without the
//!   alignments), so the implicit style of ContentControl is ported here, with Label.
//! - The ContentPresenter is FerroUI's PART_ContentPresenter. WinUI's template places it at the
//!   padding (Margin) with the content alignments as its own alignments and draws neither
//!   background nor border; the presenter here takes the padding, the alignments, the background
//!   and the border, as FerroUI's ContentControl does: the same layout, and the background and
//!   border a FerroUI application may set.
//! - ContentTransitions has no counterpart. RecognizesAccessKey is FerroUI's access key of a
//!   label (the key that focuses its target).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Label;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<Label>(), label_theme);
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

/// The template of `Label` of `label_theme`.
fn label_theme_template(t: &Template<Label>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
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
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .set(ContentPresenter::recognizes_access_key_property(), true)
        .control())
}

/// The default control theme of `Label`.
fn label_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Label>();
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Top,
    );
    theme.template(label_theme_template);
    Ok(())
}
