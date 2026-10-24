//! Opt-in development measurements through compositor batch completion.
use crate::{ReloadOutcome, ReloadStatus};
use ferroui_controls::TopLevel;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Records accepted-revision timing when `FERROUI_RELOAD_TIMINGS=1`.
/// Call on the UI thread immediately after applying the revision. The next
/// composition batch includes pending layout/scene updates through MediaContext's
/// normal render pass; this does not force layout or block the UI thread.
///
/// Output is one `FERROUI_RELOAD_FRAME=` JSON line on stderr. Completion means the
/// batch rendered, not physical display scan-out. Multiple revisions may share a
/// batch; consumers must retain the batch identity instead of claiming a distinct
/// visible frame for each revision. Hidden/stopped targets may never complete.
pub fn record_frame_timing(top_level: &TopLevel, outcome: &ReloadOutcome) {
    if outcome.status != ReloadStatus::Applied
        || std::env::var("FERROUI_RELOAD_TIMINGS").as_deref() != Ok("1")
    {
        return;
    }
    let Some(compositor) = top_level.renderer().compositor() else {
        eprintln!("FERROUI_RELOAD_FRAME_UNAVAILABLE: renderer has no compositor");
        return;
    };
    let start = Instant::now();
    let outcome = outcome.clone();
    let batch = compositor.request_commit_async();
    let sequence = batch.sequence_id();
    batch.rendered().on_completed(move || {
        let completed = SystemTime::now().duration_since(UNIX_EPOCH).ok();
        eprintln!(
            "FERROUI_RELOAD_FRAME={}",
            serde_json::json!({
                "revision": outcome.revision,
                "documents": outcome.documents,
                "batch": sequence,
                "preparation_us": outcome.preparation.as_micros(),
                "application_us": outcome.application.as_micros(),
                "commit_to_render_us": start.elapsed().as_micros(),
                "rendered_unix_ns": completed.map(|duration| duration.as_nanos().to_string()),
            })
        );
    });
}
