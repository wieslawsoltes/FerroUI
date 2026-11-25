interface StageDrag {
    current(): boolean;
    destination(hit: Element | null): HTMLElement | undefined;
    drop(card: HTMLElement): void;
}

/** Track stage gestures within the webview and preserve the handle's regular click. */
export function connectStageDrag(handle: HTMLElement, begin: () => StageDrag | undefined): void {
    let suppressClick = false;
    handle.addEventListener('dragstart', event => event.preventDefault());
    handle.addEventListener('pointerdown', event => {
        if (event.button !== 0 || !event.isPrimary) return;
        const drag = begin();
        if (!drag?.current()) return;
        event.preventDefault();
        handle.focus({ preventScroll: true });
        suppressClick = false;

        const { clientX: x, clientY: y, pointerId } = event;
        const listeners = new AbortController();
        const options = { capture: true, signal: listeners.signal };
        let moved = false;
        let highlighted: HTMLElement | undefined;
        const highlight = (card?: HTMLElement) => {
            if (card === highlighted) return;
            if (highlighted) delete highlighted.dataset.drop;
            highlighted = card;
            if (highlighted) highlighted.dataset.drop = 'true';
        };
        const destination = (pointer: PointerEvent) => drag.destination(document.elementFromPoint(pointer.clientX, pointer.clientY));
        const passedThreshold = (pointer: PointerEvent) => moved || Math.hypot(pointer.clientX - x, pointer.clientY - y) >= 5;
        const finish = () => {
            listeners.abort();
            highlight();
            if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId);
        };
        const cancel = () => { suppressClick = moved; finish(); };
        document.addEventListener('pointermove', pointer => {
            if (pointer.pointerId !== pointerId) return;
            if (!drag.current()) { cancel(); return; }
            moved = passedThreshold(pointer);
            if (moved) highlight(destination(pointer));
        }, options);
        document.addEventListener('pointerup', pointer => {
            if (pointer.pointerId !== pointerId) return;
            suppressClick = passedThreshold(pointer);
            const card = suppressClick && drag.current() ? destination(pointer) : undefined;
            finish();
            if (card) drag.drop(card);
        }, options);
        document.addEventListener('pointercancel', pointer => {
            if (pointer.pointerId === pointerId) cancel();
        }, options);
        handle.addEventListener('lostpointercapture', pointer => {
            if (pointer.pointerId === pointerId) cancel();
        }, options);
        window.addEventListener('blur', cancel, options);
        document.addEventListener('keydown', event => {
            if (event.key === 'Escape') cancel();
        }, options);
        handle.setPointerCapture(pointerId);
    });
    handle.addEventListener('click', event => {
        if (!suppressClick || event.detail === 0) return;
        suppressClick = false;
        event.stopImmediatePropagation();
    }, true);
}
