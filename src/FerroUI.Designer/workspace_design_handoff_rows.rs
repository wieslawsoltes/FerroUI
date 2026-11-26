use crate::{design_handoff::Row, icons};
use ferroui_base::{
    layout::{HorizontalAlignment, Orientation},
    media::{FontWeight, TextWrapping},
    Ref, Thickness,
};
use ferroui_controls::{
    automation::AutomationProperties, Border, Button, ColumnDefinitions, Control, Grid, StackPanel,
    TextBlock, ToolTip,
};
use std::rc::Rc;

pub(super) const HANDOFF: &str = "F1 M2 3H8V9H2Z M8 7H14V13H8Z M3 4V8H7V4Z M9 8V12H13V8Z";
pub(super) const OPEN: &str = "F1 M1 4H6L8 6H15V13H1Z M2.5 5.5V11.5H13.5V7.5H7.4L5.4 5.5Z";
pub(super) const REFRESH: &str =
    "F1 M13 1V5H9L10.7 3.3A5 5 0 1 0 13 8H14.5A6.5 6.5 0 1 1 11.8 2.2Z";
pub(super) const SOURCE: &str = crate::workspace::inspector_actions::SOURCE;
const PREVIEW: &str = "F1 M1 3H15V13H1Z M2.5 4.5V11.5H13.5V4.5Z M6 6L10 8L6 10Z";

#[derive(Clone, Copy)]
pub(super) enum RowAction {
    Copy,
    Place,
    Source,
    Preview,
    OpenAsset,
}

pub(super) fn set_icon(button: &Ref<Button>, path: &str) {
    button.set_content(Some(Control::boxed(icons::vector_icon(path, 14.0))));
}

pub(super) fn mapping(row: &Row, invoke: impl Fn(RowAction) + 'static) -> Ref<Control> {
    let grid = Grid::new();
    grid.set_column_definitions(
        ColumnDefinitions::parse("1.05*,1.9*,94").expect("mapping columns"),
    );
    grid.set_column_spacing(10.0);
    let labels = StackPanel::new();
    let title = text(&row.name, false);
    title.set_font_weight(FontWeight::SemiBold);
    labels.children().add(title.upcast::<Control>());
    if row.id != row.name {
        labels
            .children()
            .add(text(&row.id, true).upcast::<Control>());
    }
    grid.children().add(labels.upcast::<Control>());

    let details = StackPanel::new();
    details.set_spacing(2.0);
    details
        .children()
        .add(text(&row.target, false).upcast::<Control>());
    if let Some(reference) = row
        .reference
        .as_ref()
        .filter(|reference| *reference != &row.target)
    {
        details
            .children()
            .add(text(reference, true).upcast::<Control>());
    }
    if let Some(copy) = &row.copy {
        let transfer = text(
            &format!(
                "{}\n→ {} · {} bytes",
                copy.source_label, copy.destination_label, copy.bytes
            ),
            true,
        );
        ToolTip::set_tip(
            &transfer,
            Some(Rc::new(format!("{}\n→ {}", copy.source, copy.destination))),
        );
        details.children().add(transfer.upcast::<Control>());
    }
    if let Some(issue) = &row.issue {
        details
            .children()
            .add(text(issue, false).upcast::<Control>());
    }
    Grid::set_column(&details, 1);
    grid.children().add(details.upcast::<Control>());

    let tools = StackPanel::new();
    tools.set_orientation(Orientation::Horizontal);
    tools.set_horizontal_alignment(HorizontalAlignment::Right);
    tools.set_spacing(3.0);
    let invoke: Rc<dyn Fn(RowAction)> = Rc::new(invoke);
    if row.review.is_some() {
        if let Some(copy) = &row.copy {
            let import = copy.direction == "import";
            append(
                &tools,
                row,
                if import {
                    "Import asset"
                } else {
                    "Export asset"
                },
                if import {
                    icons::MOVE_DOWN
                } else {
                    icons::MOVE_UP
                },
                RowAction::Copy,
                &invoke,
            );
        }
        if row.insertion.is_some() {
            append(
                &tools,
                row,
                "Place mapped item",
                icons::ADD,
                RowAction::Place,
                &invoke,
            );
        }
        if row.definition.is_some() {
            append(
                &tools,
                row,
                "Reveal definition",
                SOURCE,
                RowAction::Source,
                &invoke,
            );
            append(
                &tools,
                row,
                "Preview definition",
                PREVIEW,
                RowAction::Preview,
                &invoke,
            );
        }
        if row.asset_uri.is_some() {
            append(
                &tools,
                row,
                "Open asset",
                OPEN,
                RowAction::OpenAsset,
                &invoke,
            );
        }
    }
    Grid::set_column(&tools, 2);
    grid.children().add(tools.upcast::<Control>());
    let border = Border::new();
    border.classes().add("handoff-row");
    border.set_padding(Thickness::new(3.0, 7.0, 3.0, 7.0));
    border.set_child(Some(grid.upcast::<Control>()));
    border.upcast::<Control>()
}

fn text(value: &str, muted: bool) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_text_wrapping(TextWrapping::Wrap);
    if muted {
        label.set_font_size(11.0);
        label.classes().add("designer-muted");
    }
    label
}

fn append(
    tools: &Ref<StackPanel>,
    row: &Row,
    name: &str,
    icon: &str,
    action: RowAction,
    invoke: &Rc<dyn Fn(RowAction)>,
) {
    let button = Button::new();
    let label = format!("{name} · {}", row.name);
    button.classes().add("icon");
    AutomationProperties::set_name(&button, Some(&label));
    ToolTip::set_tip(&button, Some(Rc::new(label)));
    set_icon(&button, icon);
    let invoke = invoke.clone();
    button.click(move |_, _| invoke(action));
    tools.children().add(button.upcast::<Control>());
}
