/** Reach a FerroUI browser bridge through a CDP session attached to the real Chrome page.
 * Unlike a native endpoint, the browser platform cannot listen on a socket. Commands below
 * execute only the transport shim in Chrome's JavaScript runtime, not application scripts.
 */
export function connectBridge(chromeSession, { onEvent = () => {}, onError = () => {}, pollInterval = 250 } = {}) {
    let next = 1;
    let closed = false;
    let queue = Promise.resolve();
    const sessions = new Set();
    let closing;
    function evaluate(expression) {
        const operation = queue.then(async () => {
            if (closed) throw new Error('Bridge connection closed');
            const { result, exceptionDetails } = await chromeSession.send('Runtime.evaluate', { expression, returnByValue: true });
            if (exceptionDetails) throw new Error(exceptionDetails.text || 'Browser bridge failed');
            if (!Array.isArray(result.value)) throw new Error('FerroUI bridge is not installed or returned invalid data');
            for (const message of result.value) if (message.method) onEvent(message);
            return result.value;
        });
        queue = operation.catch(() => {});
        return operation;
    }
    let polling = false;
    const timer = setInterval(async () => {
        if (closed || polling) return;
        polling = true;
        try { await evaluate('window.ferroui.devtools.poll()'); } catch (error) { onError(error); }
        finally { polling = false; }
    }, pollInterval);
    timer.unref?.();
    const bridge = {
        async send(method, params = {}, sessionId) {
            const id = next++;
            const request = { id, method, params, ...(sessionId ? { sessionId } : {}) };
            const messages = await evaluate(`window.ferroui.devtools.handle(${JSON.stringify(request)})`);
            const response = messages.find(m => m.id === id);
            if (!response) throw new Error(`No response for ${method}`);
            if (response.error) throw Object.assign(new Error(response.error.message), { code: response.error.code });
            if (method === 'Target.attachToTarget' || method === 'Target.attachToBrowserTarget') sessions.add(response.result.sessionId);
            if (method === 'Target.detachFromTarget') sessions.delete(params.sessionId);
            return response.result;
        },
        close() {
            if (closing) return closing;
            clearInterval(timer);
            closing = (async () => {
                try {
                    for (const sessionId of [...sessions].reverse()) {
                        try { await bridge.send('Target.detachFromTarget', { sessionId }); }
                        catch (error) { onError(error); }
                    }
                } finally { closed = true; }
            })();
            return closing;
        }
    };
    return bridge;
}
