#![cfg_attr(target_os = "emscripten", no_main)]
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate,
    layout::{HorizontalAlignment, VerticalAlignment},
    media::{immutable::ImmutableSolidColorBrush, Color, FontManagerOptions},
    rendering::composition::ElementComposition,
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, Ref, Thickness,
};
use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Control, NewApplication,
    Panel, StackPanel, TextBlock,
};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_media::{frames::*, MediaSubscription};
use ferroui_media_browser::{BrowserTextureImporter, WebGlFrameProducer};
use ferroui_media_controls::TexturePresenter;
use ferroui_themes_fluent::FluentTheme;
use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};
use wasm_bindgen::prelude::*;
mod demo;
struct Probe {
    root: Ref<StackPanel>,
    views: Vec<Ref<TexturePresenter>>,
    mailbox: Arc<FrameMailbox>,
    budget: FrameBudget,
    phase: String,
    _subscriptions: Vec<MediaSubscription>,
}
ferroui_base::ferro_thread_local! {static PROBE:RefCell<Option<Probe>>=const {RefCell::new(None)};}
fn phase(value: impl Into<String>) {
    PROBE.with(|p| {
        if let Some(p) = p.borrow_mut().as_mut() {
            p.phase = value.into();
        }
    });
}
fn initializing() -> bool {
    PROBE.with(|p| {
        p.borrow()
            .as_ref()
            .is_some_and(|p| p.phase == "initializing")
    })
}
#[repr(C)]
pub struct App {
    base: Application,
}
ferro_class!(App:Application);
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.styles().add(FluentTheme::new().as_style());
    }
    fn on_framework_initialization_completed(this: &Self) {
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_single_view_application_lifetime())
        {
            lifetime.set_main_view(Some(create()));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
fn create() -> Ref<Control> {
    let root = StackPanel::new();
    root.set_margin(Thickness::uniform(12.));
    root.set_spacing(12.);
    let title = TextBlock::new();
    title.set_text(Some("Shared WebGL textures"));
    root.children().add(&title);
    let mailbox = Arc::new(FrameMailbox::new(700, 1));
    let budget = FrameBudget::new(FrameBudgetLimits::default());
    let mut views = Vec::new();
    let mut subscriptions = Vec::new();
    for index in 0..2 {
        let panel = Panel::new();
        panel.set_width(320.);
        panel.set_height(160.);
        panel.set_horizontal_alignment(HorizontalAlignment::Left);
        let view = TexturePresenter::new();
        view.set_frame_source(Some(mailbox.clone()), Some(Rc::new(BrowserTextureImporter)));
        subscriptions.push(
            view.errors()
                .subscribe(move |error| phase(format!("error:{error}"))),
        );
        panel.children().add(&view);
        // An ordinary sibling covers part of the video visual. It must remain
        // composited above the shared texture; no native overlay is involved.
        if index == 0 {
            let overlay = Border::new();
            overlay.set_width(24.);
            overlay.set_height(24.);
            overlay.set_horizontal_alignment(HorizontalAlignment::Left);
            overlay.set_vertical_alignment(VerticalAlignment::Top);
            overlay.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(
                Color::from_rgb(255, 0, 255),
            ))));
            panel.children().add(&overlay);
        }
        root.children().add(&panel);
        views.push(view);
    }
    let weak = views[0].downgrade();
    PROBE.with(|p| {
        *p.borrow_mut() = Some(Probe {
            root: root.clone(),
            views,
            mailbox: mailbox.clone(),
            budget: budget.clone(),
            phase: "initializing".into(),
            _subscriptions: subscriptions,
        })
    });
    DispatcherTimer::run(
        move || {
            if !initializing() {
                return false;
            }
            let Some(view) = weak.upgrade() else {
                return false;
            };
            let Some(visual) = ElementComposition::get_element_visual(&view) else {
                return true;
            };
            let compositor = visual.compositor().clone();
            let budget = budget.clone();
            let mailbox = mailbox.clone();
            Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let result = async {
                    let interop = compositor
                        .try_get_composition_gpu_interop()
                        .await
                        .ok_or("No GPU interop".to_owned())?;
                    let producer = WebGlFrameProducer::from_interop(&*interop, budget)
                        .map_err(|e| e.to_string())?;
                    let pending = producer
                        .quadrants(
                            FrameIdentity {
                                source: 700,
                                generation: 1,
                                sequence: 1,
                            },
                            16,
                            8,
                        )
                        .map_err(|e| e.to_string())?
                        .ok_or("Frame budget saturated")?;
                    let frame = pending.finish().await.map_err(|e| e.to_string())?;
                    if !initializing() {
                        return Ok(());
                    }
                    mailbox.publish(frame).map_err(|e| e.to_string())?;
                    Ok::<_, String>(())
                }
                .await;
                if !initializing() {
                    return;
                }
                match result {
                    Ok(()) => {
                        phase("ready");
                        let views = PROBE.with(|p| {
                            p.borrow()
                                .as_ref()
                                .map(|p| p.views.clone())
                                .unwrap_or_default()
                        });
                        for view in views {
                            view.present_latest();
                        }
                    }
                    Err(e) => phase(format!("error:{e}")),
                }
            });
            false
        },
        Duration::from_millis(50),
        DispatcherPriority::NORMAL,
    );
    root.upcast()
}
#[wasm_bindgen(js_name=mediaProbe)]
pub fn probe() -> String {
    PROBE.with(|p|p.borrow().as_ref().map(|p| {
        let c=p.budget.counters().snapshot();let usage=p.budget.usage();
        format!("phase={};frames={};bytes={};gpu_reads={};quarantined={};imports={};copies={};conversions={};preview_attempts={};preview_bytes={}",p.phase,usage.frames,usage.bytes,p.budget.gpu_reads(),p.budget.quarantined_reads(),c.gpu_imports,c.gpu_copies,c.gpu_conversions,c.preview.attempts,c.preview.bytes)
    }).unwrap_or_else(||"phase=starting".into()))
}
#[wasm_bindgen(js_name=clearMedia)]
pub fn clear() {
    demo::clear();
    PROBE.with(|p| {
        if let Some(p) = p.borrow_mut().as_mut() {
            p.mailbox.clear();
            for view in &p.views {
                view.set_frame_source(None, None);
            }
            p.phase = "cleared".into();
        }
    });
}
#[wasm_bindgen(js_name=runMain)]
pub fn run_main(_query: &str) {
    AppBuilder::configure::<App>()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".into()),
            ..Default::default()
        }))
        .start_browser_app("out", Some(BrowserPlatformOptions::default()));
}
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("Build with scripts/build-browser.sh portable-media-browser");
}
