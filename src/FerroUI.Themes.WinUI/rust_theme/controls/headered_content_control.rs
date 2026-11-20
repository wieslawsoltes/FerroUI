//! `Controls/HeaderedContentControl.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/TextBox_themeresources.xaml (the header of DefaultTextBoxStyle:
//!   HeaderContentPresenter and its Disabled state)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WinUI has no HeaderedContentControl: the controls of WinUI with a Header (TextBox, ComboBox,
//! ToggleSwitch, the pickers) show it with the same presenter above the control. This theme is
//! that presenter above the content: HeaderContentPresenter of DefaultTextBoxStyle, in
//! TextControlHeaderForeground, with the bottom margin TextBoxTopHeaderMargin, wrapping, shown
//! only when there is a header (WinUI's controls collapse it without one), and
//! TextControlHeaderForegroundDisabled in the Disabled state.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The header presenter is FerroUI's PART_HeaderPresenter, the content presenter
//!   PART_ContentPresenter. x:DeferLoadStrategy has no counterpart. The header takes the
//!   horizontal and vertical content alignments of the control, as the content does.
//! - The border around both takes the Background, BorderBrush, BorderThickness, CornerRadius and
//!   Padding of the control (none set by the theme), as for FerroUI's other themes.
//! - RecognizesAccessKey is FerroUI's access key support.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextWrapping;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::HeaderedContentControl;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::RowDefinitions;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<HeaderedContentControl>(), headered_content_control_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `HeaderedContentControl` of `headered_content_control_theme`.
fn headered_content_control_theme_template(t: &Template<HeaderedContentControl>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
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
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,*"))?);
                    Ok(())
                })?
                .children([
                    t.make(ContentPresenter::new())
                        .name("PART_HeaderPresenter")?
                        .set(Grid::row_property(), 0)
                        .template_bind(
                            ContentPresenter::content_property(),
                            HeaderedContentControl::header_property(),
                        )
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            HeaderedContentControl::header_template_property(),
                        )
                        .set(ContentPresenter::font_weight_property(), FontWeight::Normal)
                        .dynamic(ContentPresenter::foreground_property(), "TextControlHeaderForeground")
                        .dynamic(Layoutable::margin_property(), "TextBoxTopHeaderMargin")
                        .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                        .template_bind(
                            ContentPresenter::horizontal_content_alignment_property(),
                            ContentControl::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            ContentPresenter::vertical_content_alignment_property(),
                            ContentControl::vertical_content_alignment_property(),
                        )
                        .template_bind_with(
                            Visual::is_visible_property(),
                            Some(HeaderedContentControl::header_property().property()),
                            Some(ObjectConverters::is_not_null()),
                            None,
                            None,
                        )
                        .set(ContentPresenter::recognizes_access_key_property(), true)
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::row_property(), 1)
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
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
                        .control(),
                ]),
        )
        .control())
}

/// The default control theme of `HeaderedContentControl`.
fn headered_content_control_theme(theme: &StyleBuilder) -> Result {
    theme.target::<HeaderedContentControl>();
    theme.template(headered_content_control_theme_template);
    // Disabled (the header of DefaultTextBoxStyle)
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_HeaderPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TextControlHeaderForegroundDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}
