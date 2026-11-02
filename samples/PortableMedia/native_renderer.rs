use ferroui_base::{
    layout::{HorizontalAlignment, VerticalAlignment},
    rendering::composition::{frames::*, ICompositionGpuInterop},
    threading::{DispatcherPriority, DispatcherTimer},
    Ref, WeakRef,
};
use ferroui_controls::Panel;
use ferroui_media_controls::RendererHost;
use std::{cell::RefCell, sync::Arc, time::Duration};
#[path = "gl_scene.rs"]
mod scene;
thread_local! {
    static BUDGETS: RefCell<Vec<(WeakRef<RendererHost>,FrameBudget)>> = const {RefCell::new(Vec::new())};
}
pub fn retained() -> bool {
    BUDGETS.with(|budgets| {
        let mut budgets = budgets.borrow_mut();
        budgets.retain(|(host, budget)| host.upgrade().is_some() || budget.usage().frames != 0 || budget.gpu_reads() != 0 || budget.quarantined_reads() != 0);
        budgets.iter().any(|(_, budget)| budget.usage().frames != 0 || budget.gpu_reads() != 0 || budget.quarantined_reads() != 0)
    })
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
    let id = ferroui_media::allocate_source_id();
    let renderer =
        match ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget.clone())
            .map(|producer| {
                ferroui_media_opengl::GlTextureRenderer::new(
                    producer,
                    id,
                    Arc::new(scene::Scene::default()),
                )
            }) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("RENDERER_SETUP_FAILED {e}");
                crate::desktop::FAILED.store(true, crate::desktop::Ordering::Relaxed);
                return None;
            }
        };
    let host = RendererHost::new();
    // Camera restart creates a new host/budget. Track retiring generations too,
    // rather than only the first host ever installed by this process.
    let _ = retained();
    BUDGETS.with(|budgets| {
        budgets
            .borrow_mut()
            .push((host.downgrade(), budget.clone()))
    });
    host.set_width(220.);
    host.set_height(220.);
    host.set_horizontal_alignment(HorizontalAlignment::Right);
    host.set_vertical_alignment(VerticalAlignment::Bottom);
    host.set_renderer(
        Some(std::rc::Rc::new(renderer)),
        id,
        Some(crate::desktop::providers::importer()),
    );
    let recovery_budget = budget.clone();
    host.set_renderer_factory(Some(std::rc::Rc::new(move |interop, id| {
        let producer = ferroui_media_opengl::NativeGlFrameProducer::from_interop(
            interop,
            recovery_budget.clone(),
        )?;
        Ok((
            std::rc::Rc::new(ferroui_media_opengl::GlTextureRenderer::new(
                producer,
                id,
                Arc::new(scene::Scene::default()),
            )),
            crate::desktop::providers::importer(),
        ))
    })));
    host.set_inputs(vec![source.clone()]);
    root.children().add(&host);
    let errors = host.errors().subscribe(|e| {
        eprintln!("RENDERER_ERROR {e}");
        crate::desktop::FAILED.store(true, crate::desktop::Ordering::Relaxed);
    });
    let weak = host.downgrade();
    DispatcherTimer::run(
        move || {
            let _keep_errors = &errors;
            let Some(host) = weak.upgrade() else {
                return false;
            };
            println!(
                "RENDERER_SHARED_INPUT {:?} output={:?} budget={:?} reads={} quarantine={} counters={:?}",
                source.latest().map(|v| v.metadata().identity),
                host.output()
                    .and_then(|s| s.latest())
                    .map(|v| v.metadata().identity),
                budget.usage(),
                budget.gpu_reads(),
                budget.quarantined_reads(),
                budget.counters().snapshot()
            );
            true
        },
        Duration::from_millis(400),
        DispatcherPriority::NORMAL,
    ).detach();
    Some(host)
}
