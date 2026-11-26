import type { StageContact, StagePointerDevice, StagePointerInput, StageTarget } from '../src/stagesProtocol';

export class StageContacts {
    private target?: StageTarget;
    private readonly contacts = new Map<number, StageContact>();
    private readonly pending = new Set<number>();
    private frame = 0;

    constructor(private readonly image: HTMLImageElement, private readonly send: (input: StagePointerInput) => void,
        private readonly released: () => void) {
        image.onpointerdown = event => this.begin(event);
        image.onpointermove = event => this.move(event);
        image.onpointerup = event => this.finish(event.pointerId, event);
        image.onpointercancel = event => this.finish(event.pointerId);
        image.onlostpointercapture = event => this.finish(event.pointerId);
    }

    update(target: StageTarget | undefined): void {
        if (this.target?.id === target?.id && this.target?.epoch === target?.epoch) return;
        this.cancel();
        this.target = target;
        this.image.classList.toggle('stage-contact-surface', target !== undefined);
    }

    cancel(): void {
        cancelAnimationFrame(this.frame);
        this.frame = 0;
        this.pending.clear();
        for (const contact of this.contacts.keys()) this.finish(contact);
    }

    private begin(event: PointerEvent): void {
        if (!this.target || event.button !== 0 || this.contacts.has(event.pointerId) || this.contacts.size >= 10) return;
        const device: StagePointerDevice = event.pointerType === 'touch' || event.pointerType === 'pen' ? event.pointerType : 'mouse';
        const input = this.sample(event, device, 'down');
        this.image.setPointerCapture(event.pointerId);
        event.preventDefault();
        this.contacts.set(event.pointerId, input);
        this.emit(input);
    }

    private move(event: PointerEvent): void {
        const last = this.contacts.get(event.pointerId);
        if (!last) return;
        this.contacts.set(event.pointerId, this.sample(event, last.device, 'move'));
        this.pending.add(event.pointerId);
        if (!this.frame) this.frame = requestAnimationFrame(() => {
            this.frame = 0;
            for (const contact of this.pending) this.emit(this.contacts.get(contact)!);
            this.pending.clear();
        });
    }

    private finish(contact: number, event?: PointerEvent): void {
        const last = this.contacts.get(contact);
        if (!last) return;
        if (this.pending.delete(contact) && event) this.emit(last);
        this.contacts.delete(contact);
        this.emit(event ? this.sample(event, last.device, 'up') : { ...last, phase: 'cancel' });
        if (this.image.hasPointerCapture(contact)) this.image.releasePointerCapture(contact);
        if (event && this.contacts.size === 0) this.released();
    }

    private emit(input: StageContact): void {
        if (this.target) this.send({ ...this.target, input });
    }

    private sample(event: PointerEvent, device: StagePointerDevice, phase: StageContact['phase']): StageContact {
        const bounds = this.image.getBoundingClientRect();
        return {
            contact: event.pointerId >>> 0, phase, device,
            x: (event.clientX - bounds.left) / bounds.width, y: (event.clientY - bounds.top) / bounds.height,
            pressure: event.pressure, tiltX: event.tiltX, tiltY: event.tiltY, twist: event.twist,
        };
    }
}
