//! `Controls/TableViewCell.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TableView/TableViewRow.cpp (the cell wrappers TableViewRow builds in code for the
//!   columns, RefreshGridLines)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! A cell of WinUI's TableView is a Grid TableViewRow creates for a column: TableViewCellPadding
//! around the content of the cell, at least TableViewRowMinHeight high, the text in
//! TableViewCellFontSize, centred vertically, and a vertical grid line of 1 px in
//! TabularSurfaceGridLineBrush at its trailing edge (GridLinesVisibility All, the default), whose
//! brush the last cell leaves out where it meets the border of the table. FerroUI's TableViewCell
//! is that cell as a content control; the resources are Resources/TableView_themeresources.xaml and
//! Resources/TabularSurfaces_themeresources.xaml (TableView.xaml). The foreground of the cell is the
//! one of its row (TableViewRow.xaml). The editing, the focus and the automation of the cells of
//! WinUI have no counterpart in FerroUI's read-only TableView.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::Thickness;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::TableViewCell;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<TableViewCell>(), table_view_cell_theme);
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

/// The template of `TableViewCell` of `table_view_cell_theme`.
fn table_view_cell_theme_template(t: &Template<TableViewCell>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("CellWrapper")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
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
                ),
        )
        .control())
}

/// The default control theme of `TableViewCell`.
fn table_view_cell_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TableViewCell>();
    theme.dynamic(TemplatedControl::padding_property(), "TableViewCellPadding");
    theme.dynamic(Layoutable::min_height_property(), "TableViewRowMinHeight");
    theme.dynamic(TemplatedControl::font_size_property(), "TableViewCellFontSize");
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    // The vertical grid line at the trailing edge.
    theme.dynamic(TemplatedControl::border_brush_property(), "TabularSurfaceGridLineBrush");
    theme.set(
        TemplatedControl::border_thickness_property(),
        Thickness::new(0.0, 0.0, 1.0, 0.0),
    );
    theme.template(table_view_cell_theme_template);
    // The last cell meets the border of the table: its grid line keeps its width without a brush.
    theme.style(
        nesting()
            .nth_last_child(0, 1)
            .template()
            .of::<Border>()
            .name("CellWrapper"),
        |style| {
            style.set(
                Border::border_brush_property(),
                Some(literal(Brush::parse("Transparent"))?),
            );
            Ok(())
        },
    )?;
    Ok(())
}
