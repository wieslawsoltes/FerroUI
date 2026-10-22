// Read-only evidence around real browser input. The listeners never dispatch
// events, call application commands, poll the native bridge or alter defaults.
// Native state is sampled by the caller outside the input handlers so collecting
// evidence does not put control-tree serialization inside a multi-click window.
export async function startInputObservation(page, types = ["pointerdown", "pointerup", "mousedown", "mouseup", "dblclick"], limit = 64) {
    await page.evaluate(({ types, limit }) => {
        if (globalThis.__ferroDesignInputObservation) throw new Error("An input observation is already active");
        const started = performance.now(), events = [], longTasks = [];
        let droppedEvents = 0, droppedLongTasks = 0;
        const record = (phase) => (event) => {
            if (events.length >= limit) { droppedEvents++; return; }
            events.push({ type: event.type, phase, trusted: event.isTrusted,
                eventTimeMs: event.timeStamp, observedAtMs: performance.now(),
                detail: event.detail, button: event.button, buttons: event.buttons,
                pointerId: event.pointerId ?? null, pointerType: event.pointerType ?? null,
                x: event.clientX, y: event.clientY,
                target: event.target instanceof Element ? {
                    tag: event.target.tagName, className: event.target.getAttribute("class"),
                } : null });
        };
        const before = record("capture"), after = record("bubble");
        for (const type of types) {
            document.addEventListener(type, before, { capture: true, passive: true });
            document.addEventListener(type, after, { passive: true });
        }
        const collectTasks = (entries) => {
            for (const entry of entries) {
                if (longTasks.length < limit) longTasks.push({ startTimeMs: entry.startTime, durationMs: entry.duration });
                else droppedLongTasks++;
            }
        };
        const supported = globalThis.PerformanceObserver?.supportedEntryTypes?.includes("longtask") ?? false;
        const observer = supported ? new PerformanceObserver((list) => collectTasks(list.getEntries())) : null;
        observer?.observe({ type: "longtask" });
        globalThis.__ferroDesignInputObservation = { stop() {
            for (const type of types) {
                document.removeEventListener(type, before, true);
                document.removeEventListener(type, after);
            }
            if (observer) { collectTasks(observer.takeRecords()); observer.disconnect(); }
            return { clock: "page performance milliseconds", startedAtMs: started, endedAtMs: performance.now(),
                events, droppedEvents, longTaskSupported: supported, longTasks, droppedLongTasks };
        } };
    }, { types, limit: Math.max(1, Math.min(1024, limit)) });
}

export async function stopInputObservation(page) {
    return page.evaluate(() => {
        const observation = globalThis.__ferroDesignInputObservation;
        if (!observation) return null;
        delete globalThis.__ferroDesignInputObservation;
        return observation.stop();
    });
}

export function inlineInputState(view) {
    const names = ["CanvasHost", "InlineTextHost", "InlineTextBox", "ModalBackdrop", "PageInspector", "DesignInspector"];
    return {
        renderedFrames: view.renderedFrames, sceneFrames: view.sceneFrames,
        documentRevision: view.documentRevision, historyLength: view.historyLength,
        selection: view.selection, editingText: view.editingText,
        pointerPresses: view.pointerPresses ?? null, zoom: view.zoom, pan: view.pan, tool: view.tool,
        chrome: Object.fromEntries(names.map(name => {
            const control = view.chrome[name];
            return [name, control ? Object.fromEntries(["x", "y", "width", "height", "visible", "enabled",
                "measureValid", "arrangeValid", "focused", "focusWithin"].map(key => [key, control[key]])) : null];
        })),
    };
}
