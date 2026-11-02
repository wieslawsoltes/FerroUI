//! Independent MP4 fixture orientation, crop and presented-pixel acceptance.
#[cfg(target_os = "macos")]
use super::macos_diagnostics::MetalFrameReader as NativeFrameReader;
use super::native_pixels::{composition, presented, read_texture, settle, wait};
use super::*;
use ferroui_base::{
    layout::{HorizontalAlignment, VerticalAlignment},
    media::{Color, SolidColorBrush},
    Point, Visual,
};
use ferroui_media_controls::{RendererHost, TexturePresenter};
#[cfg(windows)]
use ferroui_media_opengl::NativeGlFrameProducer as NativeFrameReader;
use image::RgbaImage;
use std::cell::Cell;
async fn capture_presented(
    window: &Ref<Window>,
    counters: &FrameTransferCounters,
) -> Result<RgbaImage, String> {
    #[cfg(windows)]
    {
        presented(window, counters)
    }
    #[cfg(target_os = "macos")]
    {
        presented(window, counters).await
    }
}
fn expected(u: f64, v: f64, orientation: u32, crop: bool) -> [u8; 3] {
    // Independent inverse of rotation followed by output-space reflection.
    let u = if orientation >= 4 { 1. - u } else { u };
    let (u, v) = match orientation % 4 {
        0 => (u, v),
        1 => (v, 1. - u),
        2 => (1. - u, 1. - v),
        _ => (1. - v, u),
    };
    let (x, y) = if crop {
        (10. + 144. * u, 6. + 80. * v)
    } else {
        (160. * u, 90. * v)
    };
    [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]]
        [(y >= 45.) as usize * 2 + (x >= 80.) as usize]
}
fn check(
    image: &RgbaImage,
    window: &Ref<Window>,
    control: &Visual,
    orientation: u32,
    crop: bool,
    projection: bool,
) -> Result<(), String> {
    let size = control.bounds().size();
    for (u, v) in [(0.20, 0.20), (0.80, 0.20), (0.20, 0.80), (0.80, 0.80)] {
        let (x, y) = if projection {
            let w = 1. + 0.35 * u + 0.15 * v;
            (
                (1. + (1.4 * u - 0.25 * v - 0.55) / w) / 2.,
                (1. - (-1.35 * v + 0.7) / w) / 2.,
            )
        } else {
            (u, v)
        };
        let p = control
            .translate_point(Point::new(size.width * x, size.height * y), window)
            .ok_or("Detached pixel view")?;
        let actual = image.get_pixel(
            (p.x * window.render_scaling()) as u32,
            (p.y * window.render_scaling()) as u32,
        );
        let rgb = expected(u, v, orientation, crop);
        let expected = rgb.map(|c| {
            if projection {
                (c as f64 * 0.85 + 32. * 0.15).round() as u8
            } else {
                c
            }
        });
        if (0..3).any(|c| actual[c].abs_diff(expected[c]) > 30) {
            return Err(format!("Video pixels ({u},{v}) {actual:?}, expected {expected:?} orientation={orientation} crop={crop} projection={projection} bounds={:?} point={p:?} image={}x{} scale={}",control.bounds(),image.width(),image.height(),window.render_scaling()));
        }
    }
    Ok(())
}
fn check_edges(
    image: &RgbaImage,
    window: &Ref<Window>,
    control: &Visual,
    orientation: u32,
    crop: bool,
    wrong_offset: bool,
) -> Result<(), String> {
    let size = control.bounds().size();
    let scale = window.render_scaling();
    let (su, sv) = if crop {
        (
            (80. - if wrong_offset { 0. } else { 10. }) / 144.,
            (45. - if wrong_offset { 0. } else { 6. }) / 80.,
        )
    } else {
        (0.5, 0.5)
    };
    let (mut eu, ev) = match orientation % 4 {
        0 => (su, sv),
        1 => (1. - sv, su),
        2 => (1. - su, 1. - sv),
        _ => (sv, 1. - su),
    };
    if orientation >= 4 {
        eu = 1. - eu;
    }
    for (horizontal, expected_at) in [(true, eu), (false, ev)] {
        let length = if horizontal {
            size.width * scale
        } else {
            size.height * scale
        };
        let start = expected(0.2, 0.2, orientation, crop);
        let finish = expected(
            if horizontal { 0.8 } else { 0.2 },
            if horizontal { 0.2 } else { 0.8 },
            orientation,
            crop,
        );
        let mut crossing = None;
        for pixel in (length * 0.35) as u32..(length * 0.65) as u32 {
            let t = pixel as f64 / length;
            let (u, v) = if horizontal { (t, 0.2) } else { (0.2, t) };
            let p = control
                .translate_point(Point::new(size.width * u, size.height * v), window)
                .ok_or("Detached crop view")?;
            let c = image.get_pixel((p.x * scale) as u32, (p.y * scale) as u32);
            let distance = |rgb: [u8; 3]| {
                (0..3)
                    .map(|i| (c[i] as i32 - rgb[i] as i32).pow(2))
                    .sum::<i32>()
            };
            if distance(finish) < distance(start) {
                crossing = Some(t);
                break;
            }
        }
        let actual = crossing.ok_or("No quadrant boundary in native crop")?;
        let source_length = if horizontal == (orientation % 2 == 0) {
            if crop {
                144.
            } else {
                160.
            }
        } else {
            if crop {
                80.
            } else {
                90.
            }
        };
        // Allow one decoded source pixel plus window-capture rounding.
        let tolerance = 1.0 / source_length + 1.0 / length;
        println!("VIDEO_EDGE_PROBE horizontal={horizontal} actual={actual} expected={expected_at} tolerance={tolerance}");
        if (actual - expected_at).abs() > tolerance {
            return Err(format!("Video crop boundary horizontal={horizontal} actual={actual} expected={expected_at} tolerance={tolerance}"));
        }
    }
    Ok(())
}
async fn exercise(
    window: Ref<Window>,
    view: Ref<MediaPlayerElement>,
    host: Option<Ref<RendererHost>>,
    budget: FrameBudget,
) -> Result<(), String> {
    let host = host.ok_or("Video pixel smoke requires --renderer")?;
    let player = view.media_player().ok_or("No player")?;
    let observed = player.clone();
    let frame = wait(move || Ok(observed.frames().latest())).await?;
    player.pause().map_err(|e| e.to_string())?;
    let orientation = std::env::args()
        .find_map(|a| a.strip_prefix("--orientation=").map(str::to_owned))
        .map(|a| a.parse::<u32>().map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or(if std::env::args().any(|a| a == "--geometry-smoke") {
            5
        } else {
            0
        });
    if orientation > 7 {
        return Err("Orientation must be 0..7".into());
    }
    let crop = std::env::args().any(|a| a == "--crop-smoke");
    println!(
        "VIDEO_PIXEL_METADATA {:?} natural={:?}",
        frame.metadata(),
        player.playback_session().natural_size
    );
    let m = frame.metadata();
    let quarter = match m.rotation {
        FrameRotation::None => 0,
        FrameRotation::Clockwise90 => 1,
        FrameRotation::Clockwise180 => 2,
        FrameRotation::Clockwise270 => 3,
    };
    let mut natural = if crop { (144, 80) } else { (160, 90) };
    if orientation % 2 == 1 {
        natural = (natural.1, natural.0);
    }
    let visible = if quarter % 2 == 1 {
        (m.visible.height, m.visible.width)
    } else {
        (m.visible.width, m.visible.height)
    };
    if m.mirrored != (orientation >= 4)
        || visible != natural
        || player.playback_session().natural_size != Some(natural)
    {
        return Err(
            "Video orientation/natural-size metadata differs from independent fixture".into(),
        );
    }
    let root = host
        .parent()
        .and_then(|p| p.downcast::<Panel>().ok())
        .ok_or("No diagnostic root")?;
    view.set_is_visible(false);
    root.set_background(Some(
        SolidColorBrush::with_color(Color::new(255, 32, 32, 32)).into(),
    ));
    let source = TexturePresenter::new();
    let receipt = Rc::new(Cell::new(None));
    let observed_receipt = receipt.clone();
    let _receipt_subscription = source.frame_presented().subscribe(move |metadata| {
        observed_receipt.set(Some(metadata.identity));
    });
    source.set_width(320.);
    source.set_height(280.);
    source.set_stretch(Stretch::Fill);
    source.set_horizontal_alignment(HorizontalAlignment::Left);
    source.set_vertical_alignment(VerticalAlignment::Top);
    source.set_frame_source(Some(player.frames()), Some(providers::importer()));
    root.children().add(&source);
    settle().await?;
    source.present_latest();
    settle().await?;
    let output_host = host.clone();
    let output = wait(move || Ok(output_host.output().and_then(|m| m.latest()))).await?;
    let interop = ElementComposition::get_element_visual(&root)
        .ok_or("No compositor")?
        .compositor()
        .try_get_composition_gpu_interop()
        .await
        .ok_or("No diagnostic interop")?;
    let producer =
        NativeFrameReader::from_interop(&*interop, budget.clone()).map_err(|e| e.to_string())?;
    let raw = read_texture(&producer, output).await?;
    for (u, v) in [(0.2, 0.2), (0.8, 0.2), (0.2, 0.8), (0.8, 0.8)] {
        let w = 1. + 0.35 * u + 0.15 * v;
        let (x, y) = (
            (1. + (1.4 * u - 0.25 * v - 0.55) / w) / 2.,
            (1. + (-1.35 * v + 0.7) / w) / 2.,
        );
        #[cfg(target_os = "macos")]
        let y = 1. - y;
        let actual = raw.get_pixel(
            (x * raw.width() as f64) as u32,
            (y * raw.height() as f64) as u32,
        );
        let expected = expected(u, v, orientation, crop).map(|c| (c as f64 * 0.85).round() as u8);
        if actual[3].abs_diff(217) > 2 || (0..3).any(|c| actual[c].abs_diff(expected[c]) > 30) {
            return Err(format!(
                "Raw projected video pixels differ {actual:?} expected {expected:?}"
            ));
        }
    }
    for (x, y) in [
        (0, 0),
        (raw.width() - 1, 0),
        (0, raw.height() - 1),
        (raw.width() - 1, raw.height() - 1),
    ] {
        if raw.get_pixel(x, y).0 != [0, 0, 0, 0] {
            return Err("Projected video has opaque exterior pixels".into());
        }
    }
    let shot = composition(
        &window,
        &budget.counters(),
        &format!("video-{orientation}-crop-{crop}"),
    )
    .await?;
    check(
        &shot,
        &window,
        &source.clone().upcast::<Visual>(),
        orientation,
        crop,
        false,
    )?;
    check_edges(
        &shot,
        &window,
        &source.clone().upcast::<Visual>(),
        orientation,
        crop,
        false,
    )?;
    check(
        &shot,
        &window,
        &host.clone().upcast::<Visual>(),
        orientation,
        crop,
        true,
    )?;
    let shown = capture_presented(&window, &budget.counters()).await?;
    check(
        &shown,
        &window,
        &host.clone().upcast::<Visual>(),
        orientation,
        crop,
        true,
    )?;
    check_edges(
        &shown,
        &window,
        &source.clone().upcast::<Visual>(),
        orientation,
        crop,
        false,
    )?;
    if crop
        && check_edges(
            &shot,
            &window,
            &source.clone().upcast::<Visual>(),
            orientation,
            crop,
            true,
        )
        .is_ok()
    {
        return Err("Wrong crop offset negative control passed".into());
    }
    check(
        &shown,
        &window,
        &source.clone().upcast::<Visual>(),
        orientation,
        crop,
        false,
    )?;
    if orientation != 0
        && check(
            &shot,
            &window,
            &source.clone().upcast::<Visual>(),
            0,
            crop,
            false,
        )
        .is_ok()
    {
        return Err("Wrong orientation negative control passed".into());
    }
    window.set_width(940.);
    window.set_height(620.);
    settle().await?;
    let resized = composition(
        &window,
        &budget.counters(),
        &format!("video-{orientation}-crop-{crop}-resized"),
    )
    .await?;
    check(
        &resized,
        &window,
        &source.clone().upcast::<Visual>(),
        orientation,
        crop,
        false,
    )?;
    check(
        &resized,
        &window,
        &host.clone().upcast::<Visual>(),
        orientation,
        crop,
        true,
    )?;
    check_edges(
        &resized,
        &window,
        &source.clone().upcast::<Visual>(),
        orientation,
        crop,
        false,
    )?;
    let shown = capture_presented(&window, &budget.counters()).await?;
    check(
        &shown,
        &window,
        &source.clone().upcast::<Visual>(),
        orientation,
        crop,
        false,
    )?;
    check(
        &shown,
        &window,
        &host.clone().upcast::<Visual>(),
        orientation,
        crop,
        true,
    )?;
    if std::env::args().any(|a| a == "--video-lifecycle-pixels") {
        let old_sequence = player
            .frames()
            .latest()
            .ok_or("No pre-seek frame")?
            .metadata()
            .identity
            .sequence;
        player
            .seek(Duration::from_millis(500))
            .await
            .map_err(|e| e.to_string())?;
        source.present_latest();
        let observed = player.clone();
        wait(move || {
            observed.poll().map_err(|e| e.to_string())?;
            Ok(observed
                .frames()
                .latest()
                .filter(|f| {
                    f.metadata().identity.sequence > old_sequence
                        && f.metadata().timestamp.abs_diff(Duration::from_millis(500))
                            < Duration::from_millis(150)
                })
                .map(|_| ()))
        })
        .await?;
        source.present_latest();
        settle().await?;
        let compositor = ElementComposition::get_element_visual(&root)
            .ok_or("No lifecycle compositor")?
            .compositor()
            .clone();
        for phase in ["seek", "recovered"] {
            if phase == "recovered" {
                let before = player
                    .frames()
                    .latest()
                    .ok_or("No reset source")?
                    .metadata()
                    .identity
                    .generation;
                let weak = source.downgrade();
                let mailbox = player.frames();
                let subscription = compositor.render_context_changed(move |state| {
                    if let Some(source) = weak.upgrade() {
                        match state {
                            ferroui_base::rendering::composition::RenderContextState::Lost => {
                                source.invalidate_device()
                            }
                            ferroui_base::rendering::composition::RenderContextState::Created => {
                                source.set_frame_source(
                                    Some(mailbox.clone()),
                                    Some(providers::importer()),
                                )
                            }
                        }
                    }
                });
                let job = compositor.invoke_server_job_async(
                    |server| {
                        server.reset_all_gpu_resources();
                        server.render_interface().value();
                        Ok(())
                    },
                    false,
                );
                wait(move || match job.take_result() {
                    Some(v) => v.map(Some).map_err(|e| e.to_string()),
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
                subscription.dispose();
            }
            let source_frame = player.frames().latest().ok_or("No lifecycle frame")?;
            if source_frame.metadata().rotation != frame.metadata().rotation
                || source_frame.metadata().mirrored != frame.metadata().mirrored
                || source_frame.metadata().visible != frame.metadata().visible
            {
                return Err("Video lifecycle changed admitted geometry".into());
            }
            if player.requested_state() != ferroui_media_video::PlaybackIntent::Pause
                || player
                    .playback_session()
                    .position
                    .abs_diff(Duration::from_millis(500))
                    > Duration::from_millis(150)
            {
                return Err("Geometry recovery lost paused intent/position".into());
            }
            let device = source_frame.metadata().device.clone();
            let observed = host.clone();
            wait(move || {
                Ok(observed
                    .output()
                    .and_then(|m| m.latest())
                    .filter(|f| f.metadata().device == device)
                    .map(|_| ()))
            })
            .await?;
            source.present_latest();
            // Decoding a fresh frame does not mean its asynchronous surface
            // update has completed. Observe acceptance before taking pixels.
            let identity = source_frame.metadata().identity;
            let received = receipt.clone();
            let observed = source.clone();
            let accepted = wait(move || {
                if let Some(error) = observed.last_error() {
                    return Err(error.to_string());
                }
                observed.present_latest();
                Ok(received.get().filter(|accepted| {
                    accepted.source == identity.source
                        && accepted.generation == identity.generation
                        && accepted.sequence >= identity.sequence
                }))
            })
            .await?;
            println!("VIDEO_PRESENTATION_READY phase={phase} identity={accepted:?}");
            settle().await?;
            let shot = composition(
                &window,
                &budget.counters(),
                &format!("video-{orientation}-crop-{crop}-{phase}"),
            )
            .await?;
            check(
                &shot,
                &window,
                &source.clone().upcast::<Visual>(),
                orientation,
                crop,
                false,
            )?;
            check(
                &shot,
                &window,
                &host.clone().upcast::<Visual>(),
                orientation,
                crop,
                true,
            )?;
            check_edges(
                &shot,
                &window,
                &source.clone().upcast::<Visual>(),
                orientation,
                crop,
                false,
            )?;
            let shown = capture_presented(&window, &budget.counters()).await?;
            check(
                &shown,
                &window,
                &source.clone().upcast::<Visual>(),
                orientation,
                crop,
                false,
            )?;
            check(
                &shown,
                &window,
                &host.clone().upcast::<Visual>(),
                orientation,
                crop,
                true,
            )?;
            println!("VIDEO_LIFECYCLE_PIXELS_PASS phase={phase} orientation={orientation} crop={crop} paused=true position_ms=500");
        }
    }
    drop(frame);
    player.clear_source();
    source.set_frame_source(None, None);
    host.set_inputs(vec![]);
    root.children().remove(&host);
    let observed = budget.clone();
    wait(move || {
        #[cfg(feature = "synthetic-camera")]
        let native_retired = ferroui_media_windows::NativeSession::synthetic_usage() == (0, 0, 0);
        #[cfg(not(feature = "synthetic-camera"))]
        let native_retired = true;
        Ok((observed.usage().frames == 0
            && observed.gpu_reads() == 0
            && observed.quarantined_reads() == 0
            && !renderer::retained()
            && native_retired)
            .then_some(()))
    })
    .await?;
    let counts = budget.counters().snapshot();
    if counts.preview != PixelTransferCount::default() || counts.gpu_copies != 0 {
        return Err(format!("Unexpected video pixel transfer route {counts:?}"));
    }
    println!(
        "VIDEO_PIXEL_SMOKE_PASS orientation={orientation} crop={crop} raw=true composed=true presented=true resized=true boundary_tolerance_source_pixels=1 preview={:?} retained=0",
        budget.counters().snapshot().preview
    );
    Ok(())
}
pub async fn run(
    window: Ref<Window>,
    view: Ref<MediaPlayerElement>,
    host: Option<Ref<RendererHost>>,
    budget: FrameBudget,
) {
    let weak = window.downgrade();
    let guard = DispatcherTimer::run_once(
        move || {
            FAILED.store(true, Ordering::Relaxed);
            eprintln!("VIDEO_PIXEL_TIMEOUT");
            if let Some(w) = weak.upgrade() {
                w.close();
            }
        },
        Duration::from_secs(30),
        DispatcherPriority::NORMAL,
    );
    if let Err(e) = exercise(window.clone(), view, host, budget).await {
        eprintln!("VIDEO_PIXEL_FAILED {e}");
        FAILED.store(true, Ordering::Relaxed);
    }
    guard.dispose();
    window.close();
}
