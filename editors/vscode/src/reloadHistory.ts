import * as vscode from 'vscode';

type Outcome = 'applied' | 'rejected' | 'rebuilt' | 'buildFailed' | 'superseded' | 'stopped';
export interface ReloadRecord {
    readonly id: number;
    readonly file: string;
    readonly revision: string;
    readonly observed: number;
    outcome?: Outcome;
    elapsedMs?: number;
    pid?: number;
    message?: string;
}

export class ReloadHistory implements vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private readonly records: ReloadRecord[] = [];
    private nextId = 0;

    observe(file: string, revision: string, observed: number): ReloadRecord {
        const record = { id: ++this.nextId, file, revision, observed };
        this.records.push(record);
        if (this.records.length > 256) this.records.shift();
        this.changed.fire();
        return record;
    }

    finish(record: ReloadRecord, outcome: Outcome, pid?: number, message?: string): void {
        if (record.outcome !== undefined) return;
        record.outcome = outcome;
        record.elapsedMs = Math.max(0, Date.now() - record.observed);
        record.pid = pid;
        record.message = message;
        this.changed.fire();
    }

    recent(limit: number): ReloadRecord[] { return this.records.slice(-limit).reverse(); }

    dispose(): void { this.changed.dispose(); }
}
