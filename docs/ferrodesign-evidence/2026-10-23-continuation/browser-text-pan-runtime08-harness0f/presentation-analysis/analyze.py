#!/usr/bin/env python3
"""Read the preserved trace and exact git objects; print the derived evidence.

No browser, compiler, network access, or application mutation is involved.
Large Chromium frame identifiers are emitted as strings to retain all digits
when another tool subsequently parses the JSON with JavaScript numbers.
"""

import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib


HERE = Path(__file__).resolve().parent
EVIDENCE = HERE.parent
REPO = next(path for path in HERE.parents if (path / ".git").exists())
COMPARISON = "c549f6bcafe614a6373c743f5efbf2a3cb0eccd2"
PROVENANCE = json.loads((EVIDENCE / "provenance.json").read_text())
RUNTIME = PROVENANCE["runtime_commit"]
HARNESS = PROVENANCE["harness_commit"]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def preserved(name):
    expected = PROVENANCE["files"][name]
    archive = (EVIDENCE / expected["archive"]).read_bytes()
    raw = gzip.decompress(archive)
    assert sha(archive) == expected["archive_sha256"]
    assert len(raw) == expected["bytes"] and sha(raw) == expected["sha256"]
    return json.loads(raw), {
        "path": "../" + expected["archive"],
        "uncompressedBytes": len(raw),
        "uncompressedSha256": sha(raw),
        "archiveSha256": sha(archive),
    }


def git_bytes(revision, path):
    return subprocess.check_output(["git", "show", revision + ":" + path], cwd=REPO)


def source(revision, path):
    raw = git_bytes(revision, path)
    blob = subprocess.check_output(
        ["git", "rev-parse", revision + ":" + path], cwd=REPO, text=True
    ).strip()
    return {"gitBlob": blob, "bytes": len(raw), "sha256": sha(raw)}


def union_us(intervals):
    result, end = 0, None
    for left, right in sorted(intervals):
        if right <= left:
            continue
        if end is None or left >= end:
            result += right - left
        elif right > end:
            result += right - end
        end = max(right, end) if end is not None else right
    return result


def pipeline(event):
    return event.get("args", {}).get("chrome_graphics_pipeline", {})


def one(items):
    assert len(items) == 1, "Expected exactly one matching trace record"
    return items[0]


trace, trace_hash = preserved("current-WebGL2-pan-trace.json")
summary, summary_hash = preserved("current-WebGL2-pan-trace-summary.json")
report, report_hash = preserved("report.json")
events = trace["traceEvents"]
analysis = summary["analysis"]
assert analysis["status"] == "complete" and summary["normalSamplesIncluded"] is False
assert not analysis["evidence"]["dataLossOccurred"]
assert not analysis["evidence"]["streamTruncated"]
main_pid, main_tid = map(int, analysis["mainThread"]["id"].split(":"))
marks = {event["name"]: event["ts"] for event in events
         if event.get("name", "").startswith("ferrodesign-pan-current-WebGL2:")}
motions = []
for observed in summary["observation"]["samples"]:
    index = observed["index"]
    phases = {phase: marks[f"ferrodesign-pan-current-WebGL2:{index}:{phase}"]
              for phase in ["armed", "capture", "bubble", "frame", "presentation", "raf1", "settled"]}
    capture, settled = phases["capture"], phases["settled"]
    window = [event for event in events if capture <= event.get("ts", -1) < settled]
    submit = one([event for event in window if event.get("ph") == "X"
                  and event.get("pid") == main_pid
                  and pipeline(event).get("step") == "STEP_SUBMIT_COMPOSITOR_FRAME"])
    surface_id = pipeline(submit)["surface_frame_trace_id"]
    aggregation = one([event for event in window if event.get("ph") == "X"
                       and pipeline(event).get("step") == "STEP_SURFACE_AGGREGATION"
                       and surface_id in pipeline(event).get("aggregated_surface_frame_trace_ids", [])])
    display_id = pipeline(aggregation)["display_trace_id"]
    post = one([event for event in window if event.get("ph") == "X"
                and pipeline(event).get("step") == "STEP_BUFFER_SWAP_POST_SUBMIT"
                and pipeline(event).get("display_trace_id") == display_id])
    swap = one([event for event in window if event.get("ph") == "X"
                and event.get("name") == "SkiaOutputSurfaceImplOnGpu::SwapBuffers"
                and (event["pid"], event["tid"]) == (post["pid"], post["tid"])
                and event["ts"] <= post["ts"]
                and event["ts"] + event["dur"] >= post["ts"] + post["dur"]])
    finished = one([event for event in window if event.get("ph") == "X"
                    and pipeline(event).get("step") == "STEP_FINISH_BUFFER_SWAP"
                    and pipeline(event).get("display_trace_id") == display_id])
    callbacks = [event for event in window if event.get("ph") == "X"
                 and (event["pid"], event["tid"]) == (main_pid, main_tid)
                 and event.get("name") == "FunctionCall"
                 and event.get("args", {}).get("data", {}).get("url", "").endswith("/ferroui.js")]
    callback = max(callbacks, key=lambda event: event["dur"])
    animation_frame = one([event for event in window if event.get("ph") == "X"
                           and event.get("name") == "FireAnimationFrame"
                           and (event["pid"], event["tid"]) == (main_pid, main_tid)
                           and event["ts"] <= callback["ts"]
                           and event["ts"] + event["dur"] >= callback["ts"] + callback["dur"]])
    throttles = [event for event in window if event.get("name") == "Swap throttled"
                 and event.get("pid") == swap["pid"]
                 and aggregation["ts"] <= event["ts"] <= swap["ts"] + swap["dur"]]
    assert len(throttles) == 2
    assert all(event["args"] == {"max_pending_swaps": 1, "pending_swaps": 1} for event in throttles)
    main_coverage = union_us([
        (max(phases["presentation"], event["ts"]), min(settled, event["ts"] + event["dur"]))
        for event in events if event.get("ph") == "X"
        and (event["pid"], event["tid"]) == (main_pid, main_tid)
    ])
    existing = analysis["samples"][index]["windows"]["presentationToSettled"]
    assert abs(main_coverage / 1000 - existing["groups"]["mainThread"]["observedWallMs"]) < 0.0011
    assert observed["after"]["renderedFrames"] - observed["before"]["renderedFrames"] == 1
    assert observed["after"]["presentedFrames"] - observed["before"]["presentedFrames"] == 1
    motions.append({
        "index": index, "traceMarkersUs": phases,
        "inputToFrameMs": round((phases["frame"] - capture) / 1000, 3),
        "inputToNativePresentationMs": round((phases["presentation"] - capture) / 1000, 3),
        "inputToSettledMs": round((settled - capture) / 1000, 3),
        "ferroUiAnimationCallback": {
            "startUs": callback["ts"], "wallUs": callback["dur"], "threadCpuUs": callback["tdur"],
            "enclosingFireAnimationFrame": {"startUs": animation_frame["ts"],
                                            "wallUs": animation_frame["dur"]},
            "source": callback["args"]["data"],
        },
        "rendererSubmission": {"timestampUs": submit["ts"], "surfaceFrameId": str(surface_id)},
        "displayAggregation": {
            "timestampUs": aggregation["ts"], "displayId": str(display_id),
            "surfaceFrameIds": [str(value) for value in pipeline(aggregation)["aggregated_surface_frame_trace_ids"]],
        },
        "browserSwap": {
            "processId": swap["pid"], "threadId": swap["tid"], "threadName": "CrGpuMain",
            "startUs": swap["ts"], "endUs": swap["ts"] + swap["dur"],
            "wallUs": swap["dur"], "threadCpuUs": swap["tdur"],
            "postSubmitStartUs": post["ts"], "postSubmitWallUs": post["dur"],
            "finishBufferSwapUs": finished["ts"],
            "throttleAssociationWindow": "matched display aggregation through matched swap end",
            "throttles": [{"timestampUs": event["ts"],
                           "relativeToSwapStartUs": event["ts"] - swap["ts"],
                           "processId": event["pid"], "threadId": event["tid"],
                           **event["args"]} for event in throttles],
        },
        "presentationToSettled": {
            "wallMs": round((settled - phases["presentation"]) / 1000, 3),
            "observedMainThreadSliceUnionMs": main_coverage / 1000,
            "observedGpuGroupSliceUnionMs": existing["groups"]["gpu"]["observedWallMs"],
        },
        "settledAfterSwapEndMs": round((settled - swap["ts"] - swap["dur"]) / 1000, 3),
    })
assert len(motions) == 6

unchanged_paths = [
    "src/Browser/FerroUI.Browser/webapp/modules/ferroui/timer.ts",
    "src/Browser/FerroUI.Browser/rendering/browser_render_timer.rs",
    "src/Browser/FerroUI.Browser/rendering/browser_web_gl_render_target.rs",
    "src/Browser/FerroUI.Browser/rendering/render_statistics.rs",
    "src/Vello/FerroUI.Vello/web_gl/mod.rs",
    "src/Vello/FerroUI.Vello/scene/vello_web_gl_scene_sink.rs",
    "src/Vello/FerroUI.Vello/platform_render_interface.rs",
    "samples/FerroDesign/crates/ferrodesign-app/src/canvas.rs",
    "samples/FerroDesign/Cargo.toml",
]
unchanged_sources = []
for path in unchanged_paths:
    previous, current = source(RUNTIME, path), source(COMPARISON, path)
    assert previous == current
    unchanged_sources.append({"path": path, "identicalAtBothRevisions": True, **previous})
old_lock = tomllib.loads(git_bytes(RUNTIME, "samples/FerroDesign/Cargo.lock").decode())
new_lock = tomllib.loads(git_bytes(COMPARISON, "samples/FerroDesign/Cargo.lock").decode())
dependencies = []
for name in ["vello_gpu", "vello_common", "vello_gpu_shaders", "glifo", "peniko", "web-sys", "js-sys"]:
    previous = [package for package in old_lock["package"] if package["name"] == name]
    current = [package for package in new_lock["package"] if package["name"] == name]
    assert previous == current and len(previous) == 1
    dependencies.append({"completeLockEntryUnchanged": True, **previous[0]})
normal = one([result for result in report["results"] if result["requestedMode"] == "WebGL2"])
result = {
    "schemaVersion": 1,
    "scope": "Read-only reanalysis of a preserved six-motion diagnostic; no new runtime execution.",
    "derivation": {"script": "analyze.py", "sha256": sha(Path(__file__).read_bytes()),
                   "requires": "Python 3.11+ and the named git objects"},
    "runtimeRevision": RUNTIME, "harnessRevision": HARNESS, "sourceComparisonRevision": COMPARISON,
    "inputs": [trace_hash, summary_hash, report_hash],
    "clock": analysis["clock"], "traceCompleteness": analysis["evidence"],
    "rendererMainThread": analysis["mainThread"], "environment": normal["environment"],
    "normal36MotionCohort": {
        "timing": normal["timing"], "presentationTiming": normal["presentationTiming"],
        "settlingTiming": normal["settlingTiming"], "preparationDelta": normal["preparationDelta"],
        "nativeFrameDelta": normal["nativeFrameDelta"],
        "maxCompactBridgeCharacters": max(sample["browserObservation"]["maxPayloadCharacters"] for sample in normal["samples"]),
        "maxCompactBridgeReadMs": max(sample["browserObservation"]["maxPollReadMs"] for sample in normal["samples"]),
        "maxFramePollCount": max(sample["browserObservation"]["pollCount"] for sample in normal["samples"]),
    },
    "diagnosticMotions": motions,
    "unchangedDirectBrowserSources": unchanged_sources,
    "unchangedDependencies": dependencies,
    "comparisonOnlySources": [{"path": path, **source(COMPARISON, path)} for path in [
        "src/Vello/FerroUI.Vello/drawing_context_impl/fragments.rs",
        "src/Vello/FerroUI.Vello/scene/vello_hybrid_scene_sink.rs",
        "src/Vello/FerroUI.Vello/gpu/metal.rs",
    ]],
    "caveats": [
        "The 36 normal samples and six traced samples are distinct cohorts; do not derive the normal p95 from the trace.",
        "Slice unions describe observed wall-clock coverage and overlap; they are not additive CPU utilisation.",
        "threadCpuUs is Chromium's tdur on the named thread, not total SwiftShader worker CPU, hardware GPU time, or time on other processes.",
        "Each matched display swap aggregates this renderer's surface and another surface; matching proves inclusion, not exclusive causation.",
        "The trace does not resolve driver/device waiting, unrecorded worker work, or shared-host scheduling inside the long swap.",
        "The native presentedFrames counter marks the platform drawing-session disposal, before Chromium's actual display swap completion.",
        "SkiaOutputSurfaceImplOnGpu names Chromium's own compositor; the application renderer remains FerroUI Vello.",
        "Matching direct browser-source blobs does not make the full current framework identical; current 0.4 runtime behavior remains unmeasured here.",
    ],
}
print(json.dumps(result, indent=2))
