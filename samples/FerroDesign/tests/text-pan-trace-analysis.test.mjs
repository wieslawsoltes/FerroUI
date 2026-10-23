import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { test } from "node:test";
import { analyzeTextPanTrace, textPanTraceLogSummary, unionMicroseconds } from "./text-pan-trace-analysis.mjs";
import { createTextPanBrowserOwner } from "./text-pan-trace.mjs";

function fixture() {
    const offset = 1_000_000, prefix = "ferrodesign-test", trace = [
        { ph: "M", name: "thread_name", pid: 1, tid: 10, args: { name: "CrRendererMain" } },
        { ph: "M", name: "thread_name", pid: 2, tid: 20, args: { name: "CrGpuMain" } },
        { ph: "M", name: "process_name", pid: 2, args: { name: "GPU Process" } },
        { ph: "M", name: "thread_name", pid: 1, tid: 11, args: { name: "Compositor" } },
    ], observation = { prefix, samples: [], droppedInputEvents: 0 };
    const add = (at, dur, name, cat, pid = 1, tid = 10) => trace.push({ ph: "X", name, cat, pid, tid, ts: offset + at * 1000, dur: dur * 1000 });
    for (let index = 0; index < 6; index++) {
        const start = 100 + index * 20;
        const sample = { index, complete: true, input: {
            capture: { trusted: true, eventTimeMs: start + 1, observedAtMs: start + 5 },
            bubble: { trusted: true, eventTimeMs: start + 1, observedAtMs: start + 6 },
        }, markers: [] };
        for (const [phase, at] of Object.entries({ armed: 0, capture: 5, bubble: 6, frame: 8, presentation: 9, raf1: 10, settled: 12 })) {
            const name = `${prefix}:${index}:${phase}`;
            sample.markers.push({ name, phase, beforeMs: start + at - 0.01, afterMs: start + at + 0.01 });
            trace.push({ ph: "I", name, cat: "blink.user_timing", pid: 1, tid: 10, ts: offset + (start + at) * 1000 });
        }
        add(start, 5, "RunTask", "toplevel");
        add(start + 2, 2, "FunctionCall", "devtools.timeline");
        add(start + 5, 1, "EventDispatch", "devtools.timeline,input");
        add(start + 2, 2, "GPUTask", "gpu", 2, 20);
        add(start + 3, 3, "DrawFrame", "cc", 1, 11);
        // A second renderer's work must not become this page's main-thread work.
        add(start, 20, "OtherRenderer", "toplevel", 3, 10);
        trace.push({ ph: "B", name: "FireAnimationFrame", cat: "devtools.timeline", pid: 1, tid: 10, ts: offset + (start + 9) * 1000 });
        trace.push({ ph: "E", pid: 1, tid: 10, ts: offset + (start + 11) * 1000 });
        observation.samples.push(sample);
    }
    return { trace, observation };
}

test("union clips overlap without adding nested or parallel wall time", () => {
    assert.equal(unionMicroseconds([[0, 4], [1, 3], [4, 5], [8, 9], [2, 2], [NaN, 5]]), 6);
});

test("marked clock alignment identifies the page thread and clips actual pre-capture work", () => {
    const { trace, observation } = fixture(), result = analyzeTextPanTrace(trace, observation);
    assert.equal(result.status, "complete");
    assert.equal(result.clock.hostClockUsed, false);
    assert.equal(result.clock.offsetUs, 1_000_000);
    assert.equal(result.clock.uncertaintyMs, 0.26);
    assert.equal(result.mainThread.id, "1:10");
    const first = result.samples[0], pre = first.windows.eventTimestampToCapture;
    assert.equal(first.pageClock.eventToCaptureMs, 4);
    assert.equal(pre.durationMs, 4);
    assert.equal(pre.groups.mainThread.observedWallMs, 4, "Nested FunctionCall must not inflate RunTask coverage");
    assert.equal(pre.groups.mainThreadTasks.observedWallMs, 4);
    assert.equal(pre.groups.gpu.observedWallMs, 2);
    assert.equal(pre.groups.compositor.observedWallMs, 2, "Clip the compositor slice at capture");
    assert.equal(first.windows.captureToBubble.groups.input.observedWallMs, 1);
    assert.equal(first.windows.presentationToSettled.groups.animationFrame.observedWallMs, 2);
    assert.equal(first.windows.presentationToSettled.groups.animationFrame.slices, 1);
    assert.ok(!pre.groups.mainThread.topEvents.some(event => event.event === "OtherRenderer"));
});

test("inconsistent or absent anchors suppress projected input windows rather than mixing clocks", () => {
    const { trace, observation } = fixture();
    trace.find(event => event.name === `${observation.prefix}:0:capture`).ts += 5000;
    const inconsistent = analyzeTextPanTrace(trace, observation);
    assert.equal(inconsistent.clock.aligned, false);
    assert.equal(inconsistent.clock.offsetUs, null);
    assert.equal(inconsistent.samples[0].windows.eventTimestampToCapture, null);
    assert.equal(inconsistent.status, "incomplete");
    const missing = analyzeTextPanTrace([], observation);
    assert.equal(missing.clock.aligned, false);
    assert.equal(missing.evidence.missingMarkers.length, 42);
    assert.equal(missing.samples[0].windows.captureToSettled, null);
});

test("duplicate markers, input loss and CDP data loss are explicit incomplete evidence", () => {
    const { trace, observation } = fixture();
    trace.push({ ...trace.find(event => event.name === `${observation.prefix}:0:armed`) });
    observation.droppedInputEvents = 3;
    const result = analyzeTextPanTrace(trace, observation, { dataLossOccurred: true, streamTruncated: true, peakBufferFraction: 1 });
    assert.equal(result.status, "incomplete");
    assert.equal(result.evidence.duplicateMarkers.length, 1);
    assert.equal(result.evidence.droppedInputEvents, 3);
    assert.equal(result.evidence.dataLossOccurred, true);
    assert.equal(result.evidence.streamTruncated, true);
    assert.equal(result.evidence.peakBufferFraction, 1);
    assert.equal(result.evidence.bufferMayBeFull, true);
});

test("async input records are counted without pretending their spans are synchronous thread work", () => {
    const { trace, observation } = fixture();
    trace.push({ ph: "b", name: "InputLatency::MouseMove", cat: "latencyInfo", pid: 1, tid: 10, ts: 1_102_000, id: 1 });
    trace.push({ ph: "e", name: "InputLatency::MouseMove", cat: "latencyInfo", pid: 1, tid: 10, ts: 1_104_000, id: 1 });
    const result = analyzeTextPanTrace(trace, observation), input = result.samples[0].windows.eventTimestampToCapture.groups.input;
    assert.equal(input.asyncOrFlowRecords, 2);
    assert.equal(input.observedWallMs, 0);
    assert.equal(result.evidence.asyncOrFlowRecords, 2);
});

test("a timestamp from an incompatible origin is not projected onto the trace", () => {
    const { trace, observation } = fixture();
    observation.samples[0].input.capture.eventTimeMs = 0;
    const result = analyzeTextPanTrace(trace, observation);
    assert.equal(result.samples[0].validInputClock, false);
    assert.equal(result.samples[0].windows.eventTimestampToCapture, null);
    assert.equal(result.status, "incomplete");
});

test("the event-count ceiling marks bounded analysis incomplete rather than silently omitting work", () => {
    const { trace, observation } = fixture();
    while (trace.length <= 120000) trace.push({ ph: "M", name: "bounded-filler" });
    const result = analyzeTextPanTrace(trace, observation);
    assert.equal(result.evidence.inputEvents, 120001);
    assert.equal(result.evidence.analyzedEvents, 120000);
    assert.equal(result.evidence.eventLimitExceeded, true);
    assert.equal(result.status, "incomplete");
});

test("unmatched synchronous slices are reported and do not invent activity", () => {
    const { trace, observation } = fixture();
    trace.push({ ph: "B", name: "Unfinished", cat: "toplevel", pid: 1, tid: 12, ts: 1_100_000 });
    trace.push({ ph: "E", pid: 1, tid: 13, ts: 1_105_000 });
    const result = analyzeTextPanTrace(trace, observation);
    assert.equal(result.evidence.unmatchedBegins, 1);
    assert.equal(result.evidence.unmatchedEnds, 1);
    assert.ok(!result.samples[0].windows.eventTimestampToCapture.groups.mainThread.topEvents.some(event => event.event === "Unfinished"));
});

test("the console projection is bounded and never prints trace events or input payloads", () => {
    const { trace, observation } = fixture(), analysis = analyzeTextPanTrace(trace, observation);
    const summary = textPanTraceLogSummary({ status: "completed", mode: "WebGL2", build: "current", observation, analysis,
        trace: { file: "current-WebGL2-pan-trace.json", bytes: 1000, sha256: "artifact only" } });
    assert.equal(summary.normalSamplesIncluded, false);
    assert.equal(summary.completedSamples, 6);
    assert.equal(summary.samples.length, 6);
    assert.equal(summary.trace.sha256, undefined);
    assert.equal(summary.observation, undefined);
    assert.ok(JSON.stringify(summary).length < 12000);
    for (const sample of summary.samples) for (const phase of Object.values(sample.phases))
        assert.ok(phase.mainThreadTopEvents.length <= 3);
});

// Public BrowserServer/ChildProcess doubles only: no browser, socket, subprocess
// or real process signal is used by these deadline/ownership regressions.
const pending = () => new Promise(() => {});
const fakeChild = () => Object.assign(new EventEmitter(), { pid: 71001, exitCode: null, signalCode: null });
function exitChild(child, code, signal) {
    child.exitCode = code; child.signalCode = signal; child.emit("exit", code, signal);
}

test("owned shutdown confirms exit and is reused by the outer finalizer", { timeout: 1000 }, async () => {
    const child = fakeChild();
    let closes = 0, kills = 0;
    const owner = createTextPanBrowserOwner({ process: () => child,
        close: async () => { closes++; exitChild(child, 0, null); },
        kill: async () => { kills++; },
    }, 5);
    const first = owner.close("diagnostic failed"), repeated = owner.close("outer finalizer");
    assert.equal(first, repeated, "Never start another close on an already stalled transport");
    const result = await first;
    assert.equal(result.status, "closed");
    assert.deepEqual(result.processExit, { observed: true, code: 0, signal: null });
    assert.equal(result.reason, "diagnostic failed");
    assert.equal(owner.shutdownStarted, true);
    assert.equal(closes, 1); assert.equal(kills, 0);
    assert.equal(child.listenerCount("exit"), 0);
});

test("hung graceful and kill cleanup still return bounded actual process-exit evidence", { timeout: 1000 }, async () => {
    const child = fakeChild();
    let kills = 0;
    const owner = createTextPanBrowserOwner({ process: () => child, close: pending,
        kill: () => { kills++; exitChild(child, null, "SIGKILL"); return pending(); },
    }, 5);
    const result = await owner.close("stalled trace");
    assert.equal(result.status, "forced");
    assert.equal(result.forcedKillAttempted, true);
    assert.equal(result.directKillAttempted, false);
    assert.deepEqual(result.processExit, { observed: true, code: null, signal: "SIGKILL" });
    assert.equal(result.errors.length, 2, "Both unfinished cleanup promises remain explicit");
    assert.equal(kills, 1);
    assert.equal(child.listenerCount("exit"), 0);
});

test("a failed public server kill falls back only to its exact owned ChildProcess", { timeout: 1000 }, async () => {
    const child = fakeChild(), signals = [];
    child.kill = signal => { signals.push(signal); exitChild(child, null, signal); return true; };
    const owner = createTextPanBrowserOwner({ process: () => child, close: pending,
        kill: async () => { throw new Error("Server kill failed"); },
    }, 5);
    const result = await owner.close("stalled trace");
    assert.equal(result.status, "forced");
    assert.equal(result.directKillAttempted, true);
    assert.equal(result.processExit.observed, true);
    assert.deepEqual(signals, ["SIGKILL"]);
    assert.equal(child.listenerCount("exit"), 0);
});

test("a kill request without an actual exit is never reported as stopped", { timeout: 1000 }, async () => {
    const child = fakeChild();
    child.kill = () => true;
    const owner = createTextPanBrowserOwner({ process: () => child, close: pending, kill: pending }, 5);
    const result = await owner.close("stalled trace");
    assert.equal(result.status, "unconfirmed");
    assert.equal(result.forcedKillAttempted, true);
    assert.equal(result.directKillAttempted, true);
    assert.deepEqual(result.processExit, { observed: false, code: null, signal: null });
    assert.equal(result.errors.length, 3);
    assert.equal(await owner.close("outer finalizer"), result);
    assert.equal(child.listenerCount("exit"), 0);
});

test("owned shutdown disconnects the connected Browser before closing its BrowserServer", { timeout: 1000 }, async () => {
    const child = fakeChild(), operations = [];
    let connected = true;
    const owner = createTextPanBrowserOwner({ process: () => child,
        close: async () => {
            operations.push("server close");
            assert.equal(connected, false, "The connected client has a lifetime independent from the process owner");
            exitChild(child, 0, null);
        },
        kill: async () => { assert.fail("Successful client/server teardown must not request a kill"); },
    }, 10);
    owner.attachClient({ isConnected: () => connected, close: async ({ reason }) => {
        operations.push("client disconnect");
        assert.equal(reason, "benchmark finished");
        connected = false;
    } });
    const first = owner.close("benchmark finished");
    assert.equal(owner.close("outer finalizer"), first);
    const result = await first;
    assert.deepEqual(operations, ["client disconnect", "server close"]);
    assert.equal(result.status, "closed");
    assert.equal(result.clientDisconnectAttempted, true);
    assert.equal(result.clientDisconnectCompleted, true);
    assert.equal(result.clientDisconnected, true);
    assert.deepEqual(result.errors, []);
    assert.equal(child.listenerCount("exit"), 0);
});

test("a stalled client disconnect still closes the owned process and remains a failed shutdown", { timeout: 1000 }, async () => {
    const child = fakeChild();
    let connected = true, closes = 0;
    const owner = createTextPanBrowserOwner({ process: () => child,
        close: async () => { closes++; connected = false; exitChild(child, 0, null); },
        kill: async () => { assert.fail("The process already exited normally"); },
    }, 5);
    owner.attachClient({ isConnected: () => connected, close: pending });
    const result = await owner.close("stalled client");
    assert.equal(closes, 1);
    assert.equal(result.status, "failed");
    assert.equal(result.clientDisconnectAttempted, true);
    assert.equal(result.clientDisconnectCompleted, false);
    assert.equal(result.clientDisconnected, true);
    assert.equal(result.gracefulCloseCompleted, true);
    assert.equal(result.forcedKillAttempted, false);
    assert.deepEqual(result.processExit, { observed: true, code: 0, signal: null });
    assert.match(result.errors[0], /disconnect owned trace browser client/);
});

test("resolved client close cannot claim successful disposal while the client stays connected", { timeout: 1000 }, async () => {
    const child = fakeChild();
    const owner = createTextPanBrowserOwner({ process: () => child,
        close: async () => { exitChild(child, 0, null); }, kill: pending,
    }, 5);
    owner.attachClient({ isConnected: () => true, close: async () => {} });
    const result = await owner.close("misreported client");
    assert.equal(result.status, "failed");
    assert.equal(result.clientDisconnectCompleted, false);
    assert.equal(result.clientDisconnected, false);
    assert.equal(result.processExit.observed, true);
    assert.match(result.errors[0], /remained connected after close resolved/);
});

test("a disconnected client and completed server close still require actual process-exit evidence", { timeout: 1000 }, async () => {
    const child = fakeChild();
    child.kill = () => true;
    const owner = createTextPanBrowserOwner({ process: () => child,
        close: async () => {}, kill: async () => {},
    }, 5);
    owner.attachClient({ isConnected: () => false, close: async () => { assert.fail("Already disconnected"); } });
    const result = await owner.close("missing process exit");
    assert.equal(result.status, "unconfirmed");
    assert.equal(result.clientDisconnectAttempted, false);
    assert.equal(result.clientDisconnected, true);
    assert.equal(result.gracefulCloseCompleted, false);
    assert.equal(result.forcedKillAttempted, true);
    assert.equal(result.directKillAttempted, true);
    assert.equal(result.processExit.observed, false);
    assert.equal(result.errors.length, 3);
});

test("normal process exit can arrive after the graceful close promise without requiring a kill", { timeout: 1000 }, async () => {
    const child = fakeChild();
    const owner = createTextPanBrowserOwner({ process: () => child,
        close: async () => { setImmediate(() => exitChild(child, 0, null)); },
        kill: async () => { assert.fail("Wait for process exit within the same bounded close phase"); },
    }, 100);
    const result = await owner.close("delayed process event");
    assert.equal(result.status, "closed");
    assert.equal(result.gracefulCloseCompleted, true);
    assert.equal(result.forcedKillAttempted, false);
    assert.deepEqual(result.processExit, { observed: true, code: 0, signal: null });
});

test("an already crashed owned process cannot be reported as a clean shutdown", { timeout: 1000 }, async () => {
    for (const [code, signal] of [[1, null], [null, "SIGSEGV"]]) {
        const child = fakeChild();
        child.exitCode = code; child.signalCode = signal;
        const owner = createTextPanBrowserOwner({ process: () => child,
            close: async () => { assert.fail("The owned process has already exited"); }, kill: pending,
        }, 5);
        const result = await owner.close("already crashed");
        assert.equal(result.status, "failed");
        assert.equal(result.gracefulCloseCompleted, false);
        assert.equal(result.forcedKillAttempted, false);
        assert.deepEqual(result.processExit, { observed: true, code, signal });
    }
});

test("client ownership is unique and cannot change after shutdown starts", { timeout: 1000 }, async () => {
    const child = fakeChild(), client = { isConnected: () => false, close: async () => {} };
    const owner = createTextPanBrowserOwner({ process: () => child,
        close: async () => { exitChild(child, 0, null); }, kill: pending,
    }, 5);
    owner.attachClient(client);
    assert.throws(() => owner.attachClient(client), /exactly one connected benchmark client/);
    const result = owner.close("finished");
    assert.throws(() => owner.attachClient(client), /after owned shutdown starts/);
    assert.equal((await result).status, "closed");
});
