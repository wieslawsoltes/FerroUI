//! `Controls/GroupBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/Expander/Expander.xaml (DefaultExpanderStyle in its expanded state, without
//!   its chevron)
//!   controls/dev/Expander/Expander_themeresources.xaml (the header of ExpanderHeaderDownStyle:
//!   its Normal and Disabled states; the resources of the document are
//!   Resources/Expander_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WinUI has no GroupBox. The WinUI control that shows a header over a group of content in a
//! card is the Expander, so this theme is an expanded Expander that cannot collapse: the header
//! bar of the Expander (ExpanderHeaderBackground, ExpanderHeaderBorderBrush,
//! ExpanderHeaderBorderThickness, ExpanderHeaderPadding, the minimum height ExpanderMinHeight,
//! the top corners of ControlCornerRadius, the header in ExpanderHeaderForeground with the
//! content alignments ExpanderHeaderHorizontalContentAlignment and
//! ExpanderHeaderVerticalContentAlignment) over the content of the Expander (its Background
//! ExpanderContentBackground, BorderBrush ExpanderContentBorderBrush, BorderThickness
//! ExpanderContentDownBorderThickness, Padding ExpanderContentPadding, the bottom corners). The
//! chevron of the Expander and its pointer and pressed states are left out: the header of a
//! group box does not toggle anything.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The header, a ToggleButton with ExpanderHeaderDownStyle in the source, is a Border
//!   (ToggleButtonGrid of the style) around FerroUI's PART_HeaderPresenter; the content Border
//!   (ExpanderContent) holds PART_ContentPresenter. The corner radii are the corner radius of
//!   the control through TopCornerRadiusFilterConverter and BottomCornerRadiusFilterConverter, as
//!   the expanded state of the source sets them.
//! - The content border takes the BorderThickness of the control (the source writes
//!   ExpanderContentDownBorderThickness on the border and in the style). The Expander's own
//!   setters that place the control (HorizontalAlignment Left, VerticalAlignment Center,
//!   MinWidth FlyoutThemeMinWidth, IsTabStop) are not taken: a group box stretches in its
//!   container, as in FerroUI's other themes.
//! - RecognizesAccessKey is FerroUI's access key support.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::BackgroundSizing;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::HeaderedContentControl;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::GroupBox;
use ferroui_controls::RowDefinitions;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<GroupBox>(), group_box_theme);
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

/// The template of `GroupBox` of `group_box_theme`.
fn group_box_theme_template(t: &Template<GroupBox>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("RootGrid")?
        .with(|e| {
            e.set_row_definitions(literal(RowDefinitions::parse("Auto,*"))?);
            Ok(())
        })?
        .children([
            t.make(Border::new())
                .name("ToggleButtonGrid")?
                .set(Grid::row_property(), 0)
                .dynamic(Border::background_property(), "ExpanderHeaderBackground")
                .template_bind(
                    Border::background_sizing_property(),
                    TemplatedControl::background_sizing_property(),
                )
                .dynamic(Border::border_brush_property(), "ExpanderHeaderBorderBrush")
                .dynamic(Border::border_thickness_property(), "ExpanderHeaderBorderThickness")
                .template_bind_with(
                    Border::corner_radius_property(),
                    Some(TemplatedControl::corner_radius_property().property()),
                    Some(t.scope().converter("TopCornerRadiusFilterConverter")?),
                    None,
                    None,
                )
                .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
                .static_resource(Decorator::padding_property(), "ExpanderHeaderPadding")?
                .child(
                    Decorator::child_property(),
                    t.make(ContentPresenter::new())
                        .name("PART_HeaderPresenter")?
                        .template_bind(
                            ContentPresenter::content_property(),
                            HeaderedContentControl::header_property(),
                        )
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            HeaderedContentControl::header_template_property(),
                        )
                        .dynamic(ContentPresenter::foreground_property(), "ExpanderHeaderForeground")
                        .static_resource(
                            ContentPresenter::horizontal_content_alignment_property(),
                            "ExpanderHeaderHorizontalContentAlignment",
                        )?
                        .static_resource(
                            ContentPresenter::vertical_content_alignment_property(),
                            "ExpanderHeaderVerticalContentAlignment",
                        )?
                        .set(ContentPresenter::recognizes_access_key_property(), true),
                )
                .control(),
            t.make(Border::new())
                .name("ExpanderContent")?
                .set(Grid::row_property(), 1)
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::background_sizing_property(),
                    TemplatedControl::background_sizing_property(),
                )
                .template_bind_with(
                    Border::corner_radius_property(),
                    Some(TemplatedControl::corner_radius_property().property()),
                    Some(t.scope().converter("BottomCornerRadiusFilterConverter")?),
                    None,
                    None,
                )
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
                .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
                .child(
                    Decorator::child_property(),
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
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
                        .set(ContentPresenter::recognizes_access_key_property(), true),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `GroupBox`.
fn group_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<GroupBox>();
    theme.dynamic(TemplatedControl::background_property(), "ExpanderContentBackground");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.static_resource(Layoutable::min_height_property(), "ExpanderMinHeight")?;
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ExpanderContentDownBorderThickness",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "ExpanderContentBorderBrush");
    theme.static_resource(TemplatedControl::padding_property(), "ExpanderContentPadding")?;
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(group_box_theme_template);
    // Disabled (ExpanderHeaderDownStyle)
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("ToggleButtonGrid"), |style| {
            style.dynamic(Border::border_brush_property(), "ExpanderHeaderDisabledBorderBrush");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_HeaderPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ExpanderHeaderDisabledForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}
