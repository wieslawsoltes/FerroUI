use crate::protocol::{Bounds, Image, Render};
use base64::{engine::general_purpose::STANDARD, Engine};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::{AppBuilder, Application, ContentControl, Window, WindowDecorations};
use ferroui_designer::{capture_preview_stage_png, DesignerSession, PreviewStatus};
use ferroui_designer_core::NodeId;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use ferroui_inspect::TreeKind;
use ferroui_vello::VelloApplicationExtensions;
use std::collections::BTreeMap;
use std::rc::Rc;

pub(crate) struct Renderer {
    app: Ref<Application>,
}

pub(crate) struct Frame {
    pub(crate) image: Image,
    pub(crate) bounds: BTreeMap<NodeId, Bounds>,
    pub(crate) notices: Vec<String>,
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
        Self { app }
    }

    pub(crate) fn render(&self, input: &Render, source: &str) -> Result<Frame, String> {
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
        let mut session =
            DesignerSession::with_uri(source, host.clone(), input.document.uri.clone())?;
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
        Dispatcher::ui_thread().run_jobs(None);
        host.update_layout();
        session.refresh_runtime_mapping();
        let control = host.upcast();
        let png = capture_preview_stage_png(&control, Default::default())?;
        let bounds = session
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
        Ok(Frame {
            image: Image {
                png: STANDARD.encode(png.png),
                width: png.pixel_width,
                height: png.pixel_height,
            },
            bounds,
            notices: session.preview_notices().to_vec(),
        })
    }
}

// The headless window roots layout and styles for the duration of capture.
struct PreviewWindow(Ref<Window>);

impl Drop for PreviewWindow {
    fn drop(&mut self) {
        self.0.close();
    }
}
