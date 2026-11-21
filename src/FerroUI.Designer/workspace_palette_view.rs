//! Palette swatches, source navigation, and the native color editor in Studio.
use super::{text, tool_button, Context, ADD, SOURCE};
use crate::style_composition::contrast_providers;
use crate::workspace::palette_comparison::{self, COMPARE};
use crate::{PreviewTheme, PropertyColorEditor, ScopedPalette};
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::Color;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    ColumnDefinitions, ComboBox, Control, Grid, ScrollViewer, StackPanel, TextBlock,
    TextBox,
};
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::component_palette::{
    palette_layers, PaletteColor, PaletteColorKind, PaletteLayer, PaletteLayerModel,
};
use ferroui_designer_tools::theme_variants::HighContrastProvider;
use std::rc::Rc;

const SUN: &str = "M8,1 V3 M8,13 V15 M1,8 H3 M13,8 H15 M3,3 L4.5,4.5 M11.5,11.5 L13,13 M13,3 L11.5,4.5 M4.5,11.5 L3,13 M12,8 A4,4 0 1 1 4,8 A4,4 0 1 1 12,8";
const MOON: &str = "F1 M14,10.2 A6.8,6.8 0 1 1 5.8,2 A6.1,6.1 0 0 0 14,10.2 Z";
const CONTRAST: &str = "F0 M8,1 A7,7 0 1 1 8,15 A7,7 0 1 1 8,1 Z M8,2.5 A5.5,5.5 0 0 0 8,13.5 Z";

pub(super) fn body(
    context: Context,
    owner: NodeId,
    layer: Option<PaletteLayer>,
) -> Result<Ref<Control>, String> {
    let workspace = context.workspace()?;
    let session = workspace.session.borrow();
    let document = session.document();
    let model = Rc::new(palette_layers(document, owner).map_err(|error| error.to_string())?);
    let title = super::super::node_title(document, owner);
    let comparison = ScopedPalette::for_session(&session).compare_palette(owner, &model)?;
    drop(session);

    let content = StackPanel::new();
    content.set_width(if comparison.high_contrast { 424.0 } else { 344.0 });
    content.set_spacing(7.0);
    content
        .children()
        .add(text(&format!("Palette · {title}")).upcast::<Control>());
    let bar = StackPanel::new();
    bar.set_orientation(Orientation::Horizontal);
    bar.set_spacing(4.0);
    let layer = layer.unwrap_or_else(|| match workspace.preview_environment().theme {
        PreviewTheme::Dark => PaletteLayer::Dark,
        PreviewTheme::HighContrast => PaletteLayer::HighContrast,
        _ => PaletteLayer::Light,
    });
    let active = model
        .iter()
        .position(|entry| entry.layer == layer)
        .unwrap_or(0);
    let chooser = choice(
        "PaletteLayer",
        "Palette layer",
        model
            .iter()
            .map(|entry| entry.layer.title().to_owned())
            .collect(),
        active,
    );
    chooser.set_min_width(100.0);
    bar.children().add(chooser.clone().upcast::<Control>());
    let status = text("");
    status.set_name(Some("PaletteStatus".into()));
    status.set_is_visible(false);
    status.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    for (theme, glyph, label, name) in [
        (
            PreviewTheme::Light,
            SUN,
            "Preview Light theme",
            "PalettePreviewLight",
        ),
        (
            PreviewTheme::Dark,
            MOON,
            "Preview Dark theme",
            "PalettePreviewDark",
        ),
        (
            PreviewTheme::HighContrast,
            CONTRAST,
            "Preview High contrast theme",
            "PalettePreviewHighContrast",
        ),
    ] {
        if theme == PreviewTheme::HighContrast && !comparison.high_contrast {
            continue;
        }
        let button = tool_button(glyph, label);
        button.set_name(Some(name.into()));
        let verify = context.clone();
        let error = status.downgrade();
        button.click(move |_, _| report(error.upgrade().as_ref(), verify.preview(theme)));
        bar.children().add(button.upcast::<Control>());
    }
    let comparison = palette_comparison::panel(&comparison, {
        let context = context.clone();
        let status = status.downgrade();
        move |source, span| {
            let result = match source {
                Some(source) => context.open_source(source),
                None => context.workspace().map(|workspace| workspace.select_source_span(span)),
            };
            report(status.upgrade().as_ref(), result);
        }
    });
    comparison.set_is_visible(false);
    let toggle = tool_button(COMPARE, "Compare theme colors");
    toggle.set_name(Some("PaletteCompare".into()));
    let weak = comparison.downgrade();
    toggle.click(move |_, _| {
        if let Some(comparison) = weak.upgrade() {
            comparison.set_is_visible(!comparison.is_visible());
        }
    });
    bar.children().add(toggle.upcast::<Control>());
    content.children().add(bar.upcast::<Control>());
    content.children().add(comparison);
    let rows = StackPanel::new();
    rows.set_spacing(4.0);
    let scroll = ScrollViewer::new();
    scroll.set_max_height(300.0);
    scroll.set_horizontal_scroll_bar_visibility(
        ferroui_controls::primitives::ScrollBarVisibility::Disabled,
    );
    scroll.set_content(Some(Control::boxed(&rows)));
    render_layer(&rows, &status, &context, owner, &model[active]);
    let target = rows.downgrade();
    let error = status.downgrade();
    let selection = chooser.downgrade();
    chooser.selection_changed(move |_, _| {
        let (Some(rows), Some(status), Some(select)) =
            (target.upgrade(), error.upgrade(), selection.upgrade())
        else {
            return;
        };
        if let Some(layer) = model.get(select.selected_index() as usize) {
            render_layer(&rows, &status, &context, owner, layer);
        }
    });
    content.children().add(scroll.upcast::<Control>());
    content.children().add(status.upcast::<Control>());
    Ok(content.upcast())
}

fn render_layer(
    panel: &StackPanel,
    status: &Ref<TextBlock>,
    context: &Context,
    owner: NodeId,
    layer: &PaletteLayerModel,
) {
    panel.children().clear();
    status.set_is_visible(false);
    if let Some(message) = &layer.unavailable {
        let note = text(message);
        note.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        panel.children().add(note.upcast::<Control>());
        return;
    }
    for color in &layer.colors {
        panel.children().add(color_row(context, status, color));
    }
    if context.writable {
        let needs_provider = layer.layer == PaletteLayer::HighContrast && layer.declaration.is_none();
        let providers = if needs_provider { contrast_providers() } else { Vec::new() };
        if needs_provider && providers.is_empty() {
            let message = text("Load a theme package to create the High contrast layer.");
            message.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
            panel.children().add(message.upcast::<Control>());
            return;
        }
        panel
            .children()
            .add(add_row(context, status, owner, layer.layer, providers));
    }
}

fn color_row(context: &Context, status: &Ref<TextBlock>, color: &PaletteColor) -> Ref<Control> {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("*,1.4*,Auto").unwrap());
    row.set_column_spacing(5.0);
    let key = text(&color.key);
    key.set_vertical_alignment(VerticalAlignment::Center);
    row.children().add(key.upcast::<Control>());
    let editable = color.editable && context.writable;
    let field = if let (true, Ok(editor)) = (editable, PropertyColorEditor::new()) {
        let value = color
            .value
            .as_deref()
            .filter(|value| Color::try_parse(value.trim()).is_some());
        let _ = editor.configure(value, false);
        editor
            .text_editor()
            .set_name(Some(format!("PaletteColor{}", color.node)));
        AutomationProperties::set_name(
            &editor.text_editor(),
            Some(&format!("{} color value", color.key)),
        );
        let verify = context.clone();
        let error = status.downgrade();
        let node = color.node;
        editor.on_commit(move |value| {
            report(
                error.upgrade().as_ref(),
                verify.edit(|session| session.change_palette_color(node, &value)),
            )
        });
        editor.control()
    } else {
        text(color.value.as_deref().unwrap_or("Source value")).upcast()
    };
    Grid::set_column(&field, 1);
    row.children().add(field);
    let reveal = tool_button(SOURCE, &format!("Reveal {} in source", color.key));
    let selection = context.clone();
    let error = status.downgrade();
    let span = color.span;
    reveal.click(move |_, _| {
        let result = selection
            .workspace()
            .map(|workspace| workspace.select_source_span(span));
        report(error.upgrade().as_ref(), result);
    });
    Grid::set_column(&reveal, 2);
    row.children().add(reveal.upcast::<Control>());
    row.upcast()
}

fn add_row(
    context: &Context,
    status: &Ref<TextBlock>,
    owner: NodeId,
    layer: PaletteLayer,
    providers: Vec<HighContrastProvider>,
) -> Ref<Control> {
    let root = StackPanel::new();
    root.set_spacing(4.0);
    let fields = Grid::new();
    fields.set_column_definitions(ColumnDefinitions::parse("*,*,Auto").unwrap());
    fields.set_column_spacing(5.0);
    let key = input("PaletteResourceKey", "Color key", "");
    let color = input("PaletteNewColor", "New color value", "#FF4C82D0");
    let kind = choice(
        "PaletteResourceType",
        "Color resource type",
        vec!["SolidColorBrush".into(), "Color".into()],
        0,
    );
    let provider = (!providers.is_empty()).then(|| choice(
        "PaletteContrastProvider",
        "High contrast theme package",
        providers.iter().map(|provider| provider.label().into()).collect(),
        0,
    ));
    let add = tool_button(ADD, "Add palette color");
    add.set_name(Some("PaletteAddColor".into()));
    let error = status.downgrade();
    let verify = context.clone();
    let values = (key.clone(), color.clone(), kind.clone());
    let selected_provider = provider.clone();
    add.click(move |_, _| {
        let kind = if values.2.selected_index() == 0 {
            PaletteColorKind::SolidColorBrush
        } else {
            PaletteColorKind::Color
        };
        let key = values.0.text().unwrap_or_default();
        let color = values.1.text().unwrap_or_default();
        let provider = selected_provider.as_ref()
            .and_then(|chooser| providers.get(chooser.selected_index() as usize).copied());
        let result = verify.edit(|session| {
            session.add_palette_color(owner, layer, key.trim(), kind, &color, provider)
        });
        report(error.upgrade().as_ref(), result);
    });
    fields.children().add(key.upcast::<Control>());
    Grid::set_column(&color, 1);
    fields.children().add(color.upcast::<Control>());
    Grid::set_column(&add, 2);
    fields.children().add(add.upcast::<Control>());
    root.children().add(fields.upcast::<Control>());
    root.children().add(kind.upcast::<Control>());
    if let Some(provider) = provider {
        root.children().add(provider.upcast::<Control>());
    }
    root.upcast()
}

pub(super) fn choice(name: &str, label: &str, items: Vec<String>, index: usize) -> Ref<ComboBox> {
    let select = ComboBox::new();
    select.set_name(Some(name.into()));
    for item in items {
        select.items().add(Some(Rc::new(item)));
    }
    select.set_selected_index(index as i32);
    AutomationProperties::set_name(&select, Some(label));
    select
}

fn input(name: &str, label: &str, value: &str) -> Ref<TextBox> {
    let field = TextBox::new();
    field.set_name(Some(name.into()));
    field.set_text(Some(value));
    field.set_placeholder_text(Some(label));
    field.set_padding(Thickness::uniform(4.0));
    AutomationProperties::set_name(&field, Some(label));
    field
}

fn report(status: Option<&Ref<TextBlock>>, result: Result<(), String>) {
    if let Some(status) = status {
        status.set_is_visible(result.is_err());
        status.set_text(result.as_ref().err().map(String::as_str));
    }
}
