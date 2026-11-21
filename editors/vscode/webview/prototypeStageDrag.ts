/** Connect stages inside one webview while preserving the title's click and keyboard actions. */
export function connectStageDrag(handle: HTMLButtonElement, source: number,
    current: () => boolean, connect: (destination: number) => void): void {
    let suppressClick = false;
    handle.addEventListener('dragstart', event => event.preventDefault());
    handle.addEventListener('pointerdown', event => {
        if (event.button !== 0 || !event.isPrimary || !current()) return;
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
        const destination = (pointer: PointerEvent) => {
            const card = document.elementFromPoint(pointer.clientX, pointer.clientY)?.closest<HTMLElement>('[data-prototype-stage]');
            return card && Number(card.dataset.prototypeStage) !== source ? card : undefined;
        };
        const passedThreshold = (pointer: PointerEvent) => moved || Math.hypot(pointer.clientX - x, pointer.clientY - y) >= 5;
        const finish = () => {
            listeners.abort();
            highlight();
            if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId);
        };
        document.addEventListener('pointermove', pointer => {
            if (pointer.pointerId !== pointerId) return;
            if (!current()) { finish(); return; }
            moved = passedThreshold(pointer);
            if (moved) highlight(destination(pointer));
        }, options);
        document.addEventListener('pointerup', pointer => {
            if (pointer.pointerId !== pointerId) return;
            suppressClick = passedThreshold(pointer);
            const card = suppressClick && current() ? destination(pointer) : undefined;
            finish();
            if (card) connect(Number(card.dataset.prototypeStage));
        }, options);
        document.addEventListener('pointercancel', pointer => {
            if (pointer.pointerId === pointerId) finish();
        }, options);
        window.addEventListener('blur', finish, options);
        handle.setPointerCapture(pointerId);
    });
    handle.addEventListener('click', event => {
        if (!suppressClick || event.detail === 0) return;
        suppressClick = false;
        event.stopImmediatePropagation();
    }, true);
}
