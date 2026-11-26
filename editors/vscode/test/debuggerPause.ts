import { strict as assert } from 'node:assert';
import { setTimeout as delay } from 'node:timers/promises';
import { DebuggerPause, withInspectorDeadline } from '../src/debuggerPause';
import { trackDebuggerPause } from '../src/debuggerPauseTracker';

export async function verifyDebuggerPause(): Promise<void> {
    const pause = new DebuggerPause();
    const changes: boolean[] = [];
    const dispose = pause.onDidChange(paused => changes.push(paused));
    let current = true;
    const page = trackDebuggerPause(pause, () => current);
    const worker = trackDebuggerPause(pause, () => current);
    const stopped = { type: 'event', event: 'stopped', body: { threadId: 1 } };
    page.onDidSendMessage?.(stopped);
    page.onWillReceiveMessage?.({ type: 'request', command: 'next', seq: 11, arguments: { threadId: 1 } });
    page.onDidSendMessage?.(stopped);
    page.onDidSendMessage?.({ type: 'response', command: 'next', request_seq: 11, success: true });
    assert.equal(pause.paused, true, 'An earlier step response keeps the newer stop.');
    worker.onDidSendMessage?.(stopped);
    page.onDidSendMessage?.({ type: 'event', event: 'continued', body: { threadId: 1 } });
    assert.equal(pause.paused, true, 'A separate adapter remains stopped.');
    worker.onWillStopSession?.();
    assert.equal(pause.paused, false);
    current = false;
    page.onDidSendMessage?.(stopped);
    assert.deepEqual(changes, [true, false], 'A retired attachment cannot pause the current page.');
    dispose();

    const signal = new AbortController().signal;
    const marker = Symbol();
    const action = withInspectorDeadline(pause, signal, 25, async active => {
        pause.set(marker, true);
        await delay(75, undefined, { signal: active });
        pause.set(marker, false);
        return 'completed';
    });
    await assert.rejects(withInspectorDeadline(pause, signal, 25, async () => {
        assert.fail('A new request must wait for Continue while the target is paused.');
    }), /paused in the debugger/);
    assert.equal(await action, 'completed', 'An action that causes a breakpoint can return after Continue.');
    const cancel = new AbortController();
    const cancelled = withInspectorDeadline(pause, cancel.signal, 25, async active => {
        pause.set(marker, true);
        return delay(75, undefined, { signal: active });
    });
    cancel.abort(new Error('The accepted generation retired.'));
    await assert.rejects(cancelled, { name: 'AbortError' });
    pause.clear();

    await verifyContinueStopped();
}

async function verifyContinueStopped(): Promise<void> {
    const pause = new DebuggerPause();
    const actions: number[] = [];
    const adapterFor = (owner: DebuggerPause) => {
        const tracker = trackDebuggerPause(owner, () => true, {
            async customRequest(command, args) {
                assert.equal(command, 'continue');
                actions.push(args.threadId);
                tracker.onDidSendMessage?.({ type: 'event', event: 'continued', body: { threadId: args.threadId } });
            },
        });
        return tracker;
    };
    const stop = (threadId?: number) => ({ type: 'event', event: 'stopped', body: { threadId } });
    adapterFor(pause).onDidSendMessage?.(stop(7));
    adapterFor(pause).onDidSendMessage?.(stop(42));
    const otherApp = new DebuggerPause();
    adapterFor(otherApp).onDidSendMessage?.(stop(101));
    await pause.continueStopped();
    assert.deepEqual(actions, [7, 42]);
    assert.equal(pause.paused, false);
    assert.equal(otherApp.paused, true);
    await assert.rejects(pause.continueStopped(), /not paused/);

    let acceptThreads!: (threads: { threads: { id: number }[] }) => void;
    const threads = new Promise<{ threads: { id: number }[] }>(resolve => { acceptThreads = resolve; });
    const replaced = trackDebuggerPause(pause, () => true, {
        async customRequest(command) {
            assert.equal(command, 'threads');
            return threads;
        },
    });
    replaced.onDidSendMessage?.(stop());
    const request = pause.continueStopped();
    await assert.rejects(pause.continueStopped(), /already continuing/);
    replaced.onDidSendMessage?.(stop());
    acceptThreads({ threads: [{ id: 8 }] });
    await assert.rejects(request, /debugger stop changed/);
    assert.equal(pause.paused, true);
    assert.deepEqual(actions, [7, 42]);
}
