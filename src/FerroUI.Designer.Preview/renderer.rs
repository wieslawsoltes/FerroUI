use crate::protocol::{Bounds, Image, Render};
use crate::rsx_placement::PlacementPolicy;
use base64::{engine::general_purpose::STANDARD, Engine};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::{AppBuilder, Application, ContentControl, Window, WindowDecorations};
use ferroui_designer::{capture_preview_stage_png, DesignerSession, LayerViewState, PreviewHostOptions, PreviewStatus};
use ferroui_designer_core::NodeId;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use ferroui_inspect::TreeKind;
use ferroui_markup_xaml::source_overlay::SourceSnapshot;
use ferroui_vello::VelloApplicationExtensions;
use serde::Serialize;
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct Renderer {
    app: Ref<Application>,
    pub(crate) stage: Option<Stage>,
}

pub(crate) struct Stage {
    pub(crate) session: Rc<RefCell<DesignerSession>>,
    pub(crate) token: u64,
    pub(crate) project: Option<u64>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) placement_policy: Option<PlacementPolicy>,
    pub(crate) project_assets: Vec<ferroui_designer::DesignerAsset>,
    pub(crate) sources: SourceSnapshot,
    _window: PreviewWindow,
}

pub(crate) struct Frame {
    pub(crate) image: Image,
    pub(crate) bounds: BTreeMap<NodeId, Bounds>,
    pub(crate) notices: Vec<String>,
    pub(crate) timeline_pose: Option<ferroui_designer::TimelinePreviewPose>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Presentation {
    image: Image,
    nodes: Vec<PresentedNode>,
    notices: Vec<String>,
    selection: Vec<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    layer_view: Option<LayerViewState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timeline_position: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timeline_targets: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) transition_position: Option<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PresentedNode {
    id: NodeId,
    bounds: Option<Bounds>,
    can_move: bool,
    can_insert: bool,
    selectable: bool,
}

impl Renderer {
    pub(crate) fn new() -> Self {
        let builder = AppBuilder::configure::<Application>()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
            .use_vello()
            .use_harfbuzz()
            .setup_without_starting();
        let app = builder.instance().expect("headless app setup");
        ferroui_themes_winui::register_types();
        app.styles()
            .add(ferroui_themes_winui::WinUITheme::new().as_style());
        Self { app, stage: None }
    }

    pub(crate) fn render(&mut self, input: &Render, source: &str, sources: SourceSnapshot, layers: Option<&LayerViewState>) -> Result<Frame, String> {
        self.stage = None;
        if !(64..=1536).contains(&input.width) || !(64..=1536).contains(&input.height) {
            return Err("Preview width and height must be between 64 and 1536".into());
        }
        let variant = if input.dark {
            ThemeVariant::dark()
        } else {
            ThemeVariant::light()
        };
        self.app.set_requested_theme_variant(Some(variant));
        let host = ContentControl::new();
        host.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        host.set_vertical_content_alignment(VerticalAlignment::Stretch);
        let mut options = PreviewHostOptions::default();
        if let Some(data) = &input.design_data {
            if let Some(name) = &data.fixture {
                ferroui_designer_core::design_fixtures::validate_name(name)?;
            }
            options.design_time_data = data.enabled;
            options.design_fixture = data.fixture.clone();
        }
        let mut session = DesignerSession::with_preview_options(source, host.clone(), input.document.uri.clone(), options)?;
        match session.preview_status() {
            PreviewStatus::Ready { .. } => (),
            PreviewStatus::InvalidSource { .. } => {
                return Err("Repair the source syntax to resume preview".into())
            }
            PreviewStatus::Failed { message, .. } => return Err(message.clone()),
        }
        let window = PreviewWindow(Window::new());
        window.0.set_window_decorations(WindowDecorations::None);
        window.0.set_width(f64::from(input.width));
        window.0.set_height(f64::from(input.height));
        window.0.set_content(Some(Rc::new(host.clone())));
        window.0.show();
        if let Some(layers) = layers.filter(|layers| !layers.is_empty()) {
            session.restore_persisted_layer_view(layers);
        }
        if let Some(targets) = &input.timeline_targets {
            session.restore_timeline_targets(targets);
        }
        let pose = input.timeline_pose.as_ref().map(|pose| session.restore_timeline_preview_pose(pose));
        let mut frame = capture(&mut session)?;
        match pose {
            Some(Ok(pose)) => frame.timeline_pose = pose,
            Some(Err(error)) => frame.notices.push(format!("Animation preview: {error}")),
            None => {}
        }
        self.stage = Some(Stage {
            session: Rc::new(RefCell::new(session)),
            token: input.frame,
            project: input.document.project,
            width: input.width,
            height: input.height,
            placement_policy: None,
            project_assets: Vec::new(),
            sources,
            _window: window,
        });
        Ok(frame)
    }

    pub(crate) fn present(&mut self, next: u64, include_layers: bool) -> Result<Presentation, String> {
        let stage = self.stage.as_mut().ok_or("Render the design first")?;
        stage.token = next;
        let mut session = stage.session.borrow_mut();
        let frame = stage.sources.run(|| capture(&mut session))?;
        let nodes = session
            .document()
            .ast()
            .elements()
            .map(|node| PresentedNode {
                id: node.id,
                bounds: frame.bounds.get(&node.id).copied(),
                can_move: session.can_move_outline_node(node.id),
                can_insert: session.can_insert_outline_child(node.id),
                selectable: session.can_canvas_select(node.id),
            })
            .collect();
        Ok(Presentation {
            image: frame.image,
            nodes,
            notices: frame.notices,
            selection: session.selection().to_vec(),
            layer_view: include_layers.then(|| session.persist_layer_view()),
            timeline_position: None,
            timeline_targets: None,
            transition_position: None,
        })
    }
}

pub(crate) fn capture(session: &mut DesignerSession) -> Result<Frame, String> {
        let host = session.preview_host();
        Dispatcher::ui_thread().run_jobs(None);
        host.update_layout();
        session.refresh_runtime_mapping();
        let control = host.upcast();
        let png = capture_preview_stage_png(&control, Default::default())?;
        let bounds: BTreeMap<NodeId, Bounds> = session
            .runtime_tree(TreeKind::Visual)
            .into_iter()
            .filter_map(|node| {
                let id = node.source_id?;
                let rect = node.bounds?;
                [rect.x, rect.y, rect.width, rect.height]
                    .iter()
                    .all(|v| v.is_finite())
                    .then_some((
                        id,
                        Bounds {
                            x: rect.x,
                            y: rect.y,
                            width: rect.width,
                            height: rect.height,
                        },
                    ))
            })
            .collect();
        let notices = session.preview_notices().to_vec();
        Ok(Frame {
            timeline_pose: None,
            image: Image {
                png: STANDARD.encode(png.png),
                width: png.pixel_width,
                height: png.pixel_height,
            },
            bounds,
            notices,
        })
}

// The headless window roots layout and styles for the duration of capture.
struct PreviewWindow(Ref<Window>);

impl Drop for PreviewWindow {
    fn drop(&mut self) {
        self.0.close();
    }
}
