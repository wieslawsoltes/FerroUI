use ferroui_base::{
    layout::{HorizontalAlignment, VerticalAlignment},
    rendering::composition::{frames::*, ICompositionGpuInterop},
    threading::{DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::Panel;
use ferroui_media_controls::RendererHost;
use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};
#[path = "../PortableMedia.Mobile/scene.rs"]
mod scene;
static BUDGET: OnceLock<FrameBudget> = OnceLock::new();
static PROBES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn probes_complete() -> bool {
    !std::env::args().any(|a| a == "--renderer")
        || PROBES.load(std::sync::atomic::Ordering::SeqCst)
            >= if std::env::args().any(|a| a == "--device-recovery-smoke") {
                2
            } else {
                1
            }
}

pub fn retained() -> bool {
    BUDGET.get().is_some_and(|b| b.usage().frames != 0)
}
pub fn install(
    root: &Ref<Panel>,
    source: Arc<FrameMailbox>,
    interop: &dyn ICompositionGpuInterop,
) -> Option<Ref<RendererHost>> {
    if !std::env::args().any(|a| a == "--renderer") {
        return None;
    }
    let budget = FrameBudget::new(Default::default());
    let _ = BUDGET.set(budget.clone());
    let id = ferroui_media::allocate_source_id();
    let renderer = match ferroui_media_metal::MetalTextureRenderer::from_interop(
        interop,
        budget.clone(),
        id,
        Arc::new(scene::Scene::default()),
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("RENDERER_SETUP_FAILED {e}");
            super::FAILED.store(true, super::Ordering::Relaxed);
            return None;
        }
    };
    let host = RendererHost::new();
    host.set_width(220.);
    host.set_height(220.);
    host.set_horizontal_alignment(HorizontalAlignment::Right);
    host.set_vertical_alignment(VerticalAlignment::Bottom);
    host.set_renderer(
        Some(std::rc::Rc::new(renderer)),
        id,
        Some(std::rc::Rc::new(
            ferroui_media_video_apple::AppleTextureImporter,
        )),
    );
    let recovery_budget = budget.clone();
    host.set_renderer_factory(Some(std::rc::Rc::new(move |interop, id| {
        Ok((
            std::rc::Rc::new(ferroui_media_metal::MetalTextureRenderer::from_interop(
                interop,
                recovery_budget.clone(),
                id,
                Arc::new(scene::Scene::default()),
            )?),
            std::rc::Rc::new(ferroui_media_video_apple::AppleTextureImporter),
        ))
    })));
    host.set_inputs(vec![source.clone()]);
    root.children().add(&host);
    let errors = host.errors().subscribe(|e| {
        eprintln!("RENDERER_ERROR {e}");
        super::FAILED.store(true, super::Ordering::Relaxed);
    });
    let weak = host.downgrade();
    let probed = std::cell::Cell::new(None::<u64>);
    DispatcherTimer::run(
        move || {
            let _keep_errors = &errors;
            let Some(host) = weak.upgrade() else {
                return false;
            };
            if std::env::args().any(|a| a == "--smoke") {
                if let Some(frame) = host
                    .output()
                    .and_then(|s| s.latest())
                    .filter(|f| probed.get() != Some(f.metadata().identity.source))
                {
                    probed.set(Some(frame.metadata().identity.source));
                    match scene::probe(&frame, std::env::args().any(|v| v == "--geometry-smoke")) {
                        Ok(()) => {
                            PROBES.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            println!("RENDERER_PIXELS_PASS transparent projection coverage");
                        }
                        Err(e) => {
                            eprintln!("RENDERER_PIXELS_FAILED {e}");
                            super::FAILED.store(true, super::Ordering::Relaxed);
                        }
                    }
                }
            }
            println!(
                "RENDERER_SHARED_INPUT {:?} output={:?} budget={:?} counters={:?}",
                source.latest().map(|v| v.metadata().identity),
                host.output()
                    .and_then(|s| s.latest())
                    .map(|v| v.metadata().identity),
                budget.usage(),
                budget.counters().snapshot()
            );
            true
        },
        Duration::from_millis(400),
        DispatcherPriority::NORMAL,
    );
    Some(host)
}
