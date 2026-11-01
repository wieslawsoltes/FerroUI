//! Repeated full native lifecycle suites in one process, with resource samples.
use super::*;
use crate::desktop::providers;
use ferroui_media_opengl::NativeGlFrameProducer;
use ferroui_media_windows::NativeDevice;
struct Watchdog(Rc<dyn IDisposable>);
impl Drop for Watchdog {
    fn drop(&mut self) {
        self.0.dispose();
    }
}
async fn exercise(
    state: Rc<State>,
    window: Ref<Window>,
    budget: FrameBudget,
) -> Result<(), String> {
    let cycles = std::env::args()
        .find_map(|a| a.strip_prefix("--camera-stress-cycles=").map(str::to_owned))
        .ok_or("No stress cycle count")?
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    if !(1..=1000).contains(&cycles) {
        return Err("Stress cycles must be 1..1000".into());
    }
    let started = Instant::now();
    let mut baseline = None;
    let mut peak = [0u64; 4];
    let mut last = [0; 4];
    let workload = std::env::args()
        .find_map(|a| a.strip_prefix("--memory-workload=").map(str::to_owned))
        .unwrap_or("full".into());
    if !matches!(workload.as_str(), "full" | "capture" | "reset") {
        return Err("Memory workload must be full, capture or reset".into());
    }
    let resets = if workload == "capture" { 0 } else { 6 };
    std::println!("CAMERA_STRESS_BEGIN cycles={cycles} workload={workload} resets_per_cycle={resets} warmup_cycles=5 private_growth_limit=134217728 gpu_growth_limit=134217728 handle_growth_limit=64 quiet={} png={}",crate::QUIET.load(Ordering::Relaxed),!std::env::args().any(|a|a=="--no-png-diagnostics"));
    for cycle in 0..cycles {
        let weak = window.downgrade();
        let _watchdog = Watchdog(DispatcherTimer::run_once(
            move || {
                FAILED.store(true, Ordering::Relaxed);
                eprintln!("CAMERA_STRESS_TIMEOUT cycle={cycle}");
                if let Some(w) = weak.upgrade() {
                    w.close();
                }
            },
            Duration::from_secs(90),
            DispatcherPriority::NORMAL,
        ));
        window.set_width(if cycle % 2 == 0 { 800. } else { 1000. });
        window.set_height(if cycle % 2 == 0 { 500. } else { 700. });
        let root = state.root.upgrade().ok_or("No stress root")?;
        let compositor = ElementComposition::get_element_visual(&root)
            .ok_or("No stress compositor")?
            .compositor()
            .clone();
        let interop = compositor
            .try_get_composition_gpu_interop()
            .await
            .ok_or("No stress context")?;
        let capture = state.capture.borrow().clone().ok_or("No stress capture")?;
        capture.rebind_provider(
            providers::camera(&*interop, budget.clone()).map_err(|e| e.to_string())?,
        );
        state.interop.replace(Some(interop));
        if workload != "reset" {
            super::exercise(state.clone(), budget.clone()).await?;
        }
        if workload == "full" {
            super::recovery::exercise(state.clone(), window.clone(), budget.clone()).await?;
        }
        if workload == "reset" {
            for _ in 0..6 {
                let job = compositor.invoke_server_job_async(
                    |server| {
                        server.reset_all_gpu_resources();
                        server.render_interface().value();
                        Ok(())
                    },
                    false,
                );
                super::wait(move || match job.take_result() {
                    Some(r) => r.map(Some).map_err(|e| e.to_string()),
                    None => Ok(None),
                })
                .await?;
                super::delay().await?;
            }
        }
        let interop = compositor
            .try_get_composition_gpu_interop()
            .await
            .ok_or("No metrics context")?;
        let producer = NativeGlFrameProducer::from_interop(&*interop, budget.clone())
            .map_err(|e| e.to_string())?;
        let device = NativeDevice::from_renderer(&producer)
            .await
            .map_err(|e| e.to_string())?;
        let sample = device.fixture_process_stats().map_err(|e| e.to_string())?;
        for i in 0..4 {
            if sample[i] != u64::MAX {
                peak[i] = peak[i].max(sample[i]);
            }
        }
        last = sample;
        std::println!("CAMERA_STRESS_SAMPLE cycle={} elapsed_ms={} private_bytes={} working_set_bytes={} local_gpu_bytes={} handles={} frames={} reads={} quarantine={} native={:?}",cycle+1,started.elapsed().as_millis(),sample[0],sample[1],sample[2],sample[3],budget.usage().frames,budget.gpu_reads(),budget.quarantined_reads(),NativeSession::synthetic_usage());
        #[cfg(feature = "memory-audit")]
        {
            let rust = crate::memory_audit::snapshot();
            let native = NativeDevice::fixture_memory_stats();
            std::println!("CAMERA_MEMORY_SAMPLE cycle={} rust_live_bytes={} rust_live_blocks={} rust_peak_bytes={} rust_requested_growth_bytes={} default_heap_allocated={} default_heap_committed={} private_committed={} mapped_committed={} image_committed={}",cycle+1,rust[0],rust[1],rust[2],rust[3],native[0],native[1],native[2],native[3],native[4]);
        }
        if cycle == 4 {
            baseline = Some(sample);
        }
        if let Some(base) = baseline {
            if sample[0] > base[0] + 128 * 1024 * 1024
                || sample[3] > base[3] + 64
                || (sample[2] != u64::MAX
                    && base[2] != u64::MAX
                    && sample[2] > base[2] + 128 * 1024 * 1024)
            {
                return Err(format!(
                    "Stress exceeded warmed resource limits baseline={base:?} sample={sample:?}"
                ));
            }
        }
    }
    std::println!("CAMERA_STRESS_PASS cycles={cycles} resets={} workload={workload} elapsed_ms={} baseline={baseline:?} last={last:?} peak={peak:?} preview={:?}",cycles*resets,started.elapsed().as_millis(),budget.counters().snapshot().preview);
    Ok(())
}
pub async fn run(state: Rc<State>, window: Ref<Window>, budget: FrameBudget) {
    if let Err(e) = exercise(state.clone(), window.clone(), budget).await {
        FAILED.store(true, Ordering::Relaxed);
        eprintln!("CAMERA_STRESS_FAILED {e}");
        eprintln!(
            "CAMERA_STRESS_RETIREMENT_DIAGNOSTIC before={:?} photo={:?}",
            NativeSession::synthetic_usage(),
            NativeSession::fixture_photo_status()
        );
        if NativeSession::synthetic_usage() == (0, 1, 0) {
            // Diagnose the failed run without turning a reset into a pass.
            // A D3D pipeline cache is released by context disposal; an unknown
            // native photo query must remain quarantined even after that reset.
            if let Some(root) = state.root.upgrade() {
                if let Some(visual) = ElementComposition::get_element_visual(&root) {
                    let mut device = None;
                    if let Some(interop) =
                        visual.compositor().try_get_composition_gpu_interop().await
                    {
                        if let Ok(producer) = NativeGlFrameProducer::from_interop(
                            &*interop,
                            FrameBudget::new(Default::default()),
                        ) {
                            device = NativeDevice::from_renderer(&producer).await.ok();
                        }
                    }
                    if let Some(device) = &device {
                        eprintln!(
                            "CAMERA_STRESS_NATIVE_HANDLES before={:?}",
                            device.fixture_retirement_stats()
                        );
                    }
                    let job = visual.compositor().invoke_server_job_async(
                        |server| {
                            server.reset_all_gpu_resources();
                            server.render_interface().value();
                            Ok(())
                        },
                        false,
                    );
                    let _ = super::wait(move || match job.take_result() {
                        Some(v) => v.map(Some).map_err(|e| e.to_string()),
                        None => Ok(None),
                    })
                    .await;
                    let _ = super::delay().await;
                    eprintln!(
                        "CAMERA_STRESS_RETIREMENT_DIAGNOSTIC after_reset={:?} photo={:?}",
                        NativeSession::synthetic_usage(),
                        NativeSession::fixture_photo_status()
                    );
                    if let Some(device) = &device {
                        eprintln!(
                            "CAMERA_STRESS_NATIVE_HANDLES after_reset={:?}",
                            device.fixture_retirement_stats()
                        );
                    }
                }
            }
        }
    }
    state.stop();
    window.close();
}
