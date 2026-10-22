import assert from "node:assert/strict";
import test from "node:test";
import { runIsolatedScenario } from "./isolated-scenario.mjs";

test("successful features preserve boundary ordering and actual check counts", async () => {
    const events = [], entry = { name: "first", status: "running", checksBefore: 3 };
    let count = 3, clock = 10;
    const passed = await runIsolatedScenario(entry, {
        prepare: async () => { events.push("durable reload"); clock += 2; },
        run: async () => { events.push("native feature"); count += 2; clock += 5; },
        captureFailure: async () => { throw new Error("No failure should be captured"); },
        checkCount: () => count, now: () => clock,
    });
    assert.equal(passed, true);
    assert.deepEqual(events, ["durable reload", "native feature"]);
    assert.deepEqual(entry, { name: "first", status: "passed", checksBefore: 3,
        ran: true, checks: 2, durationMs: 7 });
});

test("an assertion failure is retained while a later independent feature runs", async () => {
    const entries = [], events = [];
    let checks = 0;
    for (const name of ["corners", "padding"]) {
        const entry = { name, status: "running", checksBefore: checks };
        entries.push(entry);
        const passed = await runIsolatedScenario(entry, {
            prepare: async () => { events.push(name + ": verified reload"); },
            run: async () => {
                events.push(name + ": native input"); checks++;
                if (name === "corners") throw new Error("silhouette mismatch");
                checks++;
            },
            captureFailure: async current => { events.push(current.name + ": captured"); },
            checkCount: () => checks,
        });
        assert.equal(passed, name === "padding");
    }
    assert.deepEqual(events, ["corners: verified reload", "corners: native input", "corners: captured",
        "padding: verified reload", "padding: native input"]);
    assert.equal(entries[0].status, "failed");
    assert.equal(entries[0].failureStage, "scenario");
    assert.equal(entries[0].checks, 1, "Only assertions completed before failure count");
    assert.match(entries[0].error, /silhouette mismatch/);
    assert.equal(entries[1].status, "passed");
    assert.equal(entries[1].checks, 2);
    assert.equal(entries.some(entry => entry.status === "failed"), true,
        "A later pass cannot erase an earlier failure from the overall result");
});

test("failed draft/storage/panic/reload boundaries abort before running the feature", async () => {
    for (const reason of ["uncommitted draft", "storage failure", "native panic", "reload mismatch"]) {
        const error = new Error(reason), entry = { status: "running", checksBefore: 4 };
        let ran = false, captured = false;
        await assert.rejects(runIsolatedScenario(entry, {
            prepare: async () => { throw error; },
            run: async () => { ran = true; },
            captureFailure: async () => { captured = true; },
            checkCount: () => 4,
        }), actual => actual === error);
        assert.equal(ran, false); assert.equal(captured, true);
        assert.equal(entry.ran, false); assert.equal(entry.checks, 0);
        assert.equal(entry.status, "failed"); assert.equal(entry.failureStage, "boundary");
        assert.match(entry.error, new RegExp(reason));
    }
});

test("a failed capture never overwrites the primary feature or boundary error", async () => {
    for (const failureStage of ["scenario", "boundary"]) {
        const primary = new Error("primary " + failureStage), entry = { status: "running", checksBefore: 0 };
        const result = runIsolatedScenario(entry, {
            prepare: async () => { if (failureStage === "boundary") throw primary; },
            run: async () => { throw primary; },
            captureFailure: async () => { throw new Error("capture unavailable"); },
            checkCount: () => 0,
        });
        if (failureStage === "boundary") await assert.rejects(result, actual => actual === primary);
        else assert.equal(await result, false);
        assert.match(entry.error, /primary/); assert.match(entry.captureError, /capture unavailable/);
        assert.equal(entry.status, "failed"); assert.equal(entry.failureStage, failureStage);
        assert.equal(entry.checks, 0);
    }
});

test("an unresponsive capture has a deadline and aborts with the original failure", async () => {
    const primary = new Error("original pixel failure"), entry = { status: "running", checksBefore: 0 };
    let recordedBeforeCapture = false;
    await assert.rejects(runIsolatedScenario(entry, {
        prepare: async () => {},
        run: async () => { throw primary; },
        captureFailure: async current => {
            recordedBeforeCapture = current.status === "failed" && current.error.includes(primary.message);
            return new Promise(() => {});
        },
        checkCount: () => 0,
        captureTimeoutMs: 10,
    }), actual => actual === primary);
    assert.equal(recordedBeforeCapture, true);
    assert.equal(entry.status, "failed"); assert.equal(entry.ran, true);
    assert.equal(entry.captureTimedOut, true);
    assert.match(entry.error, /original pixel failure/);
    assert.match(entry.captureError, /Failure capture exceeded/);
});
