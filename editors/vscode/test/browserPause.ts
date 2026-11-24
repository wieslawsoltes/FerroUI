import { strict as assert } from 'node:assert';
import { trackBrowserPause } from '../src/browserDebugTracker';
import { BrowserPause } from '../src/browserPause';

export function verifyBrowserPause(): void {
    const pause = new BrowserPause();
    const changes: boolean[] = [];
    const dispose = pause.onDidChange(paused => changes.push(paused));
    let current = true;
    const page = trackBrowserPause(pause, () => current);
    const worker = trackBrowserPause(pause, () => current);
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
}
