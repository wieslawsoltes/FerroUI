//! A real laid-out control tree exported with and without UI provenance.
//!
//! `cargo run -p ferroui-svg --example semantic_export -- samples/SvgExport/semantics`
//! The two SVGs render identically. The richer file retains source UI structure even though
//! its visible text is outlined and the complete UI is one compositing layer.

use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{FontFamily, FontManager, IBrush};
use ferroui_base::platform::{IAssetLoader, StandardAssetLoader};
use ferroui_base::{CornerRadius, FerroLocator, Rect, Ref, Size, Thickness, Visual};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::{Border, Button, Control, StackPanel, TextBlock};
use ferroui_svg::{
    SvgDocument, SvgExportOptions, SvgFidelity, SvgLayerOptions, SvgPlatform,
    SvgSemanticAccessibilityMode, SvgSemanticCaptureOptions, SvgSemanticExportOptions,
    SvgSemanticNode, SvgSemanticSource, SvgSemanticValue,
};
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;

fn brush(color: u32) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::from_uint32(color))
}

fn label(id: &str, value: &str, size: f64, color: u32) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_name(Some(id.to_owned()));
    text.set_text(Some(value));
    text.set_font_family(FontFamily::parse("fonts:Inter#Inter").unwrap());
    text.set_font_size(size);
    text.set_foreground(Some(brush(color)));
    text
}

fn action(id: &str, value: &str, primary: bool) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(id.to_owned()));
    button.set_width(168.0);
    button.set_height(44.0);
    AutomationProperties::set_automation_id(&button, Some(id));
    AutomationProperties::set_name(&button, Some(value));
    let id = id.to_owned();
    let value = value.to_owned();
    button.set_template(Some(FuncControlTemplate::for_type::<Button>(
        move |_, _| {
            let chrome = Border::new();
            chrome.set_name(Some(format!("{id}Chrome")));
            chrome.set_corner_radius(CornerRadius::uniform(8.0));
            chrome.set_background(Some(brush(if primary { 0xff176c71 } else { 0xffe9eff3 })));
            chrome.set_padding(Thickness::uniform(12.0));
            let caption = label(
                &format!("{id}Caption"),
                &value,
                15.0,
                if primary { 0xffffffff } else { 0xff263f50 },
            );
            caption.set_horizontal_alignment(HorizontalAlignment::Center);
            caption.set_vertical_alignment(VerticalAlignment::Center);
            chrome.set_child(Some(caption.upcast::<Control>()));
            chrome.upcast::<Control>()
        },
    )));
    button
}

fn settings() -> Ref<Border> {
    let page = Border::new();
    page.set_name(Some("WorkspaceSettings".into()));
    page.set_background(Some(brush(0xfff1f5f8)));
    page.set_padding(Thickness::uniform(40.0));
    let content = StackPanel::new();
    content.set_name(Some("SettingsContent".into()));
    content.set_spacing(14.0);
    content
        .children()
        .add(label("Eyebrow", "FERROUI / UI SEMANTICS", 12.0, 0xff5c788a).upcast::<Control>());
    content
        .children()
        .add(label("Heading", "Workspace settings", 30.0, 0xff193047).upcast::<Control>());
    content.children().add(
        label(
            "Introduction",
            "One rendered scene. A preserved UI model.",
            15.0,
            0xff5c788a,
        )
        .upcast::<Control>(),
    );

    let card = Border::new();
    card.set_name(Some("ProjectCard".into()));
    card.set_margin(Thickness::new(0.0, 12.0, 0.0, 4.0));
    card.set_background(Some(brush(0xffffffff)));
    card.set_border_brush(Some(brush(0xffd9e3ea)));
    card.set_border_thickness(Thickness::uniform(1.0));
    card.set_corner_radius(CornerRadius::uniform(12.0));
    card.set_padding(Thickness::uniform(24.0));
    let fields = StackPanel::new();
    fields.set_name(Some("ProjectFields".into()));
    fields.set_spacing(14.0);
    fields
        .children()
        .add(label("ProjectLabel", "PROJECT", 12.0, 0xff5c788a).upcast::<Control>());
    fields
        .children()
        .add(label("ProjectTitle", "Aeris control module", 23.0, 0xff193047).upcast::<Control>());
    fields.children().add(
        label(
            "ProjectDescription",
            "Vector layers, outlined typography and explicit UI provenance.",
            14.0,
            0xff5c788a,
        )
        .upcast::<Control>(),
    );
    let status = Border::new();
    status.set_name(Some("StatusBadge".into()));
    status.set_horizontal_alignment(HorizontalAlignment::Left);
    status.set_padding(Thickness::new(12.0, 8.0, 12.0, 8.0));
    status.set_corner_radius(CornerRadius::uniform(6.0));
    status.set_background(Some(brush(0xffe3f3ef)));
    status.set_child(Some(
        label("StatusLabel", "All changes saved", 13.0, 0xff176c71).upcast::<Control>(),
    ));
    fields.children().add(status.upcast::<Control>());
    card.set_child(Some(fields.upcast::<Control>()));
    content.children().add(card.upcast::<Control>());

    let actions = StackPanel::new();
    actions.set_name(Some("Actions".into()));
    actions.set_orientation(Orientation::Horizontal);
    actions.set_spacing(12.0);
    actions
        .children()
        .add(action("SaveButton", "Save changes", true).upcast::<Control>());
    actions
        .children()
        .add(action("CancelButton", "Cancel", false).upcast::<Control>());
    content.children().add(actions.upcast::<Control>());
    let note = label(
        "Footer",
        "Control identity / layout / accessibility / design tokens",
        12.0,
        0xff5c788a,
    );
    note.set_margin(Thickness::new(0.0, 10.0, 0.0, 0.0));
    content.children().add(note.upcast::<Control>());
    page.set_child(Some(content.upcast::<Control>()));
    page
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples/SvgExport/semantics"));
    std::fs::create_dir_all(&directory)?;
    SvgPlatform::initialize();
    ferroui_harfbuzz::HarfBuzzPlatform::initialize();
    let loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(None));
    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(loader);
    FontManager::current()
        .add_font_collection(Rc::new(ferroui_fonts_inter::InterFontCollection::new()));

    let size = Size::new(800.0, 540.0);
    let page = settings();
    page.measure(size);
    page.arrange(Rect::from_size(size));
    let mut document = SvgDocument::new(size)?;
    let mut customize = |visual: &Visual, node: &mut SvgSemanticNode| {
        // Names are unique in this example. Real applications can use their own persistent
        // component IDs; duplicate names in separate name scopes need distinct stable keys.
        if let Some(name) = visual.name() {
            node.id = name;
        }
        if node.id == "SaveButton" {
            node.properties.insert(
                "design-token.background".into(),
                SvgSemanticValue::String("AccentPrimary".into()),
            );
            node.properties.insert(
                "command".into(),
                SvgSemanticValue::String("SaveWorkspace".into()),
            );
            node.properties.insert(
                "test-selector".into(),
                SvgSemanticValue::String("workspace.save".into()),
            );
            node.source = Some(SvgSemanticSource {
                uri: "source:///examples/semantic_export.rs#SaveButton".into(),
                line: None,
                column: None,
            });
        }
    };
    document.record_visual_with_semantics(
        SvgLayerOptions::new("settings", "Workspace settings"),
        &page.upcast::<Visual>(),
        &SvgSemanticCaptureOptions {
            include_text: true,
            accessibility: SvgSemanticAccessibilityMode::AutomationPeer,
            ..Default::default()
        },
        Some(&mut customize),
    )?;
    let plain = SvgExportOptions {
        fidelity: SvgFidelity::VectorOnly,
        ..SvgExportOptions::compact()
    };
    document
        .export(&plain)?
        .save(directory.join("workspace-settings-plain.svg"))?;
    let rich = document.export(&SvgExportOptions {
        semantics: SvgSemanticExportOptions::all(),
        ..plain
    })?;
    rich.save(directory.join("workspace-settings.svg"))?;
    rich.save_svgz(directory.join("workspace-settings.svgz"))?;
    println!(
        "{} UI nodes, {} semantic layer, {} draws, {} prepared-cache hit; {} SVG bytes",
        rich.report.semantic_nodes,
        rich.report.semantic_layers,
        rich.report.draw_calls,
        rich.report.prepared_vector_cache_hits,
        rich.report.output_bytes
    );
    Ok(())
}
