// Pure analysis of a bounded Chromium JSON trace. Durations below are unions
// of observed wall-clock intervals, never CPU utilisation or summed nested work.
// Page-clock input timestamps are aligned only through our bracketed User Timing
// marks; the Node clock is deliberately not an input to this module.
export const TRACE_ANALYSIS_LIMITS = Object.freeze({ events: 120000, samples: 6, topEvents: 3 });
const finite = Number.isFinite;
const round = value => Math.round(value * 1000) / 1000;
const key = event => `${event.pid}:${event.tid}`;
const categories = event => String(event.cat || "").split(",");

export function unionMicroseconds(intervals) {
    const ordered = intervals.filter(([start, end]) => finite(start) && finite(end) && end > start)
        .sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    let total = 0, start, end;
    for (const interval of ordered) {
        if (start === undefined) [start, end] = interval;
        else if (interval[0] <= end) end = Math.max(end, interval[1]);
        else { total += end - start; [start, end] = interval; }
    }
    return total + (start === undefined ? 0 : end - start);
}

function indexTrace(input) {
    const events = input.slice(0, TRACE_ANALYSIS_LIMITS.events), names = new Map(), processes = new Map();
    for (const event of events) {
        if (event.ph === "M" && event.name === "thread_name") names.set(key(event), String(event.args?.name || ""));
        if (event.ph === "M" && event.name === "process_name") processes.set(event.pid, String(event.args?.name || ""));
    }
    const slices = [], instants = [], asyncRecords = [], stacks = new Map();
    let unmatchedEnds = 0, invalidDurations = 0;
    // CDP JSON may interleave threads. Within a thread use timestamp order,
    // retaining input order for equal timestamps (B/E nesting relies on it).
    const ordered = events.filter(event => finite(event.ts)).sort((a, b) => a.ts - b.ts);
    for (const event of ordered) {
        if (event.ph === "X") {
            if (finite(event.dur) && event.dur >= 0) slices.push({ ...event, end: event.ts + event.dur });
            else invalidDurations++;
        } else if (event.ph === "B") {
            const stack = stacks.get(key(event)) || [];
            stack.push(event); stacks.set(key(event), stack);
        } else if (event.ph === "E") {
            const start = stacks.get(key(event))?.pop();
            if (start && event.ts >= start.ts) slices.push({ ...start, dur: event.ts - start.ts, end: event.ts });
            else unmatchedEnds++;
        } else if (["I", "i", "R"].includes(event.ph)) instants.push(event);
        else if (["b", "e", "n", "S", "T", "F", "s", "t", "f"].includes(event.ph)) asyncRecords.push(event);
    }
    return { events, names, processes, slices, instants, asyncRecords,
        completeness: { inputEvents: input.length, analyzedEvents: events.length,
            eventLimitExceeded: input.length > events.length, unmatchedEnds,
            unmatchedBegins: [...stacks.values()].reduce((sum, stack) => sum + stack.length, 0), invalidDurations,
            asyncOrFlowRecords: asyncRecords.length } };
}

function groupFor(event, index, mainThread) {
    const cat = categories(event), name = String(event.name || ""), thread = index.names.get(key(event)) || "";
    const process = index.processes.get(event.pid) || "", groups = [];
    if (key(event) === mainThread) {
        groups.push("mainThread");
        if (cat.includes("toplevel") || /^(RunTask|ThreadController.*::RunTask)$/.test(name)) groups.push("mainThreadTasks");
    }
    if (cat.some(value => /^(input|latencyInfo)(\.|$)/.test(value)) ||
        /^(EventDispatch|InputLatency|InputHandler|InputRouter)/.test(name)) groups.push("input");
    if (/Compositor|Viz/.test(thread) || cat.some(value => /^(cc|viz)(\.|$)/.test(value))) groups.push("compositor");
    if (/gpu/i.test(process) || /Gpu|GPU/.test(thread) || cat.some(value => /^gpu(\.|$)/.test(value))) groups.push("gpu");
    if (/^(FireAnimationFrame|RequestAnimationFrame|CancelAnimationFrame|AnimationFrame::)/.test(name)) groups.push("animationFrame");
    return groups;
}

function summarizeWindow(index, mainThread, start, end) {
    if (!finite(start) || !finite(end) || end < start) return null;
    const groups = Object.fromEntries(["mainThread", "mainThreadTasks", "input", "compositor", "gpu", "animationFrame"]
        .map(name => [name, { intervals: [], eventNames: new Map(), slices: 0, instantEvents: 0, asyncOrFlowRecords: 0, longestSliceUs: 0 }]));
    for (const event of index.slices) {
        const left = Math.max(start, event.ts), right = Math.min(end, event.end);
        if (right <= left) continue;
        for (const name of groupFor(event, index, mainThread)) {
            const group = groups[name], intervals = group.eventNames.get(event.name) || [];
            group.intervals.push([left, right]); intervals.push([left, right]); group.eventNames.set(event.name, intervals);
            group.slices++; group.longestSliceUs = Math.max(group.longestSliceUs, right - left);
        }
    }
    for (const event of index.instants) {
        if (event.ts < start || event.ts > end) continue;
        for (const name of groupFor(event, index, mainThread)) groups[name].instantEvents++;
    }
    for (const event of index.asyncRecords) {
        if (event.ts < start || event.ts > end) continue;
        for (const name of groupFor(event, index, mainThread)) groups[name].asyncOrFlowRecords++;
    }
    return { durationMs: round((end - start) / 1000),
        groups: Object.fromEntries(Object.entries(groups).map(([name, group]) => [name, {
            observedWallMs: round(unionMicroseconds(group.intervals) / 1000), slices: group.slices,
            instantEvents: group.instantEvents, asyncOrFlowRecords: group.asyncOrFlowRecords,
            longestClippedSliceMs: round(group.longestSliceUs / 1000),
            topEvents: [...group.eventNames].map(([event, intervals]) => ({ event: String(event).slice(0, 120), observedWallMs: round(unionMicroseconds(intervals) / 1000) }))
                .sort((a, b) => b.observedWallMs - a.observedWallMs || a.event.localeCompare(b.event))
                .slice(0, TRACE_ANALYSIS_LIMITS.topEvents),
        }])) };
}

export function analyzeTextPanTrace(trace, observation, capture = {}) {
    const input = Array.isArray(trace) ? trace : trace?.traceEvents;
    if (!Array.isArray(input)) throw new Error("Chromium trace must contain a traceEvents array");
    const index = indexTrace(input), markerEvents = new Map();
    for (const event of index.events) {
        if (!String(event.name || "").startsWith(observation.prefix + ":") ||
            !categories(event).includes("blink.user_timing") || !finite(event.ts) ||
            !["I", "i", "R", "X"].includes(event.ph)) continue;
        const entries = markerEvents.get(event.name) || [];
        entries.push(event); markerEvents.set(event.name, entries);
    }
    const samples = observation.samples.slice(0, TRACE_ANALYSIS_LIMITS.samples), anchors = [];
    const missingMarkers = [], duplicateMarkers = [], wrongThreadMarkers = [];
    let mainThread = null;
    for (const sample of samples) for (const marker of sample.markers) {
        const events = markerEvents.get(marker.name) || [];
        if (!events.length) { missingMarkers.push(marker.name); continue; }
        if (events.length !== 1) { duplicateMarkers.push(marker.name); continue; }
        const event = events[0]; mainThread ??= key(event);
        if (key(event) !== mainThread) { wrongThreadMarkers.push(marker.name); continue; }
        if (!finite(marker.beforeMs) || !finite(marker.afterMs) || marker.afterMs < marker.beforeMs) continue;
        anchors.push({ name: marker.name, ts: event.ts, beforeMs: marker.beforeMs, afterMs: marker.afterMs });
    }
    // Non-isolated Chromium can quantize performance.now(). The explicit
    // 0.25 ms allowance is part of the reported alignment uncertainty, not a
    // claimed timing precision. Inconsistent anchors suppress projected windows.
    const quantizationAllowanceUs = 250;
    const offsetLowUs = anchors.length ? Math.max(...anchors.map(mark => mark.ts - mark.afterMs * 1000 - quantizationAllowanceUs)) : null;
    const offsetHighUs = anchors.length ? Math.min(...anchors.map(mark => mark.ts - mark.beforeMs * 1000 + quantizationAllowanceUs)) : null;
    const aligned = anchors.length >= 2 && offsetLowUs <= offsetHighUs && wrongThreadMarkers.length === 0;
    const offsetUs = aligned ? (offsetLowUs + offsetHighUs) / 2 : null;
    const byName = new Map(anchors.map(mark => [mark.name, mark.ts]));
    const analyzedSamples = samples.map(sample => {
        const time = phase => byName.get(sample.markers.find(mark => mark.phase === phase)?.name);
        const captureEvent = sample.input?.capture, bubbleEvent = sample.input?.bubble;
        const armed = sample.markers.find(mark => mark.phase === "armed");
        const validInputClock = finite(captureEvent?.eventTimeMs) && captureEvent.eventTimeMs >= (armed?.beforeMs ?? Infinity) - 0.25 &&
            finite(captureEvent.observedAtMs) && captureEvent.eventTimeMs <= captureEvent.observedAtMs + 0.25;
        const eventTimestamp = aligned && validInputClock ? captureEvent.eventTimeMs * 1000 + offsetUs : null;
        const boundaries = {
            armedToCapture: [time("armed"), time("capture")],
            eventTimestampToCapture: [eventTimestamp, time("capture")],
            captureToBubble: [time("capture"), time("bubble")],
            bubbleToFrame: [time("bubble"), time("frame")],
            frameToPresentation: [time("frame"), time("presentation")],
            presentationToSettled: [time("presentation"), time("settled")],
            captureToSettled: [time("capture"), time("settled")],
        };
        return { index: sample.index, validInputClock, trustedInput: captureEvent?.trusted === true && bubbleEvent?.trusted === true,
            pageClock: { eventToCaptureMs: validInputClock ? round(captureEvent.observedAtMs - captureEvent.eventTimeMs) : null,
                captureToBubbleMs: captureEvent && bubbleEvent ? round(bubbleEvent.observedAtMs - captureEvent.observedAtMs) : null },
            windows: Object.fromEntries(Object.entries(boundaries).map(([name, [start, end]]) =>
                [name, summarizeWindow(index, mainThread, start, end)])) };
    });
    const bufferMayBeFull = (capture.peakBufferFraction ?? 0) >= 0.98;
    const complete = !capture.dataLossOccurred && !capture.streamTruncated && !bufferMayBeFull && !index.completeness.eventLimitExceeded &&
        missingMarkers.length === 0 && duplicateMarkers.length === 0 && wrongThreadMarkers.length === 0 && aligned &&
        observation.samples.length === TRACE_ANALYSIS_LIMITS.samples &&
        analyzedSamples.every(sample => sample.trustedInput && sample.validInputClock && Object.values(sample.windows).every(Boolean)) &&
        !(observation.droppedInputEvents > 0);
    return { schemaVersion: 1, status: complete ? "complete" : "incomplete", clock: {
        traceUnit: "monotonic microseconds", pageUnit: "performance.now/Event.timeStamp milliseconds", hostClockUsed: false,
        method: "Intersect trace-marker offsets bracketed by page performance.now before/after performance.mark.",
        anchorCount: anchors.length, aligned, offsetUs, uncertaintyMs: aligned ? round((offsetHighUs - offsetLowUs) / 2000) : null,
        quantizationAllowanceMs: quantizationAllowanceUs / 1000,
    }, mainThread: mainThread ? { id: mainThread, name: index.names.get(mainThread) || "unavailable" } : null,
        evidence: { ...index.completeness, dataLossOccurred: Boolean(capture.dataLossOccurred),
            streamTruncated: Boolean(capture.streamTruncated), peakBufferFraction: capture.peakBufferFraction ?? null, bufferMayBeFull,
            missingMarkers, duplicateMarkers, wrongThreadMarkers, droppedInputEvents: observation.droppedInputEvents || 0 },
        limitations: [
            "Observed slice unions are wall-clock coverage, not CPU utilisation, GPU completion, a scheduling root cause, or evidence that uncovered intervals were idle.",
            "Groups overlap and include different threads; their times must not be added. Top event names can also describe nested work.",
            "Only synchronous X/B/E slices contribute durations. I/i/R instants and async/flow records are counted separately; incomplete B/E pairs are reported and excluded.",
            "Compositor and GPU groups include all recorded browser processes; membership does not establish that this page caused each slice.",
            "Native presentation marks observe the existing public counter; two later rAF callbacks mark settling, not screen or GPU completion.",
            "Trace-enabled samples include instrumentation overhead and are excluded from the ordinary benchmark statistics.",
        ], samples: analyzedSamples };
}

// Deliberately bounded console projection; raw traces and detailed observations
// belong in ordinary output files, never encoded into CI logs.
export function textPanTraceLogSummary(diagnostic) {
    return { diagnostic: "native-text-pan-trace-v1", mode: diagnostic.mode, build: diagnostic.build,
        status: diagnostic.status, normalSamplesIncluded: false,
        pageErrors: diagnostic.pageErrors?.length || 0, consoleErrors: diagnostic.consoleErrors?.length || 0,
        droppedErrors: diagnostic.droppedErrors || 0,
        completedSamples: diagnostic.observation?.samples?.filter(sample => sample.complete).length || 0,
        trace: diagnostic.trace ? { file: diagnostic.trace.file, bytes: diagnostic.trace.bytes,
            dataLossOccurred: diagnostic.trace.dataLossOccurred, streamTruncated: diagnostic.trace.streamTruncated } : null,
        analysisStatus: diagnostic.analysis?.status ?? null, clock: diagnostic.analysis?.clock ?? null,
        evidence: diagnostic.analysis?.evidence ? { ...diagnostic.analysis.evidence,
            missingMarkers: diagnostic.analysis.evidence.missingMarkers.length,
            duplicateMarkers: diagnostic.analysis.evidence.duplicateMarkers.length,
            wrongThreadMarkers: diagnostic.analysis.evidence.wrongThreadMarkers.length } : null,
        samples: diagnostic.analysis?.samples.map(sample => ({ index: sample.index, pageClock: sample.pageClock,
            phases: Object.fromEntries(["eventTimestampToCapture", "captureToBubble", "bubbleToFrame", "presentationToSettled"]
                .map(name => { const window = sample.windows[name]; return [name, window ? {
                    durationMs: window.durationMs,
                    observedWallMs: Object.fromEntries(Object.entries(window.groups).map(([group, value]) => [group, value.observedWallMs])),
                    mainThreadTopEvents: window.groups.mainThread.topEvents.slice(0, 1),
                    inputAsyncOrFlowRecords: window.groups.input.asyncOrFlowRecords,
                    animationFrameSlices: window.groups.animationFrame.slices,
                } : null]; })) })) || [],
        error: diagnostic.error?.slice(0, 500) ?? null, cleanupErrors: diagnostic.cleanupErrors?.map(error => error.slice(0, 250)) || [] };
}
