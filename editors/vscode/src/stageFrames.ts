import type { StageImage, StageInfo, StageTarget } from './stagesProtocol';

/** Frame delivery follows the accepted cards without holding the board's edit lock. */
export class StageFrames {
    private targets = new Map<number, StageTarget>();
    private readonly pending = new Map<StageTarget, boolean>();
    private readonly delivered = new WeakSet<StageTarget>();
    private active?: StageTarget;
    private timer?: ReturnType<typeof setTimeout>;

    constructor(private readonly capture: (target: StageTarget) => Promise<StageImage>,
        private readonly present: (target: StageTarget, image: StageImage) => void,
        private readonly report: (error: unknown) => void) {}

    setStages(stages: readonly Pick<StageInfo, 'id' | 'epoch' | 'rendered_revision'>[]): void {
        const next = new Map<number, StageTarget>();
        for (const { id, epoch, rendered_revision } of stages) {
            if (rendered_revision === null) continue;
            const previous = this.targets.get(id);
            const target = previous?.epoch === epoch ? previous : { id, epoch };
            next.set(id, target);
            if (target !== this.active && !this.delivered.has(target) && !this.pending.has(target)) {
                this.pending.set(target, false);
            }
        }
        this.targets = next;
        for (const target of this.pending.keys()) {
            if (!this.isCurrent(target)) this.pending.delete(target);
        }
        this.schedule(0);
    }

    request({ id, epoch }: StageTarget): void {
        const target = this.targets.get(id);
        if (target?.epoch !== epoch) return;
        this.pending.set(target, true);
        this.schedule(32);
    }

    clear(): void {
        this.targets.clear();
        this.pending.clear();
        clearTimeout(this.timer);
        this.timer = undefined;
    }

    private isCurrent(target: StageTarget): boolean {
        return this.targets.get(target.id) === target;
    }

    private schedule(delay: number): void {
        if (this.active || this.timer !== undefined || !this.pending.size) return;
        this.timer = setTimeout(() => {
            this.timer = undefined;
            void this.sendNext();
        }, delay);
    }

    private async sendNext(): Promise<void> {
        // Interactive updates take the next slot while a multi-card set is rendering.
        const target = [...this.pending].find(([, interactive]) => interactive)?.[0] ?? this.pending.keys().next().value;
        if (!target) return;
        this.pending.delete(target);
        this.active = target;
        try {
            const image = await this.capture(target);
            if (this.isCurrent(target)) {
                this.delivered.add(target);
                this.present(target, image);
            }
        } catch (error) {
            if (this.isCurrent(target)) this.report(error);
        } finally {
            this.active = undefined;
            this.schedule(0);
        }
    }
}
