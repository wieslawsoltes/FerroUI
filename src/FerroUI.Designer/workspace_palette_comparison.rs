//! Scoped palette comparison in the native Studio definition inspector.
use super::{report, text, tool_button, Context, SOURCE};
use crate::style_palette_comparison::{ComparedColor, PaletteComparison};
use crate::PreviewTheme;
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::Ref;
use ferroui_controls::{
    Border, ColumnDefinitions, Control, Grid, ScrollViewer, StackPanel, TextBlock,
};

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
    let value = color.map_or("Unresolved", |color| {
        color.value.as_deref().unwrap_or("Expression")
    });
    if let Some(color) = Color::try_parse(value) {
        let swatch = Border::new();
        swatch.set_width(12.0);
        swatch.set_height(12.0);
        swatch.set_vertical_alignment(VerticalAlignment::Center);
        swatch.set_background(Some(SolidColorBrush::with_color(color).into()));
        cell.children().add(swatch.upcast::<Control>());
    }
    cell.children().add(text(value).upcast::<Control>());
    if let Some(source) = color.and_then(|color| color.source.clone()) {
        let open = tool_button(
            SOURCE,
            &format!("Open {} declaration for {key}", theme.label()),
        );
        let context = context.clone();
        let status = status.downgrade();
        open.click(move |_, _| {
            report(
                status.upgrade().as_ref(),
                context.open_source(source.clone()),
            )
        });
        cell.children().add(open.upcast::<Control>());
    }
    cell.upcast()
}
