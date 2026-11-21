//! Scoped palette comparison in Studio's XAML and Rust editors.
use super::inspector_actions::{SOURCE, text, tool_button};
use crate::PreviewTheme;
use crate::style_palette_comparison::{ComparedColor, PaletteComparison};
use crate::style_sources::StyleSource;
use ferroui_base::Ref;
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Border, ColumnDefinitions, Control, Grid, ScrollViewer, StackPanel, ToolTip,
};
use ferroui_designer_core::SourceSpan;
use std::rc::Rc;

const THEMES: [PreviewTheme; 3] = [
    PreviewTheme::Light,
    PreviewTheme::Dark,
    PreviewTheme::HighContrast,
];

pub(super) const COMPARE: &str = "F1 M1,2 H15 V14 H1 Z M2,3 V13 H7 V3 Z M9,3 V13 H14 V3 Z";

pub(super) fn panel(
    comparison: &PaletteComparison,
    reveal: impl Fn(Option<StyleSource>, SourceSpan) + Clone + 'static,
) -> Ref<Control> {
    let count = if comparison.high_contrast { 3 } else { 2 };
    let content = StackPanel::new();
    let heading = row("Color", count);
    for (index, theme) in THEMES[..count].iter().enumerate() {
        add_cell(&heading, index, text(theme.label()).upcast());
    }
    content.children().add(heading.upcast::<Control>());
    for values in &comparison.rows {
        let row = row(&values.key, count);
        let cells = [
            values.light.as_ref(),
            values.dark.as_ref(),
            values.high_contrast.as_ref(),
        ];
        for (index, value) in cells[..count].iter().enumerate() {
            let cell = color_cell(&values.key, THEMES[index], *value, reveal.clone());
            add_cell(&row, index, cell);
        }
        content.children().add(row.upcast::<Control>());
    }
    let scroll = ScrollViewer::new();
    scroll.set_max_height(174.0);
    scroll.set_horizontal_scroll_bar_visibility(
        ferroui_controls::primitives::ScrollBarVisibility::Disabled,
    );
    scroll.set_content(Some(Control::boxed(content)));
    scroll.upcast()
}

fn row(key: &str, count: usize) -> Ref<Grid> {
    let row = Grid::new();
    let columns = if count == 3 { "*,*,*,*" } else { "*,*,*" };
    row.set_column_definitions(ColumnDefinitions::parse(columns).unwrap());
    row.set_column_spacing(5.0);
    row.children().add(text(key).upcast::<Control>());
    row
}

fn add_cell(row: &Grid, index: usize, cell: Ref<Control>) {
    Grid::set_column(&cell, index as i32 + 1);
    row.children().add(cell);
}

fn color_cell(
    key: &str,
    theme: PreviewTheme,
    color: Option<&ComparedColor>,
    reveal: impl Fn(Option<StyleSource>, SourceSpan) + 'static,
) -> Ref<Control> {
    let cell = StackPanel::new();
    cell.set_orientation(Orientation::Horizontal);
    cell.set_spacing(3.0);
    let value = match color {
        Some(color) if color.issue.is_none() => color.value.as_deref().unwrap_or("Expression"),
        _ => "Unresolved",
    };
    if let Some(color) = Color::try_parse(value) {
        let swatch = Border::new();
        swatch.set_width(12.0);
        swatch.set_height(12.0);
        swatch.set_vertical_alignment(VerticalAlignment::Center);
        swatch.set_background(Some(SolidColorBrush::with_color(color).into()));
        cell.children().add(swatch.upcast::<Control>());
    }
    let label = text(value);
    cell.children().add(label.clone().upcast::<Control>());
    if let Some(color) = color {
        let path = color
            .path
            .iter()
            .map(|step| match step.reference {
                Some(reference) => format!("{reference} {}", step.key),
                None => step.key.clone(),
            })
            .collect::<Vec<_>>()
            .join(" → ");
        let name = format!("Open {} declaration for {key}", theme.label());
        let details = [
            Some(name.as_str()),
            Some(value),
            color.scope.as_deref(),
            (!path.is_empty()).then_some(path.as_str()),
            color.issue.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        let open = tool_button(SOURCE, &name);
        ToolTip::set_tip(&label, Some(Rc::new(details.clone())));
        ToolTip::set_tip(&open, Some(Rc::new(details.clone())));
        AutomationProperties::set_name(&open, Some(&details));
        let source = color.source.clone();
        let span = color.span;
        open.click(move |_, _| reveal(source.clone(), span));
        cell.children().add(open.upcast::<Control>());
    }
    cell.upcast()
}
