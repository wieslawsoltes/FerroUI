//! Native cancellation races. A live owner window keeps GPU retirement running
//! while secondary windows actually close during native preparation/recovery.
use super::native_pixels::{settle, wait};
use super::*;
use ferroui_media::{MediaErrorKind, MediaFuture};
use ferroui_media_camera::{CameraCapture, CameraErrorKind, CameraStartOptions};
use ferroui_media_camera_controls::CameraPreview;
use ferroui_media_video::{MediaPlayer, MediaPlayerProvider};
use ferroui_media_windows::NativeSession;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

async fn pending_native<T: 'static, E: std::fmt::Display + 'static>(
    future: Pin<Box<dyn Future<Output = Result<T, E>>>>,
) -> Result<Pin<Box<dyn Future<Output = Result<T, E>>>>, String> {
    let mut future = Some(future);
    wait(move || {
        let mut cx = Context::from_waker(Waker::noop());
        match future.as_mut().unwrap().as_mut().poll(&mut cx) {
            Poll::Pending if NativeSession::synthetic_usage().2 > 0 => Ok(future.take()),
            Poll::Pending => Ok(None),
            Poll::Ready(Ok(_)) => Err("Opening completed before the cancellation probe".into()),
            Poll::Ready(Err(e)) => Err(e.to_string()),
        }
    })
    .await
}
async fn cancelled<T>(pending: MediaFuture<T>) -> Result<(), String> {
    match pending.await {
        Err(e) if e.kind == MediaErrorKind::Cancelled => Ok(()),
        _ => Err("Superseded operation did not return Cancelled".into()),
    }
}
async fn retired(budget: FrameBudget) -> Result<(), String> {
    wait(move || {
        Ok((budget.usage().frames == 0
            && budget.gpu_reads() == 0
            && budget.quarantined_reads() == 0
            && NativeSession::synthetic_usage() == (0, 0, 0)
            && NativeSession::fixture_photo_status() == (false, false)
            && !renderer::retained())
        .then_some(()))
    })
    .await
}
async fn new_frame(player: MediaPlayer) -> Result<FrameLease, String> {
    wait(move || {
        player.poll().map_err(|e| e.to_string())?;
        Ok(player.frames().latest())
    })
    .await
}
fn child(owner: &Window) -> (Ref<Window>, Ref<CameraPreview>) {
    let window = Window::new();
    window.set_title(Some("FerroUI lifecycle cancellation probe".into()));
    window.set_width(400.);
    window.set_height(300.);
    let view = CameraPreview::new();
    window.set_content(Some(Control::boxed(view.clone())));
    window.show_with_owner(owner);
    (window, view)
}
async fn exercise(
    window: Ref<Window>,
    source: MediaSource,
    provider: Rc<dyn MediaPlayerProvider>,
    budget: FrameBudget,
) -> Result<(), String> {
    ferroui_media_camera_controls::register_types();
    let cycles = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--lifecycle-stress-cycles=")
                .map(str::to_owned)
        })
        .ok_or("Missing lifecycle count")?
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    if !(1..=1000).contains(&cycles) {
        return Err("Lifecycle cycles must be 1..1000".into());
    }
    let path = std::env::args().nth(1).ok_or("Missing fixture path")?;
    let resource = MediaSource::Resource {
        name: "lifecycle.mp4".into(),
        content_type: "video/mp4".into(),
        encoded: std::fs::read(path).map_err(|e| e.to_string())?.into(),
    };
    let compositor = ElementComposition::get_element_visual(&window)
        .ok_or("No lifecycle compositor")?
        .compositor()
        .clone();
    let start = Instant::now();
    for cycle in 0..cycles {
        let interop = compositor
            .try_get_composition_gpu_interop()
            .await
            .ok_or("No lifecycle context")?;
        let provider = if cycle == 0 {
            provider.clone()
        } else {
            providers::video(&*interop, budget.clone()).map_err(|e| e.to_string())?
        };
        let player = MediaPlayer::new(provider);
        eprintln!("LIFECYCLE_STAGE cycle={} open-cancel", cycle + 1);
        // Cancel only after the real native worker has started opening.
        let opening = pending_native(player.set_source(resource.clone())).await?;
        player.clear_source();
        cancelled(opening).await?;
        retired(budget.clone()).await?;
        eprintln!("LIFECYCLE_STAGE cycle={} open-replace", cycle + 1);
        // Replace an actual in-flight open, keeping its future alive to observe
        // cancellation instead of relying on dropping an unpolled future.
        let opening = pending_native(player.set_source(source.clone())).await?;
        eprintln!("LIFECYCLE_STAGE cycle={} replace-native-pending", cycle + 1);
        let replacement = player.set_source(resource.clone());
        cancelled(opening).await?;
        replacement.await.map_err(|e| e.to_string())?;
        eprintln!("LIFECYCLE_STAGE cycle={} replace-native-ready", cycle + 1);
        let old = new_frame(player.clone()).await?;
        eprintln!("LIFECYCLE_STAGE cycle={} seek-overlap", cycle + 1);
        let mut pending = Vec::new();
        for index in 0..12 {
            let mut seek = player.seek(Duration::from_millis(100 + index * 30));
            let mut cx = Context::from_waker(Waker::noop());
            match seek.as_mut().poll(&mut cx) {
                Poll::Pending => pending.push(seek),
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(e)) => return Err(e.to_string()),
            }
        }
        if pending.is_empty() {
            return Err("No native seeks were pending during race".into());
        }
        let pending_count = pending.len();
        player
            .seek(Duration::from_millis(700))
            .await
            .map_err(|e| e.to_string())?;
        for seek in pending {
            cancelled(seek).await?;
        }
        if player
            .playback_session()
            .position
            .abs_diff(Duration::from_millis(700))
            > Duration::from_millis(150)
        {
            return Err("Newest seek did not win".into());
        }
        let mut seeking = player.seek(Duration::from_millis(250));
        let pending = matches!(
            seeking
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        );
        let replace = player.set_source(source.clone());
        if pending {
            cancelled(seeking).await?;
        }
        replace.await.map_err(|e| e.to_string())?;
        eprintln!("LIFECYCLE_STAGE cycle={} seek-replace-frame", cycle + 1);
        let fresh = new_frame(player.clone()).await?;
        if fresh.metadata().identity.generation <= old.metadata().identity.generation {
            return Err("Replaced source published stale generation".into());
        }
        drop(old);
        drop(fresh);
        player.clear_source();
        drop(player);
        retired(budget.clone()).await?;

        let capture = CameraCapture::new(
            providers::camera(&*interop, budget.clone()).map_err(|e| e.to_string())?,
            budget.clone(),
        );
        eprintln!("LIFECYCLE_STAGE cycle={} camera-open-close", cycle + 1);
        let mut options = CameraStartOptions::default();
        options.selection.device_id = Some("ferroui:synthetic-camera/recovery".into());
        let (opening_window, opening_view) = child(&window);
        settle().await?;
        let opening = pending_native(opening_view.start(
            capture.clone(),
            options.clone(),
            providers::importer(),
        ))
        .await?;
        opening_window.close();
        if opening
            .await
            .err()
            .is_none_or(|e| e.kind != CameraErrorKind::Cancelled)
        {
            return Err("Closing opening camera did not cancel".into());
        }
        drop(opening_view);
        drop(opening_window);
        retired(budget.clone()).await?;

        let (photo_window, photo_view) = child(&window);
        eprintln!("LIFECYCLE_STAGE cycle={} camera-photo-start", cycle + 1);
        settle().await?;
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let factory_budget = budget.clone();
        photo_view.set_provider_factory(Some(Rc::new(move |interop| {
            observed.set(observed.get() + 1);
            Ok((
                providers::camera(interop, factory_budget.clone()).map_err(|e| {
                    ferroui_media_camera::CameraError::new(
                        CameraErrorKind::NativeFailure,
                        e.to_string(),
                    )
                })?,
                providers::importer(),
            ))
        })));
        photo_view
            .start(capture, options, providers::importer())
            .await
            .map_err(|e| e.to_string())?;
        let observed = photo_view.clone();
        let session =
            wait(move || Ok(observed.session().filter(|s| s.frames().latest().is_some()))).await?;
        let before = ferroui_media_camera_windows::synthetic_photo_jobs_started();
        eprintln!("LIFECYCLE_STAGE cycle={} camera-photo-job", cycle + 1);
        let observed = session.clone();
        let photo = wait(move || {
            let photo = observed.photo();
            Ok(
                (ferroui_media_camera_windows::synthetic_photo_jobs_started() > before)
                    .then_some(photo),
            )
        })
        .await?;
        let close = photo_window.clone();
        eprintln!("LIFECYCLE_STAGE cycle={} photo-recovery-close", cycle + 1);
        let subscription = compositor.render_context_changed(move |state| {
            if state == ferroui_base::rendering::composition::RenderContextState::Created {
                close.close();
            }
        });
        let epoch = NativeSession::synthetic_pattern_epoch();
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
        if photo
            .await
            .err()
            .is_none_or(|e| e.kind != CameraErrorKind::Cancelled)
        {
            return Err("Closed/recovering photo delivered success".into());
        }
        subscription.dispose();
        settle().await?;
        if !session.is_stopped()
            || photo_view.session().is_some()
            || calls.get() != 0
            || NativeSession::synthetic_pattern_epoch() != epoch
        {
            return Err("Window shutdown lost to queued camera recovery".into());
        }
        drop(session);
        drop(photo_view);
        drop(photo_window);
        drop(interop);
        retired(budget.clone()).await?;
        std::println!("LIFECYCLE_STRESS_SAMPLE cycle={} open_cancel=true open_replace=true pending_seeks={} latest_seek_ms=700 seek_replace=true camera_close_opening=true photo_cancel=true close_during_recovery=true reacquisitions=0 frames=0 reads=0 quarantine=0 native=(0, 0, 0)",cycle+1,pending_count);
    }
    if budget.counters().snapshot().preview != PixelTransferCount::default() {
        return Err("Lifecycle preview used CPU pixels".into());
    }
    std::println!(
        "LIFECYCLE_STRESS_PASS cycles={cycles} elapsed_ms={} preview_cpu=0 retained=0",
        start.elapsed().as_millis()
    );
    Ok(())
}
pub async fn run(
    window: Ref<Window>,
    source: MediaSource,
    provider: Rc<dyn MediaPlayerProvider>,
    budget: FrameBudget,
) {
    let weak = window.downgrade();
    let cycles = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--lifecycle-stress-cycles=")
                .and_then(|n| n.parse::<u64>().ok())
        })
        .unwrap_or(1)
        .min(1000);
    let guard = DispatcherTimer::run_once(
        move || {
            FAILED.store(true, Ordering::Relaxed);
            eprintln!("LIFECYCLE_STRESS_TIMEOUT");
            if let Some(w) = weak.upgrade() {
                w.close();
            }
        },
        Duration::from_secs(cycles * 60 + 30),
        DispatcherPriority::NORMAL,
    );
    if let Err(e) = exercise(window.clone(), source, provider, budget).await {
        eprintln!("LIFECYCLE_STRESS_FAILED {e}");
        FAILED.store(true, Ordering::Relaxed);
    }
    guard.dispose();
    window.close();
}
