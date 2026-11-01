//! Explicit synthetic MF camera integration, never an OS privacy/hardware gate.
use super::*;
use ferroui_media::{MediaError, MediaErrorKind};
use ferroui_media_camera::{
    CameraErrorKind, CameraPermission, CameraPhoto, CameraStartOptions, PhotoOrigin,
};
use ferroui_media_windows::NativeSession;
use std::{task::Poll, time::Instant};

async fn wait<T: 'static>(
    mut check: impl FnMut() -> Result<Option<T>, String> + 'static,
) -> Result<T, String> {
    let started = Instant::now();
    ferroui_media_windows::poll_native(move || match check() {
        Ok(Some(value)) => Poll::Ready(Ok(value)),
        Err(message) => Poll::Ready(Err(MediaError::new(MediaErrorKind::NativeFailure, message))),
        Ok(None) if started.elapsed() > Duration::from_secs(5) => {
            Poll::Ready(Err(MediaError::new(
                MediaErrorKind::NativeFailure,
                "Synthetic camera stage timed out",
            )))
        }
        _ => Poll::Pending,
    })
    .await
    .map_err(|e| e.to_string())
}
async fn delay() -> Result<(), String> {
    let started = Instant::now();
    wait(move || Ok((started.elapsed() > Duration::from_millis(50)).then_some(()))).await
}
async fn photo(session: &CameraSession) -> Result<CameraPhoto, String> {
    let started = Instant::now();
    loop {
        match session.photo().await {
            Ok(photo) => return Ok(photo),
            Err(e)
                if e.kind == CameraErrorKind::Busy
                    && started.elapsed() < Duration::from_secs(5) =>
            {
                delay().await?
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}
fn validate_photo(photo: &CameraPhoto) -> Result<(), String> {
    if photo.origin != PhotoOrigin::PreviewSnapshot || photo.width != 320 || photo.height != 240 {
        return Err(format!("Invalid photo metadata {photo:?}"));
    }
    let image = image::load_from_memory(&photo.encoded)
        .map_err(|e| e.to_string())?
        .to_rgb8();
    if image.dimensions() != (320, 240) {
        return Err("Independent JPEG dimensions differ".into());
    }
    let expected = [[255u8, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];
    for (i, color) in expected.iter().enumerate() {
        let pixel = image.get_pixel((i % 2) as u32 * 160 + 80, (i / 2) as u32 * 120 + 60);
        if (0..3).any(|c| pixel[c].abs_diff(color[c]) > 25) {
            return Err(format!("JPEG quadrant {i}: {pixel:?}, expected {color:?}"));
        }
    }
    println!("SYNTHETIC_CAMERA_JPEG_PASS independently_decoded=320x240 quadrants=4 origin=PreviewSnapshot");
    Ok(())
}
async fn exercise(state: Rc<State>, budget: FrameBudget) -> Result<(), String> {
    if !std::env::args().any(|a| a == "--synthetic-camera") {
        return Err("--camera-smoke requires the explicit --synthetic-camera profile".into());
    }
    if NativeSession::synthetic_usage() != (0, 0, 0) {
        return Err("Discovery acquired synthetic resources".into());
    }
    let capture = state.capture.borrow().clone().ok_or("No capture")?;
    let pending = capture.start(Default::default());
    capture.cancel_pending_start();
    if pending
        .await
        .err()
        .is_none_or(|e| e.kind != CameraErrorKind::Cancelled)
    {
        return Err("Canceled start succeeded".into());
    }
    let mut pending = Some(capture.start(Default::default()));
    let cancelled = capture.clone();
    let pending = wait(move || {
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match pending.as_mut().unwrap().as_mut().poll(&mut cx) {
            Poll::Ready(Ok(session)) => {
                session.stop();
                Err("Camera committed before pending-start cancellation probe".into())
            }
            Poll::Ready(Err(e)) => Err(e.to_string()),
            Poll::Pending if NativeSession::synthetic_usage().2 > 0 => {
                cancelled.cancel_pending_start();
                Ok(pending.take())
            }
            Poll::Pending => Ok(None),
        }
    })
    .await?;
    if pending
        .await
        .err()
        .is_none_or(|e| e.kind != CameraErrorKind::Cancelled)
        || capture.permission() != CameraPermission::NotDetermined
    {
        return Err("Native pending-start cancellation failed".into());
    }
    wait(|| Ok((NativeSession::synthetic_usage() == (0, 0, 0)).then_some(()))).await?;
    println!("SYNTHETIC_CAMERA_START_CANCEL_PASS native_worker_started=true resources=0");
    let mut generation = 0;
    for cycle in 0..2 {
        let session = capture
            .start(Default::default())
            .await
            .map_err(|e| e.to_string())?;
        let settings = session.settings();
        if settings.format.width != 320
            || settings.format.height != 240
            || (settings.format.frames_per_second - 30000. / 1001.).abs() > 0.0001
        {
            return Err(format!("Wrong negotiated format {settings:?}"));
        }
        if let Some(presenter) = state.presenter.upgrade() {
            presenter.set_frame_source(
                Some(session.frames()),
                Some(super::super::providers::importer()),
            );
        }
        {
            if let (Some(root), Some(interop)) =
                (state.root.upgrade(), state.interop.borrow().clone())
            {
                *state.renderer.borrow_mut() =
                    super::super::renderer::install(&root, session.frames(), &*interop)
                        .map(|host| host.downgrade());
            }
        }
        *state.session.borrow_mut() = Some(session.clone());
        let frames = session.frames();
        let observed = budget.clone();
        let identity = wait(move || {
            Ok(frames
                .latest()
                .filter(|f| {
                    f.metadata().identity.sequence >= 3
                        && observed.counters().snapshot().gpu_imports > 0
                })
                .map(|f| f.metadata().identity))
        })
        .await?;
        if identity.generation <= generation {
            return Err("Camera restart did not advance generation".into());
        }
        generation = identity.generation;
        if std::env::args().any(|a| a == "--renderer") {
            let observed = state.clone();
            let output = wait(move || {
                Ok(observed
                    .renderer
                    .borrow()
                    .as_ref()
                    .and_then(|w| w.upgrade())
                    .and_then(|host| host.output())
                    .and_then(|frames| frames.latest())
                    .map(|frame| frame.metadata().identity))
            })
            .await?;
            println!("SYNTHETIC_CAMERA_RENDERER_PASS cycle={cycle} output={output:?}");
        }
        let frames = session.frames();
        let mut held = Vec::<FrameLease>::new();
        let held = wait(move || {
            if let Some(frame) = frames.latest() {
                if held
                    .last()
                    .is_none_or(|last| last.metadata().identity != frame.metadata().identity)
                {
                    held.push(frame);
                }
            }
            Ok((held.len() == 3).then(|| std::mem::take(&mut held)))
        })
        .await?;
        delay().await?;
        if budget.usage().frames != 3 || NativeSession::synthetic_usage().1 > 16 {
            return Err("Saturated capture exceeded storage limits".into());
        }
        let sequence = held.last().unwrap().metadata().identity.sequence;
        drop(held);
        let frames = session.frames();
        wait(move || {
            Ok(frames
                .latest()
                .filter(|f| f.metadata().identity.sequence > sequence)
                .map(|_| ()))
        })
        .await?;
        println!("SYNTHETIC_CAMERA_SATURATION_PASS retained=3 resumed=true");
        let before = budget.counters().snapshot();
        if before.preview != PixelTransferCount::default() {
            return Err("CPU preview transfer".into());
        }
        if cycle == 0 && before.gpu_copies != 0 {
            return Err("GPU copy before explicit photo work".into());
        }
        println!("SYNTHETIC_CAMERA_PREVIEW_PASS cycle={cycle} generation={generation} format={:?} preview={:?}",settings.format,before.preview);
        let completed = photo(&session).await?;
        validate_photo(&completed)?;
        // Check Stop wins even when a real native job has already produced bytes.
        let started = Instant::now();
        loop {
            let before = ferroui_media_camera_windows::synthetic_photo_jobs_started();
            let pending = session.photo();
            if ferroui_media_camera_windows::synthetic_photo_jobs_started() > before {
                state.stop();
                if pending
                    .await
                    .err()
                    .is_none_or(|e| e.kind != CameraErrorKind::Cancelled)
                {
                    return Err("Stop did not cancel pending photo delivery".into());
                }
                println!("SYNTHETIC_CAMERA_PHOTO_CANCEL_PASS native_job_started=true");
                break;
            }
            let _ = pending.await;
            if started.elapsed() > Duration::from_secs(5) {
                return Err("Could not start pending native photo".into());
            }
            delay().await?;
        }
        drop(session);
        let observed = budget.clone();
        wait(move || {
            Ok((observed.usage().frames == 0
                && observed.gpu_reads() == 0
                && observed.quarantined_reads() == 0
                && NativeSession::synthetic_usage() == (0, 0, 0)
                && !super::super::renderer::retained())
            .then_some(()))
        })
        .await?;
        validate_photo(&completed)?; // Encoded bytes outlive capture and GPU storage.
        println!(
            "SYNTHETIC_CAMERA_STOP_PASS frames=0 native_sources=0 native_textures=0 workers=0"
        );
    }
    let mut options = CameraStartOptions::default();
    options.selection.device_id = Some("ferroui:synthetic-camera/disconnect".into());
    println!("SYNTHETIC_CAMERA_DISCONNECT_BEGIN");
    let session = capture.start(options).await.map_err(|e| e.to_string())?;
    println!("SYNTHETIC_CAMERA_DISCONNECT_STARTED");
    if let Some(presenter) = state.presenter.upgrade() {
        presenter.set_frame_source(
            Some(session.frames()),
            Some(super::super::providers::importer()),
        );
    }
    let observed = session.clone();
    let presenter = state.presenter.clone();
    wait(move || match observed.poll() {
        Err(e) if e.kind == CameraErrorKind::DeviceLost => {
            println!("SYNTHETIC_CAMERA_DISCONNECT_DETECTED {e}");
            Ok(Some(()))
        }
        Err(e) => Err(format!("Wrong disconnection error: {e:?}")),
        Ok(()) => {
            if let Some(presenter) = presenter.upgrade() {
                presenter.present_latest();
            }
            Ok(None)
        }
    })
    .await?;
    if !session.is_stopped() || session.frames().latest().is_some() {
        return Err("Disconnected session retained preview".into());
    }
    state.stop();
    drop(session);
    let observed = budget.clone();
    wait(move || {
        Ok(
            (observed.usage().frames == 0 && NativeSession::synthetic_usage() == (0, 0, 0))
                .then_some(()),
        )
    })
    .await?;
    println!("SYNTHETIC_CAMERA_DISCONNECT_PASS native_MF_error=DeviceLost resources=0");
    let counts = budget.counters().snapshot();
    if counts.preview != PixelTransferCount::default() || counts.photo.attempts == 0 {
        return Err(format!("Unexpected transfer accounting {counts:?}"));
    }
    println!(
        "SYNTHETIC_CAMERA_SMOKE_PASS cycles=2 preview={:?} photo={:?}",
        counts.preview, counts.photo
    );
    Ok(())
}
pub async fn run(state: Rc<State>, window: Ref<Window>, budget: FrameBudget) {
    let weak = window.downgrade();
    let watchdog = DispatcherTimer::run_once(
        move || {
            FAILED.store(true, Ordering::Relaxed);
            eprintln!("SYNTHETIC_CAMERA_TIMEOUT");
            if let Some(window) = weak.upgrade() {
                window.close();
            }
        },
        Duration::from_secs(25),
        DispatcherPriority::NORMAL,
    );
    let result = exercise(state.clone(), budget).await;
    watchdog.dispose();
    state.stop();
    if let Err(error) = result {
        eprintln!("SYNTHETIC_CAMERA_FAILED {error}");
        FAILED.store(true, Ordering::Relaxed);
    }
    window.close();
}
