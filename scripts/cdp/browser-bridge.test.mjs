import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { connectBridge } from './browser-bridge.mjs';

test('bridge preserves payloads, delivers events and detaches on close', async () => {
    const requests = [];
    const events = [];
    const context = { window: { ferroui: { devtools: {
        handle(request) {
            requests.push(request);
            return [{ id: request.id, result: request.method === 'Target.attachToTarget' ? { sessionId: 'test-session' } : { echoed: request.params } },
                { method: 'DOM.documentUpdated', params: {} }];
        },
        poll() { return []; }
    } } } };
    const chrome = { async send(method, params) {
        assert.equal(method, 'Runtime.evaluate');
        assert.equal(params.returnByValue, true);
        return { result: { value: vm.runInNewContext(params.expression, context) } };
    } };
    const bridge = connectBridge(chrome, { onEvent: e => events.push(e), pollInterval: 10000 });
    const { sessionId } = await bridge.send('Target.attachToTarget', { targetId: '1', flatten: true });
    const payload = { text: 'quotes " \\ and ); throw new Error("must remain data")' };
    const result = await bridge.send('Input.insertText', payload, sessionId);
    assert.equal(result.echoed.text, payload.text);
    assert.equal(requests[1].sessionId, sessionId);
    assert.equal(events.length, 2);
    await bridge.close();
    assert.equal(requests.at(-1).method, 'Target.detachFromTarget');
    assert.equal(requests.at(-1).params.sessionId, sessionId);
    await assert.rejects(bridge.send('DOM.getDocument'), /closed/);
});

test('bridge surfaces protocol errors and missing bridge failures', async () => {
    const chrome = { async send() { return { result: { value: [{ id: 1, error: { code: -32601, message: 'unsupported' } }] } }; } };
    const bridge = connectBridge(chrome, { pollInterval: 10000 });
    await assert.rejects(bridge.send('Unknown.method'), e => e.code === -32601 && e.message === 'unsupported');
    await bridge.close();
    const missing = connectBridge({ async send() { return { exceptionDetails: { text: 'bridge missing' } }; } }, { pollInterval: 10000 });
    await assert.rejects(missing.send('DOM.getDocument'), /bridge missing/);
    await missing.close();
});
