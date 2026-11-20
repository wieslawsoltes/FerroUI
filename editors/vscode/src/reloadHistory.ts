import * as vscode from 'vscode';
import type { ReloadFrame, ReloadFrameIdentity, RuntimeFrameReceipt } from './reloadFrames';

type Outcome = 'applied' | 'rejected' | 'rebuilt' | 'buildFailed' | 'superseded' | 'stopped';
const capacity = 256;
export interface ReloadRecord {
    readonly id: number;
    readonly file: string;
    readonly revision: string;
    readonly observed: number;
    outcome?: Outcome;
    elapsedMs?: number;
    acknowledgedMs?: number;
    pid?: number;
    message?: string;
    generation?: number;
    frame?: ReloadFrame;
}

export class ReloadHistory implements vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private readonly records: ReloadRecord[] = [];
    private readonly earlyFrames = new Map<number, RuntimeFrameReceipt>();
    private nextId = 0;
    private nextGeneration = 0;
    private activeGeneration?: number;

    observe(file: string, revision: string, observed: number): ReloadRecord {
        const record = { id: ++this.nextId, file, revision, observed };
        this.records.push(record);
        if (this.records.length > capacity) this.records.shift();
        this.changed.fire();
        return record;
    }

    startGeneration(): number {
        if (this.activeGeneration !== undefined) this.endGeneration(this.activeGeneration);
        this.activeGeneration = ++this.nextGeneration;
        return this.activeGeneration;
    }

    finish(record: ReloadRecord, outcome: Outcome, pid?: number, message?: string, identity?: ReloadFrameIdentity): void {
        if (record.outcome !== undefined) return;
        record.outcome = outcome;
        record.elapsedMs = Math.max(0, Date.now() - record.observed);
        record.pid = pid;
        record.message = message;
        if (identity) {
            record.generation = identity.generation;
            record.acknowledgedMs = elapsedSince(record.observed, identity.acknowledgedAt);
            if (identity.frame !== undefined) {
                const active = identity.generation === this.activeGeneration;
                record.frame = { receipt: identity.frame, status: active ? 'pending' : 'processExited' };
                const receipt = this.earlyFrames.get(identity.frame);
                if (receipt && active) this.recordFrame(record, receipt);
            }
        }
        this.changed.fire();
    }

    observeFrame(generation: number, receipt: RuntimeFrameReceipt): void {
        if (generation !== this.activeGeneration || this.earlyFrames.has(receipt.frame)) return;
        let applied = false;
        for (const record of this.records) {
            if (record.generation === generation && record.frame?.receipt === receipt.frame && record.frame.status === 'pending') {
                this.recordFrame(record, receipt);
                applied = true;
            }
        }
        // A render can finish before VS Code observes the save or activates a candidate.
        this.earlyFrames.set(receipt.frame, receipt);
        if (this.earlyFrames.size > capacity) this.earlyFrames.delete(this.earlyFrames.keys().next().value!);
        if (applied) this.changed.fire();
    }

    endGeneration(generation: number): void {
        if (generation !== this.activeGeneration) return;
        this.activeGeneration = undefined;
        this.earlyFrames.clear();
        let changed = false;
        for (const record of this.records) {
            if (record.generation === generation && record.frame?.status === 'pending') {
                record.frame.status = 'processExited';
                changed = true;
            }
        }
        if (changed) this.changed.fire();
    }

    private recordFrame(record: ReloadRecord, receipt: RuntimeFrameReceipt): void {
        record.frame = {
            receipt: receipt.frame, batch: receipt.batch, status: receipt.status,
            afterAcknowledgementMs: receipt.afterAcknowledgementMs ?? undefined,
            elapsedMs: receipt.status === 'rendered' ? elapsedSince(record.observed, receipt.unixMs) : undefined,
        };
    }

    recent(limit: number): ReloadRecord[] { return this.records.slice(-limit).reverse(); }

    dispose(): void { this.changed.dispose(); }
}

function elapsedSince(observed: number, completion: number | null | undefined): number | undefined {
    if (completion != null && completion >= observed) return completion - observed;
}
