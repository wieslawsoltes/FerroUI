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
}
