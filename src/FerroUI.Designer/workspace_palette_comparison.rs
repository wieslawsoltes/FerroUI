//! Scoped palette comparison in the native Studio definition inspector.
use super::{Context, SOURCE, report, text, tool_button};
use crate::PreviewTheme;
use crate::style_palette_comparison::{ComparedColor, PaletteComparison};
use ferroui_base::Ref;
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Border, ColumnDefinitions, Control, Grid, ScrollViewer, StackPanel, TextBlock, ToolTip,
};
use std::rc::Rc;

const THEMES: [PreviewTheme; 3] = [
    PreviewTheme::Light,
    PreviewTheme::Dark,
    PreviewTheme::HighContrast,
];

pub(super) fn panel(
    comparison: &PaletteComparison,
    context: &Context,
    status: &Ref<TextBlock>,
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
            let cell = color_cell(&values.key, THEMES[index], *value, context, status);
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
    context: &Context,
    status: &Ref<TextBlock>,
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
        let context = context.clone();
        let status = status.downgrade();
        open.click(move |_, _| {
            let result = match source.clone() {
                Some(source) => context.open_source(source),
                None => context
                    .workspace()
                    .map(|workspace| workspace.select_source_span(span)),
            };
            report(status.upgrade().as_ref(), result);
        });
        cell.children().add(open.upcast::<Control>());
    }
    cell.upcast()
}
