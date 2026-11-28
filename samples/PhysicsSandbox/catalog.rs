//! Navigation metadata and task-oriented entry points for the studio.

use crate::icons::{icon_label, icon_view, Icons};
use crate::scenes::SceneKind;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::{Border, TextBlock};
use ferroui_view::prelude::*;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StudioPage {
    Home,
    CompositorShaders,
    LiveUiTree,
    NodeGraph,
    Scene(SceneKind),
}

impl StudioPage {
    pub(crate) fn from_key(key: &str) -> Self {
        match key {
            "compositor" => Self::CompositorShaders,
            "ui" => Self::LiveUiTree,
            "nodegraph" => Self::NodeGraph,
            key => SceneKind::ALL
                .into_iter()
                .find(|scene| scene.key() == key)
                .map(Self::Scene)
                .unwrap_or(Self::Home),
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Home => "Overview",
            Self::CompositorShaders => "Composition & effects",
            Self::LiveUiTree => "Interactive workspace",
            Self::NodeGraph => "Parametric workspace",
            Self::Scene(kind) => kind.title(),
        }
    }

    pub(crate) fn icon(self) -> &'static str {
        match self {
            Self::Home => Icons::HOME,
            Self::CompositorShaders => Icons::SPARKLE,
            Self::LiveUiTree => Icons::LAYERS,
            Self::NodeGraph => Icons::NODE_GRAPH,
            Self::Scene(SceneKind::Playground) => Icons::CUBE,
            Self::Scene(SceneKind::GpuParticles) => Icons::WATER,
            Self::Scene(SceneKind::ClothSoftBody) => Icons::CLOTH,
            Self::Scene(SceneKind::UiSprings) => Icons::UI_MOTION,
            Self::Scene(SceneKind::RagdollWrecking) => Icons::SHATTER,
            Self::Scene(SceneKind::VehicleTerrain) => Icons::VEHICLE,
            Self::Scene(SceneKind::CharacterDemo) => Icons::PERSON,
            Self::Scene(SceneKind::LaserGravity) => Icons::ORBIT,
        }
    }
}

fn gallery_card(
    page: StudioPage,
    description: &'static str,
    navigate: Rc<dyn Fn(StudioPage)>,
) -> Element {
    button()
        .width(288.0)
        .min_height(156.0)
        .padding(20.0)
        .horizontal_content_alignment(HorizontalAlignment::Stretch)
        .vertical_content_alignment(VerticalAlignment::Top)
        .content(
            stack_panel()
                .spacing(14.0)
                .child(
                    border()
                        .width(32.0)
                        .height(32.0)
                        .horizontal_alignment(HorizontalAlignment::Left)
                        .child(icon_view(page.icon(), 24.0)),
                )
                .child(
                    text_block()
                        .text(page.title())
                        .font_size(16.0)
                        .font_weight(FontWeight::SemiBold),
                )
                .child(
                    text_block()
                        .text(description)
                        .font_size(14.0)
                        .dynamic_resource(
                            TextBlock::foreground_property(),
                            "TextFillColorSecondaryBrush",
                        )
                        .text_wrapping(TextWrapping::Wrap),
                ),
        )
        .on_click(move |_| navigate(page))
        .into()
}

fn section_title(title: &'static str, description: &'static str) -> Element {
    stack_panel()
        .spacing(6.0)
        .child(
            text_block()
                .text(title)
                .font_size(20.0)
                .font_weight(FontWeight::SemiBold),
        )
        .child(
            text_block()
                .text(description)
                .font_size(14.0)
                .dynamic_resource(
                    TextBlock::foreground_property(),
                    "TextFillColorSecondaryBrush",
                )
                .text_wrapping(TextWrapping::Wrap),
        )
        .into()
}

pub(crate) fn build_home_page(gpu_available: bool, navigate: Rc<dyn Fn(StudioPage)>) -> Element {
    let open_ui = navigate.clone();
    let open_playground = navigate.clone();
    let hero = border()
        .dynamic_resource(Border::background_property(), "CardBackgroundFillColorDefaultBrush")
        .dynamic_resource(Border::border_brush_property(), "CardStrokeColorDefaultBrush")
        .border_thickness(1.0).corner_radius(8.0).padding(24.0)
        .child(stack_panel().spacing(16.0)
            .child(text_block().text("Build interfaces that feel alive.").font_size(28.0).font_weight(FontWeight::SemiBold)
                .text_wrapping(TextWrapping::Wrap))
            .child(text_block().text("Explore purposeful motion, direct manipulation, and physical simulation. Every workspace has a clear task, live controls, and a reset path so you can experiment confidently.")
                .font_size(14.0).max_width(760.0).horizontal_alignment(HorizontalAlignment::Left)
                .dynamic_resource(TextBlock::foreground_property(), "TextFillColorSecondaryBrush")
                .text_wrapping(TextWrapping::Wrap))
            .child(wrap_panel().item_spacing(8.0).line_spacing(8.0)
                .child(button().class("accent").padding((16.0, 10.0, 16.0, 10.0))
                    .content(icon_label(Icons::LAYERS, "Open interactive workspace", 16.0))
                    .on_click(move |_| open_ui(StudioPage::LiveUiTree)))
                .child(button().padding((16.0, 10.0, 16.0, 10.0))
                    .content("Explore the physics lab")
                    .on_click(move |_| open_playground(StudioPage::Scene(SceneKind::Playground)))))
            .child(text_block().text(if gpu_available {
                "GPU compute available · CPU fallback available · Light and dark themes"
            } else {
                "CPU simulation ready · Light and dark themes"
            }).font_size(12.0).text_wrapping(TextWrapping::Wrap)
                .dynamic_resource(TextBlock::foreground_property(), "TextFillColorSecondaryBrush")));

    let interfaces = wrap_panel().item_spacing(12.0).line_spacing(12.0)
        .child(gallery_card(StudioPage::LiveUiTree,
            "Edit a document, triage an inbox, and adjust a task using spring-driven built-in controls.", navigate.clone()))
        .child(gallery_card(StudioPage::CompositorShaders,
            "Compare motion and visual effects on real workspace cards, then tune the response.", navigate.clone()))
        .child(gallery_card(StudioPage::NodeGraph,
            "Connect parameters to simulation, inspect the graph, and explore complete presets.", navigate.clone()));

    let mut experiments = wrap_panel().item_spacing(12.0).line_spacing(12.0);
    for kind in SceneKind::ALL {
        experiments = experiments.child(gallery_card(
            StudioPage::Scene(kind),
            kind.description(),
            navigate.clone(),
        ));
    }

    scroll_viewer().padding(24.0)
        .content(stack_panel().spacing(28.0)
            .child(hero)
            .child(section_title("Interface patterns", "Practical interactions composed from FerroUI controls and the WinUI 3 theme."))
            .child(interfaces)
            .child(section_title("Simulation lab", "Eight focused experiments. Select a scene to see the controls and tools that apply to it."))
            .child(experiments))
        .into()
}
