//! The counts of the frames of a window, for measurements: how many scenes a frame renders, how
//! many commands and draws they hold, what they upload and where the engine spends its time.
//!
//! Off unless the environment variable `FERROUI_PROGPU_PERF` is set when the first frame is
//! rendered. Its value is the number of frames of a window between two lines on the standard
//! error stream (`FERROUI_PROGPU_PERF=60`; `1` and anything that is not a number mean 120). The
//! scenes of the layers and of the offscreen targets a frame renders are counted with the frame
//! of the window that follows them.
//!
//! With the counts on, every frame asks the engine for the times of its stages, which costs the
//! engine a few clock readings a frame; with them off nothing here runs but one load of a flag.
//!
//! The line also says what the page uploads cost and how much of that only moved because an
//! insertion or removal shifted the rest of a page: the `share` is that time against encode
//! plus flush, with the time the engine spent finding those bytes taken out of the encode
//! stage. It is the number the rule on chunked buffers reads (the plan of the backend,
//! section 2.3.12). Finding them makes these frames slower than frames without the counts.

use progpu_native_sys::progpu::{
    progpu_native_scene_frame_metrics, progpu_native_scene_metrics, progpu_native_upload_counters,
};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static EVERY: OnceLock<Option<u32>> = OnceLock::new();
static TOTALS: Mutex<Totals> = Mutex::new(Totals::new());

/// The sums of the scenes rendered since the last line.
struct Totals {
    frames: u32,
    scenes: u64,
    commands: u64,
    draws: u64,
    family_switches: u64,
    submissions: u64,
    stream_bytes: u64,
    geometry_bytes: u64,
    texture_bytes: u64,
    table_bytes: u64,
    staging_bytes: u64,
    update: Duration,
    render: Duration,
    preflight_ns: u64,
    resource_ns: u64,
    encode_ns: u64,
    flush_ns: u64,
    finalize_ns: u64,
    uploaded_bytes: u64,
    shifted_bytes: u64,
    upload_ns: u64,
    shifted_ns: u64,
    counting_ns: u64,
    counting_in_encode_ns: u64,
}

impl Totals {
    const fn new() -> Self {
        Self {
            frames: 0,
            scenes: 0,
            commands: 0,
            draws: 0,
            family_switches: 0,
            submissions: 0,
            stream_bytes: 0,
            geometry_bytes: 0,
            texture_bytes: 0,
            table_bytes: 0,
            staging_bytes: 0,
            update: Duration::ZERO,
            render: Duration::ZERO,
            preflight_ns: 0,
            resource_ns: 0,
            encode_ns: 0,
            flush_ns: 0,
            finalize_ns: 0,
            uploaded_bytes: 0,
            shifted_bytes: 0,
            upload_ns: 0,
            shifted_ns: 0,
            counting_ns: 0,
            counting_in_encode_ns: 0,
        }
    }
}

/// The number of frames between two lines, or `None` with the counts off.
fn every() -> Option<u32> {
    *EVERY.get_or_init(|| {
        let value = std::env::var("FERROUI_PROGPU_PERF").ok()?;
        Some(value.trim().parse::<u32>().ok().filter(|frames| *frames > 1).unwrap_or(120))
    })
}

/// Whether the frames are counted (and so ask the engine for the times of their stages).
pub(crate) fn enabled() -> bool {
    every().is_some()
}

/// Counts a scene the engine took (`stream_bytes` of stream in `update`) and rendered (in
/// `render`, with the metrics of the engine).
pub(crate) fn scene(
    stream_bytes: usize,
    update: Duration,
    _scene: &progpu_native_scene_metrics,
    render: Duration,
    frame: &progpu_native_scene_frame_metrics,
    uploads: &progpu_native_upload_counters,
) {
    if !enabled() {
        return;
    }
    let mut totals = TOTALS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    totals.scenes += 1;
    totals.commands += u64::from(frame.command_count);
    totals.draws += u64::from(frame.draw_call_count);
    totals.family_switches += u64::from(frame.family_switch_count);
    totals.submissions += frame.submission_count;
    totals.stream_bytes += stream_bytes as u64;
    totals.geometry_bytes += frame.vertex_upload_bytes + frame.index_upload_bytes;
    totals.texture_bytes += frame.texture_upload_bytes;
    totals.table_bytes += frame.uniform_upload_bytes
        + frame.brush_upload_bytes
        + frame.gradient_stop_upload_bytes
        + frame.text_style_upload_bytes
        + frame.color_glyph_upload_bytes;
    totals.staging_bytes += frame.coverage_staging_bytes;
    totals.update += update;
    totals.render += render;
    totals.preflight_ns += frame.cpu_preflight_nanoseconds;
    totals.resource_ns += frame.cpu_resource_nanoseconds;
    totals.encode_ns += frame.cpu_encode_nanoseconds;
    totals.flush_ns += frame.cpu_flush_nanoseconds;
    totals.finalize_ns += frame.cpu_finalize_nanoseconds;
    totals.uploaded_bytes += uploads.uploaded_bytes;
    totals.shifted_bytes += uploads.shifted_bytes;
    totals.upload_ns += uploads.upload_nanoseconds;
    totals.shifted_ns += uploads.shifted_nanoseconds;
    totals.counting_ns += uploads.counting_nanoseconds;
    totals.counting_in_encode_ns += uploads.counting_in_encode_nanoseconds;
}

/// The shifted uploads in percent of encode plus flush, without the time spent finding them.
fn shifted_share(totals: &Totals) -> f64 {
    let stages = (totals.encode_ns + totals.flush_ns).saturating_sub(totals.counting_in_encode_ns);
    if stages == 0 {
        return 0.0;
    }
    totals.shifted_ns as f64 * 100.0 / stages as f64
}

/// Counts a frame of a window and, at the chosen number of frames, writes the means of a frame
/// and starts the sums again.
pub(crate) fn window_frame() {
    let Some(every) = every() else { return };
    let mut totals = TOTALS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    totals.frames += 1;
    if totals.frames < every {
        return;
    }
    let frames = f64::from(totals.frames);
    let mean = |sum: u64| sum as f64 / frames;
    let ms = |nanoseconds: u64| nanoseconds as f64 / 1.0e6 / frames;
    eprintln!(
        "[ProGPU perf] frames {}: scenes {:.2}, commands {:.0}, draws {:.0}, family switches {:.0}, submissions {:.2}, \
         stream {:.0} kB, geometry {:.0} kB, tables {:.0} kB, textures {:.0} kB, staging {:.0} kB, \
         update {:.2} ms, render {:.2} ms (preflight {:.2}, resources {:.2}, encode {:.2}, flush {:.2}, \
         finalize {:.2}), \
         uploads {:.0} kB in {:.3} ms, shifted {:.0} kB in {:.3} ms, share {:.1} % of encode + flush, \
         counting {:.3} ms",
        totals.frames,
        mean(totals.scenes),
        mean(totals.commands),
        mean(totals.draws),
        mean(totals.family_switches),
        mean(totals.submissions),
        mean(totals.stream_bytes) / 1024.0,
        mean(totals.geometry_bytes) / 1024.0,
        mean(totals.table_bytes) / 1024.0,
        mean(totals.texture_bytes) / 1024.0,
        mean(totals.staging_bytes) / 1024.0,
        totals.update.as_secs_f64() * 1000.0 / frames,
        totals.render.as_secs_f64() * 1000.0 / frames,
        ms(totals.preflight_ns),
        ms(totals.resource_ns),
        ms(totals.encode_ns),
        ms(totals.flush_ns),
        ms(totals.finalize_ns),
        mean(totals.uploaded_bytes) / 1024.0,
        ms(totals.upload_ns),
        mean(totals.shifted_bytes) / 1024.0,
        ms(totals.shifted_ns),
        shifted_share(&totals),
        ms(totals.counting_ns),
    );
    *totals = Totals::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_share_of_shifted_uploads_leaves_out_the_time_spent_counting() {
        let mut totals = Totals::new();
        assert_eq!(0.0, shifted_share(&totals));
        totals.encode_ns = 900;
        totals.flush_ns = 200;
        totals.counting_in_encode_ns = 100;
        totals.shifted_ns = 100;
        assert!((shifted_share(&totals) - 10.0).abs() < 1.0e-9);
    }
}
