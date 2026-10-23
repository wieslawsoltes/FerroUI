// Optional, separately labelled diagnostic input. This module never contributes
// samples to the benchmark's ordinary warmup or performance distributions.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { open, readFile, rename, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { analyzeTextPanTrace, textPanTraceLogSummary } from "./text-pan-trace-analysis.mjs";

export const TEXT_PAN_TRACE_LIMITS = Object.freeze({ samples: 6, cohortMs: 60000,
    cleanupMs: 20000, teardownCommandMs: 3000, failureSnapshotMs: 1000, errorRecords: 8,
    ownedShutdownMs: 12000,
    bufferKiB: 8192, streamBytes: 24 * 1024 * 1024, chunkBytes: 64 * 1024 });
export const TEXT_PAN_TRACE_CATEGORIES = Object.freeze([
    "toplevel", "devtools.timeline", "blink.user_timing", "input", "latencyInfo", "cc", "gpu", "viz",
]);
const motions = [[4, 0], [4, 4], [0, 4], [-4, 4], [-4, 0], [-4, -4]];
const message = error => error?.stack || String(error);

// A timeout always leads to page/session teardown in the caller's finally path.
// Racing a protocol command alone would leave its work queued in Chromium.
async function bounded(work, until, label) {
    const remaining = until - performance.now();
    if (remaining <= 0) throw Object.assign(new Error(`Trace diagnostic deadline: ${label}`), { name: "TraceDiagnosticTimeout" });
    let timer;
    try {
        return await Promise.race([Promise.resolve().then(work), new Promise((_, reject) => {
            timer = setTimeout(() => reject(Object.assign(new Error(`Trace diagnostic deadline: ${label}`),
                { name: "TraceDiagnosticTimeout" })), remaining);
        })]);
    } finally { clearTimeout(timer); }
}

export { bounded as boundedTextPanOperation };

// Browser from chromium.launch() has no public process/kill handle. Optional
// tracing therefore owns a BrowserServer, whose public process()/kill() APIs
// let teardown verify process exit without private Playwright fields or PID scans.
// Reuse one shutdown promise: the outer benchmark must not repeat a hung close.
export function createTextPanBrowserOwner(server, commandMs = TEXT_PAN_TRACE_LIMITS.teardownCommandMs) {
    const child = server.process();
    assert.ok(child && Number.isSafeInteger(child.pid) && child.pid > 0, "Tracing must own its launched Chromium process");
    assert.ok(Number.isFinite(commandMs) && commandMs > 0);
    let client, shutdown, processExit, resolveExit;
    const exited = new Promise(resolve => { resolveExit = resolve; });
    const observeExit = (code, signal) => {
        processExit ??= { observed: true, code: code ?? null, signal: signal ?? null };
        resolveExit(processExit);
    };
    const hasExited = () => {
        if (!processExit && (child.exitCode != null || child.signalCode != null)) observeExit(child.exitCode, child.signalCode);
        return Boolean(processExit);
    };
    child.once("exit", observeExit);
    hasExited();
    const attachClient = browser => {
        assert.ok(!shutdown, "Cannot attach a browser client after owned shutdown starts");
        assert.ok(!client, "An owned browser has exactly one connected benchmark client");
        assert.ok(browser && typeof browser.close === "function" && typeof browser.isConnected === "function",
            "An owned browser client requires public close/isConnected APIs");
        client = browser;
    };
    const close = reason => shutdown ??= (async () => {
        const result = { reason, processId: child.pid, status: "closing",
            clientDisconnectAttempted: false, clientDisconnectCompleted: false, clientDisconnected: null,
            gracefulCloseCompleted: false,
            forcedKillAttempted: false, directKillAttempted: false, processExit: null, errors: [] };
        try {
            // The connected Browser and its BrowserServer have separate public
            // lifetimes. Dispose the WebSocket client before closing Chromium;
            // context.close() alone leaves that client transport connected.
            // https://playwright.dev/docs/api/class-browser#browser-close
            // Failure here must not strand the separately owned process.
            if (client) {
                if (client.isConnected()) {
                    result.clientDisconnectAttempted = true;
                    try {
                        await bounded(() => client.close({ reason }), performance.now() + commandMs, "disconnect owned trace browser client");
                        if (client.isConnected()) throw new Error("Owned browser client remained connected after close resolved");
                        result.clientDisconnectCompleted = true;
                    } catch (error) { result.errors.push(message(error)); }
                }
                result.clientDisconnected = !client.isConnected();
            }
            if (!hasExited()) {
                try {
                    await bounded(() => Promise.all([server.close(), exited]), performance.now() + commandMs, "close owned trace browser");
                    result.gracefulCloseCompleted = true;
                } catch (error) { result.errors.push(message(error)); }
            }
            if (!hasExited()) {
                result.forcedKillAttempted = true;
                try {
                    // kill() owns the browser process tree. Also observe the
                    // ChildProcess: completion of a close request is not proof
                    // that browser-wide recording has stopped.
                    await bounded(() => Promise.all([server.kill(), exited]), performance.now() + commandMs, "kill owned trace browser");
                } catch (error) { result.errors.push(message(error)); }
            }
            if (!hasExited()) {
                result.directKillAttempted = true;
                try {
                    // This is the exact ChildProcess supplied by our own public
                    // BrowserServer, never a discovered or user-supplied PID.
                    if (!child.kill("SIGKILL")) throw new Error("Owned browser process rejected SIGKILL");
                    await bounded(() => exited, performance.now() + commandMs, "observe owned browser process exit");
                } catch (error) { result.errors.push(message(error)); }
            }
            if (client) result.clientDisconnected = !client.isConnected();
            result.processExit = hasExited() ? processExit : { observed: false, code: null, signal: null };
            const abnormalExit = result.processExit.observed && (result.processExit.code !== 0 || result.processExit.signal !== null);
            result.status = !result.processExit.observed ? "unconfirmed"
                : result.forcedKillAttempted || result.directKillAttempted ? "forced"
                : result.errors.length || result.clientDisconnected === false || abnormalExit ? "failed" : "closed";
            return result;
        } finally { child.off("exit", observeExit); }
    })();
    return { processId: child.pid, get shutdownStarted() { return Boolean(shutdown); }, attachClient, close };
}

async function saveStream(session, stream, output, filename, until) {
    const path = resolve(output, filename), file = await open(path, "w"), digest = createHash("sha256");
    let bytes = 0, streamTruncated = false;
    try {
        while (true) {
            const chunk = await bounded(() => session.send("IO.read", { handle: stream, size: TEXT_PAN_TRACE_LIMITS.chunkBytes }), until, "trace stream read");
            const data = Buffer.from(chunk.data, chunk.base64Encoded ? "base64" : "utf8");
            const retained = data.subarray(0, Math.max(0, TEXT_PAN_TRACE_LIMITS.streamBytes - bytes));
            await file.writeFile(retained); digest.update(retained); bytes += retained.length;
            if (retained.length !== data.length || (!chunk.eof && bytes === TEXT_PAN_TRACE_LIMITS.streamBytes)) {
                streamTruncated = true; break;
            }
            if (chunk.eof) break;
        }
    } finally { await file.close(); }
    if (streamTruncated) {
        const partial = filename.replace(/\.json$/, ".partial.json");
        await rename(path, resolve(output, partial)); filename = partial;
    }
    return { file: filename, bytes, sha256: digest.digest("hex"), streamTruncated };
}

export async function runTextPanTraceDiagnostic({ browser, browserOwner, page, output, build, mode, pointer, documentSha256, compactBridge }) {
    const prefix = `ferrodesign-pan-${build}-${mode}`, stem = `${build}-${mode}-pan-trace`;
    const diagnostic = { schemaVersion: 1, build, mode, status: "running", normalSamplesIncluded: false,
        limits: TEXT_PAN_TRACE_LIMITS, categories: TEXT_PAN_TRACE_CATEGORIES, motions,
        observation: { prefix, samples: [], droppedInputEvents: 0 }, cleanupErrors: [], fatalBrowserClosure: false,
        pageErrors: [], consoleErrors: [], droppedErrors: 0 };
    if (!compactBridge) {
        diagnostic.status = "unsupported";
        diagnostic.error = "The diagnostic requires the compact frame bridge; legacy full-state baselines retain only their ordinary measurements.";
        console.log(JSON.stringify(textPanTraceLogSummary(diagnostic)));
        return diagnostic;
    }
    assert.ok(browserOwner && typeof browserOwner.close === "function", "Tracing requires a public owned-process shutdown handle");
    const until = performance.now() + TEXT_PAN_TRACE_LIMITS.cohortMs;
    let session, sessionPending = false, buttonHeld = false, traceStarted = false, traceStartPending = false, stream, timedOut = false;
    let peakBufferFraction = 0, complete, resolveComplete;
    const completion = new Promise(resolveCompleteValue => { resolveComplete = resolveCompleteValue; });
    const snapshot = () => page.evaluate(() => globalThis.__ferroDesignPanTrace?.snapshot() || null);
    const fail = error => { diagnostic.status = "failed"; diagnostic.error ??= message(error); };
    const recordError = (list, error) => {
        if (list.length < TEXT_PAN_TRACE_LIMITS.errorRecords) list.push(String(error).slice(0, 1000));
        else diagnostic.droppedErrors++;
    };
    const pageError = error => recordError(diagnostic.pageErrors, message(error));
    const consoleError = item => { if (item.type() === "error") recordError(diagnostic.consoleErrors, item.text()); };
    page.on("pageerror", pageError); page.on("console", consoleError);
    const closeBrowser = async reason => {
        diagnostic.fatalBrowserClosure = true;
        diagnostic.browserShutdown = await browserOwner.close(reason);
        if (!diagnostic.browserShutdown.processExit.observed)
            fail(new Error("Owned Chromium process exit could not be confirmed; no further benchmark mode may run"));
    };
    try {
        await bounded(() => page.mouse.move(pointer.x, pointer.y), until, "position pointer");
        buttonHeld = true;
        await bounded(() => page.mouse.down({ button: "middle" }), until, "begin diagnostic pan");
        await bounded(() => page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)))), until, "initial frame settling");
        // Browser-level tracing survives closing a stalled page during cleanup.
        sessionPending = true;
        session = await bounded(() => browser.newBrowserCDPSession(), until, "create trace session");
        sessionPending = false;
        session.on("Tracing.bufferUsage", event => { peakBufferFraction = Math.max(peakBufferFraction, event.percentFull ?? event.value ?? 0); });
        session.on("Tracing.tracingComplete", event => { complete = event; resolveComplete(event); });
        traceStartPending = true;
        await bounded(() => session.send("Tracing.start", {
            transferMode: "ReturnAsStream", streamFormat: "json", streamCompression: "none", bufferUsageReportingInterval: 500,
            traceConfig: { recordMode: "recordUntilFull", traceBufferSizeInKb: TEXT_PAN_TRACE_LIMITS.bufferKiB,
                enableSampling: false, enableSystrace: false,
                includedCategories: TEXT_PAN_TRACE_CATEGORIES },
        }), until, "start bounded trace");
        traceStarted = true; traceStartPending = false;
        await bounded(() => page.evaluate(({ prefix, samples }) => {
            if (globalThis.__ferroDesignPanTrace) throw new Error("A pan trace observer is already active");
            const observation = { prefix, clock: "page performance milliseconds", samples: [], droppedInputEvents: 0 };
            const read = () => {
                const raw = globalThis.ferroDesign.ferroDesignFrameState();
                if (raw.length > 4096) throw new Error("Trace sampling requires the bounded compact frame projection");
                return JSON.parse(raw);
            };
            const original = read();
            let current = null;
            const mark = phase => {
                const name = `${prefix}:${current.index}:${phase}`, beforeMs = performance.now();
                performance.mark(name);
                current.markers.push({ phase, name, beforeMs, afterMs: performance.now() });
            };
            const record = phase => event => {
                if (!current || event.clientX !== current.pointer.x || event.clientY !== current.pointer.y || !(event.buttons & 4)) return;
                if (current.input[phase]) { observation.droppedInputEvents++; return; }
                current.input[phase] = { trusted: event.isTrusted, eventTimeMs: event.timeStamp, observedAtMs: performance.now(),
                    x: event.clientX, y: event.clientY, buttons: event.buttons, pointerType: event.pointerType };
                mark(phase);
            };
            const capture = record("capture"), bubble = record("bubble");
            document.addEventListener("pointermove", capture, { capture: true, passive: true });
            document.addEventListener("pointermove", bubble, { passive: true });
            globalThis.__ferroDesignPanTrace = {
                original, read, mark,
                arm(index, pointer, delta) {
                    if (observation.samples.length >= samples) throw new Error("Diagnostic sample limit exceeded");
                    const before = read();
                    current = { index, pointer, delta, before, markers: [], input: {}, complete: false };
                    observation.samples.push(current); mark("armed"); return before;
                },
                get current() { return current; },
                snapshot() { return observation; },
                stop() {
                    document.removeEventListener("pointermove", capture, true);
                    document.removeEventListener("pointermove", bubble);
                    for (const sample of observation.samples) for (const marker of sample.markers) performance.clearMarks(marker.name);
                    return observation;
                },
            };
        }, { prefix, samples: TEXT_PAN_TRACE_LIMITS.samples }), until, "install bounded trace observer");
        for (let index = 0; index < motions.length; index++) {
            const [dx, dy] = motions[index];
            const before = await bounded(() => page.evaluate(({ index, pointer, dx, dy }) =>
                globalThis.__ferroDesignPanTrace.arm(index, { x: pointer.x + dx, y: pointer.y + dy }, { x: dx, y: dy }),
            { index, pointer, dx, dy }), until, "arm diagnostic motion");
            await bounded(() => page.mouse.move(pointer.x + dx, pointer.y + dy, { steps: 1 }), until, "dispatch diagnostic motion");
            const frame = await bounded(() => page.waitForFunction(({ after, dx, dy }) => {
                const probe = globalThis.__ferroDesignPanTrace, view = probe.read();
                if (view.renderedFrames <= after || Math.abs(view.pan.x - probe.original.pan.x - dx) >= 0.01 ||
                    Math.abs(view.pan.y - probe.original.pan.y - dy) >= 0.01) return false;
                probe.current.frame = view; probe.mark("frame");
                const presented = view.presentedFrames > probe.current.before.presentedFrames;
                if (presented) { probe.current.presentation = view.presentedFrames; probe.mark("presentation"); }
                return { presented };
            }, { after: before.renderedFrames, dx, dy }, { timeout: Math.max(1, Math.min(30000, until - performance.now())) }), until, "observe diagnostic native frame");
            const observed = await bounded(() => frame.jsonValue(), until, "read diagnostic frame outcome");
            await bounded(() => frame.dispose(), until, "dispose native frame observation");
            if (!observed.presented) {
                const presentation = await bounded(() => page.waitForFunction(after => {
                    const probe = globalThis.__ferroDesignPanTrace, view = probe.read();
                    if (view.presentedFrames <= after) return false;
                    probe.current.presentation = view.presentedFrames;
                    probe.mark("presentation"); return true;
                }, before.presentedFrames, { timeout: Math.max(1, Math.min(30000, until - performance.now())) }), until, "observe diagnostic native presentation");
                await bounded(() => presentation.dispose(), until, "dispose native presentation observation");
            }
            await bounded(() => page.evaluate(async () => {
                const probe = globalThis.__ferroDesignPanTrace;
                await new Promise(done => requestAnimationFrame(() => {
                    probe.mark("raf1"); requestAnimationFrame(() => { probe.mark("settled"); done(); });
                }));
                const after = probe.read(), sample = probe.current;
                if (after.documentRevision !== probe.original.documentRevision || after.visibleNodes !== 150 || after.invalidNodes !== 0)
                    throw new Error("Diagnostic pan changed the document or visible text workload");
                if (!sample.input.capture?.trusted || !sample.input.bubble?.trusted)
                    throw new Error("Diagnostic motion lacks its real trusted capture/bubble input");
                sample.after = after; sample.complete = true;
            }), until, "observe diagnostic settling");
            diagnostic.observation = await bounded(snapshot, until, "retain diagnostic observation");
        }
        const document = await bounded(() => page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson()), until, "verify unchanged diagnostic document");
        assert.equal(createHash("sha256").update(document).digest("hex"), documentSha256, "Diagnostic panning must preserve exact document bytes");
        assert.deepEqual(diagnostic.pageErrors, [], "Trace cohort must not raise page errors");
        assert.deepEqual(diagnostic.consoleErrors, [], "Trace cohort must not raise console errors");
        diagnostic.status = "completed";
    } catch (error) {
        timedOut = error?.name === "TraceDiagnosticTimeout" || error?.name === "TimeoutError";
        fail(error);
        try { diagnostic.observation = await bounded(snapshot, performance.now() + TEXT_PAN_TRACE_LIMITS.failureSnapshotMs, "snapshot failed diagnostic") || diagnostic.observation; }
        catch { /* A stalled page may not answer; earlier completed samples stay explicit. */ }
    } finally {
        const cleanupUntil = performance.now() + TEXT_PAN_TRACE_LIMITS.cleanupMs;
        try {
            if (timedOut && sessionPending) {
                // The session-creation result is unknown. Closing only the page
                // cannot cancel a browser-level protocol operation.
                await closeBrowser("Pan trace session creation timed out");
            } else if (timedOut) {
                // Cancels outstanding page.evaluate/waitForFunction/mouse calls,
                // rather than leaving a raced operation running after return.
                await bounded(() => page.close({ reason: "Pan trace diagnostic deadline" }), cleanupUntil, "close timed-out diagnostic page");
            } else if (!page.isClosed()) {
                diagnostic.observation = await bounded(() => page.evaluate(() => {
                    const probe = globalThis.__ferroDesignPanTrace;
                    if (!probe) return null;
                    delete globalThis.__ferroDesignPanTrace; return probe.stop();
                }), cleanupUntil, "remove trace observers") || diagnostic.observation;
                if (buttonHeld) await bounded(() => page.mouse.up({ button: "middle" }), cleanupUntil, "release diagnostic input");
            }
        } catch (error) {
            diagnostic.cleanupErrors.push(message(error)); fail(error);
            if (!page.isClosed()) {
                try { await bounded(() => page.close({ reason: "Pan trace observer cleanup" }), cleanupUntil, "close diagnostic page"); }
                catch (closeError) { diagnostic.cleanupErrors.push(message(closeError)); await closeBrowser("Pan trace page did not close"); }
            }
        }
        try {
            if ((traceStarted || traceStartPending) && !diagnostic.fatalBrowserClosure && browser.isConnected()) {
                await bounded(() => session.send("Tracing.end"), cleanupUntil, "end trace");
                const result = complete || await bounded(() => completion, cleanupUntil, "flush trace");
                stream = result.stream;
                if (!stream) throw new Error("Chromium returned no trace stream");
                diagnostic.trace = { ...await saveStream(session, stream, output, `${stem}.json`, cleanupUntil),
                    dataLossOccurred: result.dataLossOccurred, peakBufferFraction };
                if (!diagnostic.trace.streamTruncated) {
                    const trace = JSON.parse(await readFile(resolve(output, diagnostic.trace.file), "utf8"));
                    diagnostic.analysis = analyzeTextPanTrace(trace, diagnostic.observation, diagnostic.trace);
                    if (diagnostic.status === "completed" && diagnostic.analysis.status !== "complete") diagnostic.status = "incomplete";
                } else if (diagnostic.status === "completed") diagnostic.status = "incomplete";
            }
        } catch (error) {
            diagnostic.cleanupErrors.push(message(error)); fail(error);
            // If tracing never completed, detaching alone can leave browser-wide
            // recording active. End the owned browser before any further run.
            if (!complete && browser.isConnected()) await closeBrowser("Pan trace recording did not stop");
        } finally {
            if (stream && !diagnostic.fatalBrowserClosure && browser.isConnected()) {
                try { await bounded(() => session.send("IO.close", { handle: stream }), performance.now() + TEXT_PAN_TRACE_LIMITS.teardownCommandMs, "close trace stream"); }
                catch (error) { diagnostic.cleanupErrors.push(message(error)); fail(error); }
            }
            if (session && !diagnostic.fatalBrowserClosure) {
                try { await bounded(() => session.detach(), performance.now() + TEXT_PAN_TRACE_LIMITS.teardownCommandMs, "detach trace session"); }
                catch (error) {
                    diagnostic.cleanupErrors.push(message(error)); fail(error);
                    if (browser.isConnected()) await closeBrowser("Pan trace session did not detach");
                }
            }
        }
        page.off("pageerror", pageError); page.off("console", consoleError);
    }
    await writeFile(resolve(output, `${stem}-summary.json`), JSON.stringify(diagnostic, null, 2));
    console.log(JSON.stringify(textPanTraceLogSummary(diagnostic)));
    return diagnostic;
}
