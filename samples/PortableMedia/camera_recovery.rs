//! Owned CameraPreview recovery and independent source/projected/composed pixels.
use super::*;
use crate::desktop::{providers, renderer};
use ferroui_base::{
    layout::{HorizontalAlignment, VerticalAlignment},
    media::{Color, SolidColorBrush, Stretch},
    rendering::composition::{Compositor, ElementComposition, RenderContextState},
    Thickness, Visual,
};
use ferroui_media_camera_controls::CameraPreview;
use ferroui_media_controls::TextureFrameImporter;
use ferroui_media_opengl::{NativeGlFrameProducer, NativeGlTextureImporter};
#[path = "camera_pixels.rs"]
mod pixels;

async fn reset(compositor: Rc<Compositor>) -> Result<(), String> {
    let job = compositor.invoke_server_job_async(
        |server| {
            server.reset_all_gpu_resources();
            server.render_interface().value();
            Ok(())
        },
        false,
    );
    wait(move || match job.take_result() {
        Some(result) => result.map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    })
    .await
}
async fn frame(view: Ref<CameraPreview>, after: u64) -> Result<CameraSession, String> {
    wait(move || {
        Ok(view.session().filter(|s| {
            s.frames()
                .latest()
                .is_some_and(|f| f.metadata().identity.generation > after)
        }))
    })
    .await
}
pub async fn exercise(
    state: Rc<State>,
    window: Ref<Window>,
    budget: FrameBudget,
) -> Result<(), String> {
    ferroui_media_camera_controls::register_types();
    let root = state.root.upgrade().ok_or("No camera root")?;
    let old_background = root.background();
    root.set_background(Some(
        SolidColorBrush::with_color(Color::new(255, 32, 48, 64)).into(),
    ));
    if let Some(presenter) = state.presenter.upgrade() {
        presenter.set_is_visible(false);
    }
    let view = CameraPreview::new();
    view.set_width(320.);
    view.set_height(240.);
    view.set_stretch(Stretch::Fill);
    view.set_horizontal_alignment(HorizontalAlignment::Left);
    view.set_vertical_alignment(VerticalAlignment::Top);
    view.set_margin(Thickness::new(16., 16., 0., 0.));
    let borrowed = CameraPreview::new();
    borrowed.set_width(320.);
    borrowed.set_height(240.);
    borrowed.set_stretch(Stretch::Fill);
    borrowed.set_horizontal_alignment(HorizontalAlignment::Right);
    borrowed.set_vertical_alignment(VerticalAlignment::Top);
    borrowed.set_margin(Thickness::new(0., 16., 16., 0.));
    root.children().add(&view);
    root.children().add(&borrowed);
    let visual = ElementComposition::get_element_visual(&root).ok_or("No root visual")?;
    let compositor = visual.compositor().clone();
    let factory_budget = budget.clone();
    let factory_calls = Rc::new(Cell::new(0));
    let calls = factory_calls.clone();
    let fail_factory = Rc::new(Cell::new(false));
    let fail = fail_factory.clone();
    view.set_provider_factory(Some(Rc::new(move |interop| {
        calls.set(calls.get() + 1);
        if fail.replace(false) {
            return Err(ferroui_media_camera::CameraError::new(
                CameraErrorKind::Unsupported,
                "Injected recovery factory failure",
            ));
        }
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
    let errors = Rc::new(RefCell::new(Vec::new()));
    let observed_errors = errors.clone();
    let _errors = view
        .errors()
        .subscribe(move |e| observed_errors.borrow_mut().push(e.clone()));
    let borrowed_calls = Rc::new(Cell::new(0));
    let calls = borrowed_calls.clone();
    borrowed.set_provider_factory(Some(Rc::new(move |_| {
        calls.set(calls.get() + 1);
        Err(ferroui_media_camera::CameraError::new(
            CameraErrorKind::NativeFailure,
            "Borrowed control must not restart its caller's session",
        ))
    })));
    let capture = state
        .capture
        .borrow()
        .clone()
        .ok_or("No capture for owned preview")?;
    let mut options = CameraStartOptions::default();
    options.selection.device_id = Some("ferroui:synthetic-camera/recovery".into());
    view.start(capture.clone(), options.clone(), providers::importer())
        .await
        .map_err(|e| e.to_string())?;
    let mut session = frame(view.clone(), 0).await?;
    borrowed.set_session(Some(&session), Some(providers::importer()));
    let interop = compositor
        .try_get_composition_gpu_interop()
        .await
        .ok_or("No active interop")?;
    let host = renderer::install(&root, session.frames(), &*interop)
        .ok_or("Recovery pixel test requires --renderer")?;
    let mut previous_pattern = 0;
    for phase in 0..3 {
        if phase > 0 {
            let old = session.frames().latest().ok_or("No pre-reset source")?;
            let old_generation = old.metadata().identity.generation;
            let old_device = old.metadata().device.clone();
            let old_context = old.metadata().device_generation;
            let pending_photo = if phase == 1 {
                let started = Instant::now();
                loop {
                    let before = ferroui_media_camera_windows::synthetic_photo_jobs_started();
                    let pending = session.photo();
                    if ferroui_media_camera_windows::synthetic_photo_jobs_started() > before {
                        break Some(pending);
                    }
                    let _ = pending.await;
                    if started.elapsed() > Duration::from_secs(5) {
                        return Err("No photo job before reset".into());
                    }
                    delay().await?;
                }
            } else {
                None
            };
            if phase == 2 {
                fail_factory.set(true);
            }
            reset(compositor.clone()).await?;
            if let Some(pending) = pending_photo {
                if pending
                    .await
                    .err()
                    .is_none_or(|e| e.kind != CameraErrorKind::Cancelled)
                {
                    return Err("GPU reset did not cancel pending photo delivery".into());
                }
                println!("CAMERA_RECOVERY_PHOTO_CANCEL_PASS native_job_started=true");
            }
            if phase == 2 {
                let observed = errors.clone();
                wait(move || {
                    Ok(observed
                        .borrow()
                        .iter()
                        .any(|e| e.kind == CameraErrorKind::Unsupported)
                        .then_some(()))
                })
                .await?;
                if view.session().is_some() {
                    return Err("Factory failure retained session".into());
                }
                let shot =
                    pixels::composition(&window, &budget.counters(), "factory-failure").await?;
                pixels::check_cleared(&shot, &window, &view.clone().upcast::<Visual>())?;
                let shown = pixels::presented(&window, &budget.counters())?;
                pixels::check_cleared(&shown, &window, &view.clone().upcast::<Visual>())?;
                println!("CAMERA_RECOVERY_FACTORY_FAILURE_PASS stale_preview=cleared");
                reset(compositor.clone()).await?;
            }
            session = frame(view.clone(), old_generation).await?;
            let fresh = session.frames().latest().unwrap();
            if fresh.metadata().device == old_device
                && fresh.metadata().device_generation == old_context
            {
                return Err("Camera recovery reused the old device context".into());
            }
            let interop = compositor
                .try_get_composition_gpu_interop()
                .await
                .ok_or("No restored interop")?;
            if NativeGlTextureImporter.import(&*interop, &old).is_ok() {
                return Err("Old camera frame admitted by restored context".into());
            }
            if budget.usage().frames < 2 {
                return Err("Held old-generation frame was not charged".into());
            }
            borrowed.set_session(Some(&session), Some(providers::importer()));
            host.set_inputs(vec![session.frames()]);
            drop(fresh);
            drop(old);
            if budget.quarantined_reads() != 0 {
                return Err("Healthy context recovery quarantined storage".into());
            }
            println!(
                "CAMERA_RECOVERY_PASS phase={phase} generation={} stale_frame=rejected",
                session
                    .frames()
                    .latest()
                    .unwrap()
                    .metadata()
                    .identity
                    .generation
            );
        }
        let pattern = NativeSession::synthetic_pattern_epoch();
        if pattern <= previous_pattern {
            return Err("Recovery fixture did not change its pixels".into());
        }
        previous_pattern = pattern;
        let observed = host.clone();
        let current = session.frames().latest().unwrap().metadata().device.clone();
        let output = wait(move || {
            Ok(observed
                .output()
                .and_then(|f| f.latest())
                .filter(|f| f.metadata().device == current))
        })
        .await?;
        let interop = compositor
            .try_get_composition_gpu_interop()
            .await
            .ok_or("No pixel-probe context")?;
        let producer = NativeGlFrameProducer::from_interop(&*interop, budget.clone())
            .map_err(|e| e.to_string())?;
        pixels::projection(&producer, output, pattern).await?;
        delay().await?;
        let shot =
            pixels::composition(&window, &budget.counters(), &format!("recovery-{phase}")).await?;
        pixels::check_view(&shot, &window, &view.clone().upcast::<Visual>(), pattern)?;
        pixels::check_view(
            &shot,
            &window,
            &borrowed.clone().upcast::<Visual>(),
            pattern,
        )?;
        pixels::check_composed_projection(
            &shot,
            &window,
            &host.clone().upcast::<Visual>(),
            pattern,
        )?;
        let presented = pixels::presented(&window, &budget.counters())?;
        pixels::check_view(
            &presented,
            &window,
            &view.clone().upcast::<Visual>(),
            pattern,
        )?;
        pixels::check_view(
            &presented,
            &window,
            &borrowed.clone().upcast::<Visual>(),
            pattern,
        )?;
        pixels::check_composed_projection(
            &presented,
            &window,
            &host.clone().upcast::<Visual>(),
            pattern,
        )?;
        println!("CAMERA_PRESENTED_PIXELS_PASS phase={phase} window_only=true source=true projected=true");
        if pixels::check_view(
            &shot,
            &window,
            &view.clone().upcast::<Visual>(),
            pattern + 1,
        )
        .is_ok()
        {
            return Err("Composition wrong-palette negative control passed".into());
        }
        let snapshot = photo(&session).await?;
        budget
            .counters()
            .cpu_transfer(
                PixelTransferPurpose::Diagnostic,
                snapshot.encoded.len() as u64 + 320 * 240 * 3,
                PresentationRoute::SharedTexture,
            )
            .map_err(|e| e.to_string())?;
        let decoded = image::load_from_memory(&snapshot.encoded)
            .map_err(|e| e.to_string())?
            .to_rgb8();
        for (u, v) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
            let actual = decoded.get_pixel((u * 320.) as u32, (v * 240.) as u32);
            let expected = pixels::color(pattern, u, v);
            if (0..3).any(|c| actual[c].abs_diff(expected[c]) > 25) {
                return Err("Recovered JPEG differs from active pattern".into());
            }
        }
        println!("CAMERA_COMPOSITION_PIXELS_PASS phase={phase} pattern={pattern} owned=true borrowed=true projected=true jpeg=true negative_control=rejected");
        if phase == 0 {
            window.set_width(940.);
            window.set_height(620.);
            delay().await?;
            view.set_width(300.);
            view.set_height(180.);
            borrowed.set_width(300.);
            borrowed.set_height(180.);
            for stretch in [Stretch::Fill, Stretch::Uniform, Stretch::UniformToFill] {
                view.set_stretch(stretch);
                borrowed.set_stretch(stretch);
                delay().await?;
                let shot = pixels::composition(
                    &window,
                    &budget.counters(),
                    &format!("geometry-{stretch:?}"),
                )
                .await?;
                pixels::check_stretch(
                    &shot,
                    &window,
                    &view.clone().upcast::<Visual>(),
                    pattern,
                    stretch,
                )?;
                let wrong = if stretch == Stretch::Fill {
                    Stretch::Uniform
                } else {
                    Stretch::Fill
                };
                if pixels::check_stretch(
                    &shot,
                    &window,
                    &view.clone().upcast::<Visual>(),
                    pattern,
                    wrong,
                )
                .is_ok()
                {
                    return Err("Wrong stretch negative control passed".into());
                }
                pixels::check_stretch(
                    &shot,
                    &window,
                    &borrowed.clone().upcast::<Visual>(),
                    pattern,
                    stretch,
                )?;
                let presented = pixels::presented(&window, &budget.counters())?;
                pixels::check_stretch(
                    &presented,
                    &window,
                    &view.clone().upcast::<Visual>(),
                    pattern,
                    stretch,
                )?;
                pixels::check_stretch(
                    &presented,
                    &window,
                    &borrowed.clone().upcast::<Visual>(),
                    pattern,
                    stretch,
                )?;
                println!(
                    "CAMERA_GEOMETRY_PIXELS_PASS stretch={stretch:?} composed=true presented=true"
                );
            }
            for control in [&view, &borrowed] {
                control.set_width(320.);
                control.set_height(240.);
                control.set_stretch(Stretch::Fill);
            }
            delay().await?;
        }
    }
    if borrowed_calls.get() != 0 || factory_calls.get() != 3 {
        return Err(format!(
            "Unexpected recovery factories owned={} borrowed={}",
            factory_calls.get(),
            borrowed_calls.get()
        ));
    }
    // Detaching a borrowed CameraPreview must leave its caller's session alive.
    root.children().remove(&borrowed);
    let seq = session
        .frames()
        .latest()
        .unwrap()
        .metadata()
        .identity
        .sequence;
    let observed = session.clone();
    wait(move || {
        Ok(observed
            .frames()
            .latest()
            .filter(|f| f.metadata().identity.sequence > seq)
            .map(|_| ()))
    })
    .await?;
    if session.is_stopped() {
        return Err("Borrowed detach stopped owned session".into());
    }
    println!("CAMERA_BORROWED_DETACH_PASS caller_active=true");
    host.set_inputs(vec![]);
    root.children().remove(&host);
    drop(session);
    // Created queues recovery; a later handler's explicit Stop/Detach must win
    // before its asynchronous factory can acquire another native camera.
    for detach in [false, true] {
        if detach {
            let interop = compositor
                .try_get_composition_gpu_interop()
                .await
                .ok_or("No lifecycle context")?;
            capture.rebind_provider(
                providers::camera(&*interop, budget.clone()).map_err(|e| e.to_string())?,
            );
            view.start(capture.clone(), options.clone(), providers::importer())
                .await
                .map_err(|e| e.to_string())?;
            let _ = frame(view.clone(), 0).await?;
        }
        let before = factory_calls.get();
        let observed = view.clone();
        let parent = root.clone();
        let cancelled = Rc::new(Cell::new(false));
        let signal = cancelled.clone();
        let subscription = compositor.render_context_changed(move |state| {
            if state == RenderContextState::Created {
                if detach {
                    parent.children().remove(&observed);
                } else {
                    observed.stop();
                }
                signal.set(true);
            }
        });
        reset(compositor.clone()).await?;
        wait(move || Ok(cancelled.get().then_some(()))).await?;
        subscription.dispose();
        delay().await?;
        if view.session().is_some() || factory_calls.get() != before {
            return Err("Stop/detach lost to queued camera recovery".into());
        }
        println!("CAMERA_RECOVERY_CANCEL_PASS detach={detach} reacquisitions=0");
    }
    root.children().add(&view);
    delay().await?;
    if view.session().is_some() {
        return Err("Reattachment silently started camera".into());
    }
    // Exercise native MF loss before any compositor notification. The polling
    // path has already stopped the session when it preserves the restart request.
    let interop = compositor
        .try_get_composition_gpu_interop()
        .await
        .ok_or("No native-loss context")?;
    capture
        .rebind_provider(providers::camera(&*interop, budget.clone()).map_err(|e| e.to_string())?);
    options.selection.device_id = Some("ferroui:synthetic-camera/disconnect".into());
    let before_errors = errors.borrow().len();
    view.start(capture, options, providers::importer())
        .await
        .map_err(|e| e.to_string())?;
    let _ = frame(view.clone(), 0).await?;
    let observed = view.clone();
    let observed_errors = errors.clone();
    wait(move || {
        Ok((observed.session().is_none()
            && observed_errors
                .borrow()
                .iter()
                .skip(before_errors)
                .any(|e| e.kind == CameraErrorKind::DeviceLost))
        .then_some(()))
    })
    .await?;
    let shot = pixels::composition(&window, &budget.counters(), "native-loss").await?;
    pixels::check_cleared(&shot, &window, &view.clone().upcast::<Visual>())?;
    reset(compositor.clone()).await?;
    let repaired = frame(view.clone(), 0).await?;
    let jpeg = photo(&repaired).await?;
    validate_photo(&jpeg)?;
    if factory_calls.get() != 4 {
        return Err("Native loss did not restart through the provider factory".into());
    }
    view.stop();
    drop(repaired);
    println!("CAMERA_NATIVE_LOSS_RECOVERY_PASS typed_error=DeviceLost restarted=true jpeg=true");
    root.children().remove(&view);
    let observed = budget.clone();
    wait_for(Duration::from_secs(10), move || {
        if observed.quarantined_reads() != 0 {
            return Err("Recovery quarantined GPU reads".into());
        }
        Ok((observed.usage().frames == 0
            && observed.gpu_reads() == 0
            && observed.quarantined_reads() == 0
            && NativeSession::synthetic_usage() == (0, 0, 0)
            && !renderer::retained())
        .then_some(()))
    })
    .await?;
    if errors.borrow().iter().any(|e| {
        e.kind != CameraErrorKind::DeviceLost
            && !(e.kind == CameraErrorKind::Unsupported
                && e.to_string().contains("Injected recovery factory failure"))
    }) {
        return Err(format!("Camera recovery errors: {:?}", errors.borrow()));
    }
    if budget.counters().snapshot().preview != PixelTransferCount::default() {
        return Err("Recovery used CPU preview transfers".into());
    }
    root.set_background(old_background);
    if let Some(presenter) = state.presenter.upgrade() {
        presenter.set_is_visible(true);
    }
    println!(
        "CAMERA_RECOVERY_PIXELS_SMOKE_PASS resets=6 automatic_restarts=3 failed_restarts=1 cancelled_restarts=2 preview_cpu=0 retained=0 diagnostic={:?}",
        budget.counters().snapshot().diagnostic
    );
    Ok(())
}
