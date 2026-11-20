//! `Controls/TableViewColumnHeader.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TableView/TableView.cpp (RebuildHeaders, AppendResizeGripperVisual,
//!   ApplyGridLinesToHeader: the header cells, which TableView builds in code)
//!   controls/dev/ResizeGripper/ResizeGripper.xaml (the implicit ResizeGripper style)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! A header cell of WinUI's TableView is a Grid TableView creates for a column: a content presenter
//! in TableViewHeaderFontSize and TableViewHeaderFontWeight with TableViewHeaderCellPadding, at
//! least TableViewRowMinHeight high, on a transparent fill; a grid line of 1 px in
//! TabularSurfaceGridLineBrush at its trailing edge, which the last header leaves out where it meets
//! the border of the table; and, for a column the user can resize, a ResizeGripper of
//! TableViewResizeGripperWidth at the trailing edge. FerroUI's TableViewColumnHeader is that cell as
//! a control (its resizer is the part PART_Resizer, a Thumb). Its resources are
//! Resources/TableView_themeresources.xaml, Resources/TabularSurfaces_themeresources.xaml and
//! Resources/ResizeGripper_themeresources.xaml (TableView.xaml).
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The PointerOver and Pressed fills of a header (TabularSurfaceHeader*PointerOver/Pressed) are
//!   the states of a header WinUI's TableView can sort by; FerroUI's TableView does not sort, so a
//!   header has only the Normal state, as a header of a column WinUI does not sort by.
//! - The ResizeGripper is the keyed Thumb theme TableViewColumnHeaderResizerStyle: its separator
//!   (ResizeGripperSeparatorBrush, ResizeGripperSeparatorThickness) shows on pointer over and press
//!   (the states PointerOver and Pressed of the source).
//! - The grid line, a Border of 1 px TableView adds to the cell, is the trailing border of the content
//!   presenter (BorderThickness 0,0,1,0), which the last header draws without a brush; the
//!   presenter honours the background, the border, the corner radius and the padding of the header
//!   as the Fluent template of FerroUI does.
//! - The sort indicator, the tool tip of the header and the automation view are left out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::Cursor;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::Thumb;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Panel;
use ferroui_controls::TableViewColumn;
use ferroui_controls::TableViewColumnHeader;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The ResizeGripper of a header (ResizeGripper.xaml), horizontal.
    resources.control_theme(
        "TableViewColumnHeaderResizerStyle",
        table_view_column_header_resizer_style,
    );
    resources.control_theme(type_key::<TableViewColumnHeader>(), table_view_column_header_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `Thumb` of `table_view_column_header_resizer_style`.
fn table_view_column_header_resizer_style_template(t: &Template<Thumb>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .set(
            Layoutable::horizontal_alignment_property(),
            HorizontalAlignment::Stretch,
        )
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .name("Separator")?
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .dynamic(Border::border_thickness_property(), "ResizeGripperSeparatorThickness")
                .set(Visual::opacity_property(), 0.0)
                .set(InputElement::is_hit_test_visible_property(), false)
                .set(
                    Layoutable::horizontal_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch),
        )
        .control())
}

/// `TableViewColumnHeaderResizerStyle`: a control theme of `Thumb`.
fn table_view_column_header_resizer_style(theme: &StyleBuilder) -> Result {
    theme.target::<Thumb>();
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "ResizeGripperSeparatorBrush");
    theme.template(table_view_column_header_resizer_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("Separator"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("Separator"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `TableViewColumnHeader` of `table_view_column_header_theme`.
fn table_view_column_header_theme_template(t: &Template<TableViewColumnHeader>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("HeaderCell")?
        // The content presenter draws the fill and the grid line (the border at the trailing
        //               edge) and honours the corner radius, as in the Fluent template of FerroUI.
        .children([
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .template_bind(
                    ContentPresenter::background_property(),
                    TemplatedControl::background_property(),
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
                .control(),
            t.make(Thumb::new())
                .name("PART_Resizer")?
                .static_resource(StyledElement::theme_property(), "TableViewColumnHeaderResizerStyle")?
                .dynamic(Layoutable::width_property(), "TableViewResizeGripperWidth")
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                .set(
                    InputElement::cursor_property(),
                    Some(literal(Cursor::parse("SizeWestEast"))?),
                )
                .bind(
                    InputElement::is_enabled_property(),
                    path()
                        .templated_parent()
                        .prop_or_null(TableViewColumnHeader::column_property())
                        .prop(TableViewColumn::can_user_effectively_resize_property())
                        .binding()
                        .with_fallback_value(value("False".to_string())),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `TableViewColumnHeader`.
fn table_view_column_header_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TableViewColumnHeader>();
    // The fill of a header cell: transparent, so that the padding takes pointer input.
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    // The grid line at the trailing edge.
    theme.dynamic(TemplatedControl::border_brush_property(), "TabularSurfaceGridLineBrush");
    theme.set(
        TemplatedControl::border_thickness_property(),
        Thickness::new(0.0, 0.0, 1.0, 0.0),
    );
    theme.dynamic(TemplatedControl::padding_property(), "TableViewHeaderCellPadding");
    theme.dynamic(Layoutable::min_height_property(), "TableViewRowMinHeight");
    theme.dynamic(TemplatedControl::font_size_property(), "TableViewHeaderFontSize");
    theme.dynamic(TemplatedControl::font_weight_property(), "TableViewHeaderFontWeight");
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.template(table_view_column_header_theme_template);
    // The last header meets the border of the table: its grid line keeps its width without a
    //         brush.
    theme.style(
        nesting()
            .nth_last_child(0, 1)
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.set(
                ContentPresenter::border_brush_property(),
                Some(literal(Brush::parse("Transparent"))?),
            );
            Ok(())
        },
    )?;
    Ok(())
}
