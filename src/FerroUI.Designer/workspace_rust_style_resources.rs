//! Author and compare appearance layers in Studio's Rust color palette.
use super::super::palette_comparison::{self, COMPARE};
use super::{SOURCE, Scope, add_button, fields, picker, resource_setters, text};
use crate::style_palette_comparison::PaletteComparison;
use crate::{PropertyColorEditor, icons};
use ferroui_base::layout::Orientation;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Control, Grid, StackPanel};
use ferroui_designer_rsx::{
    ResourceColorMode, RsxStyle, RsxStyleEdit, RsxStyleField, RsxStyleResource,
    RsxStyleResourceFamily,
};
use ferroui_designer_tools::component_palette::PaletteLayer;

pub(super) fn section(
    body: &StackPanel,
    scope: &Scope,
    rule: &RsxStyle,
    fields: &[&RsxStyleField],
    choices: &[RsxStyleResourceFamily<'_>],
    comparison: &PaletteComparison,
) {
    let style = rule.declaration.start;
    let bar = StackPanel::new();
    bar.set_orientation(Orientation::Horizontal);
    bar.set_spacing(4.0);
    bar.children()
        .add(text("Color resources").upcast::<Control>());
    let display = scope.pane.palette.clone();
    let layer = display
        .layer
        .get()
        .or_else(|| rule.resources.iter().find_map(|entry| entry.layer))
        .unwrap_or(PaletteLayer::Shared);
    let layers = PaletteLayer::ALL;
    let selection = layers.iter().position(|entry| *entry == layer).unwrap_or(0);
    let layer_picker = picker(
        "Rust color layer",
        layers.into_iter().map(PaletteLayer::title),
        selection,
    );
    layer_picker.set_min_width(106.0);
    bar.children().add(layer_picker.clone().upcast::<Control>());
    let pane = fields::prompt("Add Rust color resource");
    let key = fields::input("Resource key", "");
    key.set_placeholder_text(Some("Resource key"));
    pane.children().add(key.clone().upcast::<Control>());
    let active = scope.clone();
    fields::value_row(&pane, "Color value", "#FF4C82D0", &[], move |value| {
        let key = key.text().unwrap_or_default().trim().to_owned();
        let layer = active.pane.palette.layer.get().unwrap_or(layer);
        active.apply(
            RsxStyleEdit::AddColorResource {
                style,
                key,
                value,
                layer: Some(layer),
            },
            None,
        );
    });
    fields::flyout_button(&bar, icons::ADD, "Add Rust color resource", pane);
    resource_setters::add_setter(&bar, scope, rule, fields, choices);
    body.children().add(bar.clone().upcast::<Control>());

    let colors = StackPanel::new();
    let rows: Vec<_> = layers
        .into_iter()
        .map(|choice| {
            let list = StackPanel::new();
            color_rows(&list, scope, rule, choice);
            list.set_is_visible(choice == layer);
            colors.children().add(list.clone().upcast::<Control>());
            list.downgrade()
        })
        .collect();
    body.children().add(colors.clone().upcast::<Control>());
    let comparison = (!comparison.rows.is_empty()).then(|| {
        let active = scope.clone();
        let comparison = palette_comparison::panel(comparison, move |source| active.reveal_declaration(source));
        comparison.set_is_visible(display.compare.get());
        colors.set_is_visible(!display.compare.get());
        body.children().add(comparison.clone());
        comparison.downgrade()
    });
    layer_picker.selection_changed({
        let picker = layer_picker.downgrade();
        let display = display.clone();
        let comparison = comparison.clone();
        let colors = colors.downgrade();
        move |_, _| {
            let Some(index) = picker
                .upgrade()
                .and_then(|picker| usize::try_from(picker.selected_index()).ok())
            else {
                return;
            };
            let Some(layer) = layers.get(index) else {
                return;
            };
            display.layer.set(Some(*layer));
            display.compare.set(false);
            if let Some(colors) = colors.upgrade() {
                colors.set_is_visible(true);
            }
            if let Some(comparison) = comparison.as_ref().and_then(|view| view.upgrade()) {
                comparison.set_is_visible(false);
            }
            for (row, view) in rows.iter().enumerate() {
                if let Some(view) = view.upgrade() {
                    view.set_is_visible(row == index);
                }
            }
        }
    });
    if let Some(comparison) = comparison {
        let colors = colors.downgrade();
        add_button(&bar, COMPARE, "Compare Rust theme colors", move || {
            let show = !display.compare.get();
            display.compare.set(show);
            if let (Some(comparison), Some(colors)) = (comparison.upgrade(), colors.upgrade()) {
                comparison.set_is_visible(show);
                colors.set_is_visible(!show);
            }
        });
    }
}

fn color_rows(body: &StackPanel, scope: &Scope, rule: &RsxStyle, layer: PaletteLayer) {
    let style = rule.declaration.start;
    for resource in rule
        .resources
        .iter()
        .filter(|entry| entry.layer == Some(layer))
    {
        let resource_id = resource.declaration.start;
        let row = if let Some((constant, color)) = resource.shared.as_ref().zip(resource.color()) {
            let row = fields::read_only_row(body, &resource.key, &color);
            edit_shared(&row, scope, style, resource, &constant.name, &color);
            row
        } else if let (Some(color), Ok(editor)) = (resource.color(), PropertyColorEditor::new()) {
            let (row, tools) = fields::row(body, &resource.key);
            let _ = editor.configure(Some(&color), false);
            let label = if layer == PaletteLayer::Shared {
                resource.key.clone()
            } else {
                format!("{} {}", resource.key, layer.title())
            };
            AutomationProperties::set_name(
                &editor.text_editor(),
                Some(&format!("{label} color resource value")),
            );
            let active = scope.clone();
            editor.on_commit(move |value| {
                active.apply(
                    RsxStyleEdit::ColorResource {
                        style,
                        resource: resource_id,
                        value,
                        mode: ResourceColorMode::Literal,
                    },
                    None,
                );
            });
            let field = editor.control();
            Grid::set_column(&field, 1);
            row.children().add(field);
            tools
        } else {
            fields::source_row(body, &resource.key)
        };
        let declaration = declaration(resource);
        let active = scope.clone();
        add_button(
            &row,
            SOURCE,
            &format!("Reveal {} resource", resource.key),
            move || active.reveal_declaration(declaration.clone()),
        );
    }
    if body.children().is_empty() {
        body.children()
            .add(text("No colors in this layer.").upcast::<Control>());
    }
}

pub(super) fn declaration(resource: &RsxStyleResource) -> palette_comparison::Declaration {
    resource.shared.as_ref().and_then(|constant| {
        constant.uri.clone().map(|uri| palette_comparison::Declaration::RustModule { uri, span: constant.value })
    }).unwrap_or_else(|| palette_comparison::Declaration::Local(resource.color_source()))
}

fn edit_shared(
    row: &StackPanel,
    scope: &Scope,
    style: usize,
    resource: &RsxStyleResource,
    constant: &str,
    color: &str,
) {
    let pane = fields::prompt(&format!("Change {} from const {constant}", resource.key));
    let owner = resource.shared.as_ref().and_then(|color| color.uri.as_deref())
        .and_then(|uri| ferroui_designer_project::file_path(uri).ok())
        .and_then(|path| path.file_name()?.to_str().map(str::to_owned));
    let shared = owner.map_or_else(|| "Edit the shared constant for all uses".to_owned(),
        |name| format!("Edit the shared constant in {name}"));
    let target = picker(
        "Rust color edit scope",
        ["Use a local color", shared.as_str()],
        0,
    );
    pane.children().add(target.clone().upcast::<Control>());
    let active = scope.clone();
    let resource = resource.declaration.start;
    fields::value_row(&pane, "Color value", color, &[], move |value| {
        let mode = if target.selected_index() == 1 {
            ResourceColorMode::Declaration
        } else {
            ResourceColorMode::Local
        };
        active.apply(
            RsxStyleEdit::ColorResource {
                style,
                resource,
                value,
                mode,
            },
            None,
        );
    });
    fields::flyout_button(
        row,
        icons::EXPRESSION,
        &format!("Change color from const {constant}"),
        pane,
    );
}
