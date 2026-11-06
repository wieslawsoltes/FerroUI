//! Same-process AVFoundation/GPU lifetime and native-memory acceptance probe.
//! Fixed limits are emitted before warm-up; never inferred from a passing run.
use super::{
    macos_diagnostics::memory,
    native_pixels::{settle, wait},
    *,
};
use ferroui_media::{MediaErrorKind, MediaFuture};
use ferroui_media_video::{MediaPlayer, PlaybackIntent};
use std::task::{Context, Poll, Waker};
const MIB: u64 = 1024 * 1024;
const FOOTPRINT_GROWTH: u64 = 64 * MIB;
const METAL_GROWTH: u64 = 32 * MIB;
const PORT_GROWTH: u64 = 64;
async fn cancelled<T>(operation: MediaFuture<T>) -> Result<(), String> {
    match operation.await {
        Err(e) if e.kind == MediaErrorKind::Cancelled => Ok(()),
        _ => Err("Superseded native operation did not return Cancelled".into()),
    }
}
async fn retired(budget: FrameBudget) -> Result<(), String> {
    wait(move || {
        Ok((budget.usage().frames == 0
            && budget.gpu_reads() == 0
            && budget.quarantined_reads() == 0
            && !renderer::retained()
            && ferroui_media_video_apple::native_live_sessions() == 0)
            .then_some(()))
    })
    .await
}
async fn frame(player: MediaPlayer) -> Result<FrameLease, String> {
    wait(move || {
        player.poll().map_err(|e| e.to_string())?;
        Ok(player.frames().latest())
    })
    .await
}
async fn exercise(
    window: Ref<Window>,
    root: Ref<Panel>,
    view: Ref<MediaPlayerElement>,
    source: MediaSource,
    budget: FrameBudget,
) -> Result<(), String> {
    let cycles = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--macos-lifecycle-cycles=")
                .map(str::to_owned)
        })
        .ok_or("Missing cycle count")?
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    if !(12..=1000).contains(&cycles) {
        return Err("Mac lifecycle cycles must be 12..1000 (10 warm-up cycles)".into());
    }
    let negative = std::env::args().any(|a| a == "--macos-leak-control");
    let backend = if std::env::args().any(|a| a == "--opengl") { "cgl" } else { "metal" };
    let workload = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--macos-memory-workload=")
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "full".into());
    if !["source", "renderer", "reset", "resize", "full"].contains(&workload.as_str()) {
        return Err("Unknown Mac memory workload".into());
    }
    let resets = if workload == "reset" || workload == "full" {
        2
    } else {
        0
    };
    std::println!("MACOS_LIFECYCLE_PROFILE backend={backend} metal_counter_scope=metal_allocations_only cycles={cycles} warmup=10 workload={workload} footprint_growth_max={FOOTPRINT_GROWTH} metal_growth_max={METAL_GROWTH} port_growth_max={PORT_GROWTH} source=avc-aac.mp4 source_size=160x90 renderer=220x220 resets_per_cycle={resets} leak_control={negative}");
    let compositor = ElementComposition::get_element_visual(&window)
        .ok_or("No compositor")?
        .compositor()
        .clone();
    let started = Instant::now();
    view.set_autoplay(false);
    let mut baseline = None;
    let mut retained_control = Vec::new();
    let mut pending_opens = 0;
    let mut pending_seeks = 0;
    for cycle in 1..=cycles {
        let interop = compositor
            .try_get_composition_gpu_interop()
            .await
            .ok_or("No Apple GPU interop")?;
        // Poll real native preparation before clearing, retaining the future to
        // verify cancellation instead of silently dropping unstarted work.
        let provider = providers::video(&*interop, budget.clone()).map_err(|e| e.to_string())?;
        let opening = MediaPlayer::new(provider.clone());
        let mut operation = opening.set_source(source.clone());
        let pending = operation
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending();
        opening.clear_source();
        if pending {
            cancelled(operation).await?;
            pending_opens += 1;
        }
        drop(opening);
        retired(budget.clone()).await?;
        view.set_source(source.clone(), provider, providers::importer())
            .await
            .map_err(|e| e.to_string())?;
        let player = view.media_player().ok_or("No owned player")?;
        let mut options = player.options();
        options.muted = true;
        player.set_options(options).map_err(|e| e.to_string())?;
        player.play().await.map_err(|e| e.to_string())?;
        let old = frame(player.clone()).await?;
        let host = if workload == "source" || workload == "resize" {
            None
        } else {
            let host = renderer::install(&root, player.frames(), &*interop)
                .ok_or("Lifecycle requires --renderer")?;
            let observed = host.clone();
            wait(move || Ok(observed.output().and_then(|m| m.latest()).map(|_| ()))).await?;
            Some(host)
        };
        drop(interop);
        player.pause().map_err(|e| e.to_string())?;
        let mut seeks = Vec::new();
        for index in 0..6 {
            let mut seek = player.seek(Duration::from_millis(100 + index * 50));
            match seek.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
                Poll::Pending => seeks.push(seek),
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(e)) => return Err(e.to_string()),
            }
        }
        player
            .seek(Duration::from_millis(650))
            .await
            .map_err(|e| e.to_string())?;
        pending_seeks += seeks.len();
        for seek in seeks {
            cancelled(seek).await?;
        }
        drop(old);
        for reset in 0..resets {
            let before = frame(player.clone()).await?.metadata().identity.generation;
            let job = compositor.invoke_server_job_async(
                |server| {
                    server.reset_all_gpu_resources();
                    server.render_interface().value();
                    Ok(())
                },
                false,
            );
            wait(move || match job.take_result() {
                Some(r) => r.map(Some).map_err(|e| e.to_string()),
                None => Ok(None),
            })
            .await?;
            let observed = player.clone();
            wait(move || {
                observed.poll().map_err(|e| e.to_string())?;
                Ok(observed
                    .frames()
                    .latest()
                    .filter(|f| f.metadata().identity.generation > before)
                    .map(|_| ()))
            })
            .await?;
            if player.requested_state() != PlaybackIntent::Pause
                || player
                    .playback_session()
                    .position
                    .abs_diff(Duration::from_millis(650))
                    > Duration::from_millis(150)
            {
                return Err(format!("Reset {reset} lost paused intent/seek"));
            }
            let current = frame(player.clone()).await?;
            let generation = current.metadata().device_generation;
            drop(current);
            let observed = host.as_ref().unwrap().clone();
            wait(move || {
                Ok(observed
                    .output()
                    .and_then(|m| m.latest())
                    .filter(|f| f.metadata().device_generation == generation)
                    .map(|_| ()))
            })
            .await?;
        }
        player.stop().await.map_err(|e| e.to_string())?;
        // Replace while an independent old-generation frame is still leased.
        let old = frame(player.clone()).await?;
        let interop = compositor
            .try_get_composition_gpu_interop()
            .await
            .ok_or("No replacement Metal context")?;
        let provider = providers::video(&*interop, budget.clone()).map_err(|e| e.to_string())?;
        view.set_source(source.clone(), provider, providers::importer())
            .await
            .map_err(|e| e.to_string())?;
        let replacement = view.media_player().ok_or("No replacement player")?;
        let fresh = frame(replacement.clone()).await?;
        if fresh.metadata().identity.source == old.metadata().identity.source {
            return Err("Replacement reused previous source identity".into());
        }
        drop(old);
        drop(fresh);
        if let Some(host) = host.as_ref() {
            host.set_inputs(vec![]);
            root.children().remove(host);
        }
        drop(host);
        view.set_media_player(None, None);
        drop(player);
        drop(replacement);
        drop(interop);
        retired(budget.clone()).await?;
        if (workload == "full" || workload == "resize") && cycle % 2 == 0 {
            window.set_width(940.);
            window.set_height(620.);
        } else {
            window.set_width(800.);
            window.set_height(500.);
        }
        settle().await?;
        if negative && cycle > 10 {
            retained_control.push(vec![0xa5u8; 16 * MIB as usize]);
        }
        std::hint::black_box(&retained_control);
        let sample = memory()?;
        if cycle == 10 {
            baseline = Some(sample);
        }
        if let Some(base) = baseline {
            let footprint = sample.footprint.saturating_sub(base.footprint);
            let metal = sample.metal.saturating_sub(base.metal);
            let ports = sample.ports.saturating_sub(base.ports);
            if sample.device != base.device
                || footprint > FOOTPRINT_GROWTH
                || metal > METAL_GROWTH
                || ports > PORT_GROWTH
            {
                return Err(format!("Native memory growth exceeds frozen limit: footprint={footprint} metal={metal} ports={ports} sample={sample:?} baseline={base:?}"));
            }
        }
        if budget.counters().snapshot().preview != PixelTransferCount::default() {
            return Err("CPU preview transfer during lifecycle run".into());
        }
        let (rust_bytes, rust_blocks) = crate::macos_allocations::live();
        std::println!("MACOS_MEMORY cycle={cycle} footprint={} resident={} metal={} device={} ports={} malloc_bytes={} malloc_blocks={} rust_bytes={rust_bytes} rust_blocks={rust_blocks} frames=0 reads=0 quarantine=0 native_owners=0 renderer_retained=false elapsed_ms={}",sample.footprint,sample.resident,sample.metal,sample.device,sample.ports,sample.malloc_bytes,sample.malloc_blocks,started.elapsed().as_millis());
    }
    if negative {
        return Err("Forced retained-memory negative control was not detected".into());
    }
    if pending_opens == 0 || pending_seeks == 0 {
        return Err(
            "No real preparation/seek operation was pending during lifecycle probes".into(),
        );
    }
    std::println!("MACOS_LIFECYCLE_PASS cycles={cycles} resets={} pending_opens={pending_opens} superseded_seeks={pending_seeks} preview=0 retained=0 native_owners=0",cycles*resets);
    Ok(())
}
pub async fn run(
    window: Ref<Window>,
    root: Ref<Panel>,
    view: Ref<MediaPlayerElement>,
    source: MediaSource,
    budget: FrameBudget,
) {
    if let Err(error) = exercise(window.clone(), root, view, source, budget).await {
        eprintln!("MACOS_LIFECYCLE_FAILED {error}");
        FAILED.store(true, Ordering::Relaxed);
    }
    window.close();
}
