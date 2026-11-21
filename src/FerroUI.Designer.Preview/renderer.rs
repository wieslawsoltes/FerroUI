use crate::protocol::{Bounds, Image, Render};
use crate::rsx_animation::AuthoredRsxFrame;
use crate::rsx_placement::PlacementPolicy;
use base64::{engine::general_purpose::STANDARD, Engine};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::{AppBuilder, Application, ContentControl, Window, WindowDecorations};
use ferroui_designer::{
    capture_preview_stage_png, DesignerSession, LayerViewState, PreviewHostOptions, PreviewStatus,
    PreviewTheme,
};
use ferroui_designer_core::NodeId;
use ferroui_designer_project::RustModuleSources;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use ferroui_inspect::TreeKind;
use ferroui_markup_xaml::source_overlay::SourceSnapshot;
use ferroui_markup_xaml::styling::StyleInclude;
use ferroui_vello::VelloApplicationExtensions;
use serde::Serialize;
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct Renderer {
    app: Ref<Application>,
    pub(crate) stage: Option<Stage>,
    item_template_scope: Option<RetainedItemTemplate>,
    pub(crate) accepted_rust_module_document: Option<String>,
}

struct RetainedItemTemplate {
    uri: String,
    project: Option<u64>,
    scope: ferroui_designer::ItemTemplateScopeSnapshot,
}

pub(crate) struct Stage {
    pub(crate) session: Rc<RefCell<DesignerSession>>,
    pub(crate) studio: ferroui_designer::mcp::DesignerMcp,
    pub(crate) token: u64,
    pub(crate) project: Option<u64>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) placement_policy: Option<PlacementPolicy>,
    pub(crate) authored_rsx: Option<AuthoredRsxFrame>,
    pub(crate) rsx_spacing: Option<crate::spacing::RsxOwners>,
    pub(crate) project_assets: Vec<ferroui_designer::DesignerAsset>,
    pub(crate) sources: SourceSnapshot,
    pub(crate) canvas_snap: RefCell<Option<crate::canvas_snap::PreparedSnap>>,
    _window: PreviewWindow,
}

impl Stage {
    pub(crate) fn rust_modules(
        &self,
        source: &crate::protocol::Source,
    ) -> Option<Rc<RustModuleSources>> {
        let rust = self.authored_rsx.as_ref()?;
        if rust.uri != source.uri || rust.source() != source.source || self.project != source.project {
            return None;
        }
        rust.module_sources()
    }
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
    definition_scope: Option<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    layer_view: Option<LayerViewState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timeline_position: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timeline_targets: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) transition_position: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) open_definition: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) open_rust_template: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) return_to_consumer: Option<String>,
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
        ferroui_controls_treedatagrid::register_types();
        ferroui_themes_winui::register_types();
        app.styles()
            .add(ferroui_themes_winui::WinUITheme::new().as_style());
        let table_styles = StyleInclude::new(None);
        table_styles.set_source(Some(
            "ferres://FerroUI.Controls.TreeDataGrid/Themes/Generic.xaml"
                .parse()
                .expect("built-in TreeDataGrid style URI"),
        ));
        app.styles().add(table_styles.as_style());
        Self {
            app,
            stage: None,
            item_template_scope: None,
            accepted_rust_module_document: None,
        }
    }

    pub(crate) fn take_stage(&mut self) -> Option<Stage> {
        let stage = self.stage.take()?;
        {
            let session = stage.session.borrow();
            self.item_template_scope = session.item_template_scope_snapshot().map(|scope| {
                RetainedItemTemplate {
                    uri: session.document_uri().into(),
                    project: stage.project,
                    scope,
                }
            });
        }
        Some(stage)
    }

    pub(crate) fn retire_frame(&mut self, input: &Render) {
        self.take_stage();
        if self.item_template_scope.as_ref().is_some_and(|saved| {
            saved.uri != input.document.uri
                || saved.project != input.document.project
                || input.document.language != crate::protocol::Language::Xaml
        }) {
            self.item_template_scope = None;
        }
    }

    pub(crate) fn render(
        &mut self, input: &Render, source: &str, sources: SourceSnapshot,
        includes: ferroui_designer_project::PreviewIncludes, layers: Option<&LayerViewState>,
    ) -> Result<Frame, String> {
        if !(64..=1536).contains(&input.width) || !(64..=1536).contains(&input.height) {
            return Err("Preview width and height must be between 64 and 1536".into());
        }
        let theme = input.theme.unwrap_or(if input.dark {
            PreviewTheme::Dark
        } else {
            PreviewTheme::Light
        });
        self.app.set_requested_theme_variant(theme.resolve()?);
        let host = ContentControl::new();
        host.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        host.set_vertical_content_alignment(VerticalAlignment::Stretch);
        let mut options = PreviewHostOptions::default();
        if let Some(data) = &input.design_data {
            options.design_time_data = data.enabled;
        }
        let fixture = input.design_data.as_ref().and_then(|data| data.fixture.clone());
        let mut session = DesignerSession::with_preview_fixture(
            source, host.clone(), input.document.uri.clone(), options, fixture,
        )?;
        match session.preview_status() {
            PreviewStatus::Ready { .. } => (),
            PreviewStatus::InvalidSource { .. } => {
                return Err("Repair the source syntax to resume preview".into())
            }
            PreviewStatus::Failed { message, .. } => return Err(message.clone()),
        }
        session.set_accepted_preview_includes(includes);
        let window = PreviewWindow(Window::new());
        window.0.set_window_decorations(WindowDecorations::None);
        window.0.set_width(f64::from(input.width));
        window.0.set_height(f64::from(input.height));
        window.0.set_content(Some(Rc::new(host.clone())));
        window.0.show();
        if let Some(layers) = layers.filter(|layers| !layers.is_empty()) {
            session.restore_persisted_layer_view(layers);
        }
        let item_template_error = self
            .item_template_scope
            .as_ref()
            .and_then(|saved| session.restore_item_template_scope(&saved.scope).err());
        if let Some(targets) = &input.timeline_targets {
            session.restore_timeline_targets(targets);
        }
        let pose = input.timeline_pose.as_ref().map(|pose| session.restore_timeline_preview_pose(pose));
        let mut frame = capture(&mut session)?;
        if let Some(error) = item_template_error {
            frame.notices.push(format!("Item template preview: {error}"));
        }
        match pose {
            Some(Ok(pose)) => frame.timeline_pose = pose,
            Some(Err(error)) => frame.notices.push(format!("Animation preview: {error}")),
            None => {}
        }
        let session = Rc::new(RefCell::new(session));
        self.stage = Some(Stage {
            studio: ferroui_designer::mcp::DesignerMcp::new(&session),
            session,
            token: input.frame,
            project: input.document.project,
            width: input.width,
            height: input.height,
            placement_policy: None,
            authored_rsx: None,
            rsx_spacing: None,
            project_assets: Vec::new(),
            sources,
            canvas_snap: RefCell::new(None),
            _window: window,
        });
        self.item_template_scope = None;
        Ok(frame)
    }

    pub(crate) fn present(&mut self, next: u64, include_layers: bool) -> Result<Presentation, String> {
        let stage = self.stage.as_mut().ok_or("Render the design first")?;
        stage.token = next;
        stage.studio.invalidate_spacing();
        let mut session = stage.session.borrow_mut();
        let frame = stage.sources.run(|| capture(&mut session))?;
        let nodes = session
            .document()
            .ast()
            .elements()
            .map(|node| PresentedNode {
                id: node.id,
                bounds: frame.bounds.get(&node.id).copied(),
                can_move: session.can_move_outline_node(node.id)
                    && stage.placement_policy.as_ref().is_none_or(|policy| {
                        node.parent.is_some_and(|parent| policy.editable_children(parent))
                    }),
                can_insert: session.can_insert_outline_child(node.id)
                    && stage.placement_policy.as_ref().is_none_or(|policy| policy.editable_children(node.id)),
                selectable: session.can_canvas_select(node.id),
            })
            .collect();
        Ok(Presentation {
            image: frame.image,
            nodes,
            notices: frame.notices,
            selection: session.selection().to_vec(),
            definition_scope: session.definition_scope().map(|scope| scope.node),
            layer_view: include_layers.then(|| session.persist_layer_view()),
            timeline_position: None,
            timeline_targets: None,
            transition_position: None,
            open_definition: None,
            open_rust_template: None,
            return_to_consumer: None,
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
