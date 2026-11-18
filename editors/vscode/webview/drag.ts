import type { DragGesture, DragMessage, DragOrigin, DropTarget, FrameTarget } from '../src/protocol';

const mime = 'application/vnd.ferroui.designer-drag';
type Send = (message: DragMessage) => void;

export function startDrag(event: DragEvent, frame: FrameTarget, origin: DragOrigin, send: Send, label?: string): DragGesture | undefined {
    const transfer = event.dataTransfer;
    if (!transfer) return undefined;
    const gesture = { id: crypto.randomUUID(), frame, origin };
    transfer.setData(mime, gesture.id);
    transfer.effectAllowed = origin.type === 'asset' ? 'copy' : 'move';
    if (label) {
        const ghost = document.createElement('span');
        ghost.className = 'drag-label';
        ghost.textContent = label;
        document.body.append(ghost);
        transfer.setDragImage(ghost, 12, 12);
        setTimeout(() => ghost.remove(), 0);
    }
    send({ type: 'beginDrag', gesture });
    return gesture;
}

export function matchDrag(event: DragEvent, frame: FrameTarget | undefined, gesture: DragGesture | undefined): gesture is DragGesture {
    return Boolean(frame && gesture && frame.session === gesture.frame.session && frame.token === gesture.frame.token
        && event.dataTransfer?.types.includes(mime));
}

export function allowDrop(event: DragEvent, gesture: DragGesture): void {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = gesture.origin.type === 'asset' ? 'copy' : 'move';
}

export function finishDrop(event: DragEvent, frame: FrameTarget | undefined, gesture: DragGesture | undefined, target: DropTarget, send: Send): void {
    if (!frame || !matchDrag(event, frame, gesture) || event.dataTransfer?.getData(mime) !== gesture.id) return;
    event.preventDefault();
    send({ type: 'drop', id: gesture.id, frame, target });
}
