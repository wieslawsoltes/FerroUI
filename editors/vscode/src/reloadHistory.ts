import * as vscode from 'vscode';
import type { ReloadFrame, ReloadFrameIdentity, RuntimeFrameReceipt } from './reloadFrames';

type Outcome = 'applied' | 'rejected' | 'rebuilt' | 'buildFailed' | 'superseded' | 'stopped';
const capacity = 256;
export interface ReloadIdentity { session: string; target: string }

export interface ReloadRecord {
    readonly id: number;
    readonly file: string;
    readonly revision: string;
    readonly observed: number;
    readonly session?: string;
    readonly target?: string;
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
    private readonly generations = new Map<number, { session?: string; frames: Map<number, RuntimeFrameReceipt> }>();
    private readonly active = new Map<string | undefined, number>();
    private nextId = 0;
    private nextGeneration = 0;
    observe(file: string, revision: string, observed: number, run?: ReloadIdentity): ReloadRecord {
        const record = { id: ++this.nextId, file, revision, observed, ...run };
        this.records.push(record);
        if (this.records.length > capacity) this.records.shift();
        this.changed.fire();
        return record;
    }

    startGeneration(session?: string): number {
        const prior = this.active.get(session);
        if (prior !== undefined) this.endGeneration(prior);
        const generation = ++this.nextGeneration;
        this.active.set(session, generation);
        this.generations.set(generation, { session, frames: new Map() });
        return generation;
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
                const process = this.generations.get(identity.generation);
                const active = process !== undefined && process.session === record.session;
                record.frame = { receipt: identity.frame, status: active ? 'pending' : 'processExited' };
                const receipt = active ? process.frames.get(identity.frame) : undefined;
                if (receipt) this.recordFrame(record, receipt);
            }
        }
        this.changed.fire();
    }

    observeFrame(generation: number, receipt: RuntimeFrameReceipt): void {
        const process = this.generations.get(generation);
        if (!process || process.frames.has(receipt.frame)) return;
        let applied = false;
        for (const record of this.records) {
            if (record.session === process.session && record.generation === generation && record.frame?.receipt === receipt.frame && record.frame.status === 'pending') {
                this.recordFrame(record, receipt);
                applied = true;
            }
        }
        // A render can finish before VS Code observes the save or activates a candidate.
        process.frames.set(receipt.frame, receipt);
        if (process.frames.size > capacity) process.frames.delete(process.frames.keys().next().value!);
        if (applied) this.changed.fire();
    }

    endGeneration(generation: number): void {
        const process = this.generations.get(generation);
        if (!process) return;
        this.generations.delete(generation);
        this.active.delete(process.session);
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

    recent(limit: number, session?: string): ReloadRecord[] {
        const records = session === undefined ? this.records : this.records.filter(record => record.session === session);
        return records.slice(-limit).reverse();
    }

    dispose(): void { this.changed.dispose(); }
}

function elapsedSince(observed: number, completion: number | null | undefined): number | undefined {
    if (completion != null && completion >= observed) return completion - observed;
}
