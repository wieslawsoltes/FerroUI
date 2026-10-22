// Feature assertions may fail independently, but a failed session boundary
// invalidates everything that follows. Never hide the first error or turn an
// incomplete feature into a pass while collecting later evidence.
export async function captureWithinDeadline(capture, milliseconds = 20000) {
    if (!Number.isFinite(milliseconds) || milliseconds <= 0) throw new Error("Capture deadline must be positive and finite");
    let timeout;
    try {
        return await Promise.race([
            Promise.resolve().then(capture),
            new Promise((_, reject) => {
                timeout = setTimeout(() => {
                    const error = new Error("Failure capture exceeded " + milliseconds + " ms; stop this renderer session");
                    error.code = "SCENARIO_CAPTURE_TIMEOUT";
                    reject(error);
                }, milliseconds);
            }),
        ]);
    } finally { clearTimeout(timeout); }
}

export async function runIsolatedScenario(entry, {
    prepare, run, captureFailure, checkCount, now = () => performance.now(), captureTimeoutMs = 20000,
}) {
    const started = now();
    let stage = "boundary";
    entry.ran = false;
    try {
        await prepare();
        stage = "scenario";
        entry.ran = true;
        await run();
        entry.status = "passed";
    } catch (error) {
        entry.status = "failed";
        entry.failureStage = stage;
        entry.error = error?.stack || String(error);
        let captureTimedOut = false;
        try { await captureWithinDeadline(() => captureFailure(entry), captureTimeoutMs); }
        catch (captureError) {
            entry.captureError = captureError?.stack || String(captureError);
            captureTimedOut = captureError?.code === "SCENARIO_CAPTURE_TIMEOUT";
            if (captureTimedOut) entry.captureTimedOut = true;
        }
        if (stage === "boundary" || captureTimedOut) throw error;
    } finally {
        entry.checks = checkCount() - entry.checksBefore;
        entry.durationMs = Math.max(0, Math.round(now() - started));
    }
    return entry.status === "passed";
}
