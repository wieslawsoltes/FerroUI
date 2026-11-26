import { createHash } from 'node:crypto';
import * as vscode from 'vscode';
import type { CollaborationClient } from './collaborationClient';
import type { GuestRole, RoomReview, RoomStatus } from './collaborationProtocol';
import { errorMessage } from './protocol';

export interface SavedRoom {
    id: string;
    uri: string;
    label: string;
    base?: string;
}

export interface RoomSourceReview extends RoomReview {
    editor_version: number;
    local: string;
    can_publish: boolean;
}

export type SourceAction = 'apply' | 'publish' | 'publish_resolution' | 'checkpoint' | 'checkpoint_resolution';

export class CollaborationDocument {
    private task = Promise.resolve();
    private pending = 0;
    private timer?: NodeJS.Timeout;
    private stopping = false;
    private snapshot?: RoomReview;
    private submitted?: string;
    private processError?: string;

    constructor(readonly saved: SavedRoom, readonly document: vscode.TextDocument, private readonly client: CollaborationClient,
        private status: RoomStatus, private readonly changed: () => void,
        private readonly remember: (room: SavedRoom) => Promise<void>) {}

    async start(initialSource: string, initialEol: vscode.EndOfLine): Promise<void> {
        if (this.status.revision) {
            const snapshot = await this.readSnapshot();
            if (!snapshot.rebase && initialSource === normalizeLines(snapshot.source, initialEol)) await this.saveBase(snapshot.source);
        }
        this.schedule();
    }

    get current(): RoomStatus {
        return this.processError ? { ...this.status, phase: 'blocked', message: this.processError, revision: null, online: [] } : this.status;
    }

    get needsReview(): boolean {
        return Boolean(this.snapshot?.rebase || (this.snapshot && this.saved.base !== sourceHash(this.snapshot.source)));
    }

    async read(signal?: AbortSignal): Promise<RoomSourceReview> {
        return this.run(async () => {
            const version = this.document.version;
            const local = this.document.getText();
            const snapshot = await this.readSnapshot();
            this.requireVersion(version, signal);
            return { ...snapshot, editor_version: version, local, can_publish: this.canPublish(snapshot) };
        });
    }

    async synchronize(signal?: AbortSignal): Promise<RoomStatus> {
        return this.run(async () => { signal?.throwIfAborted(); await this.poll(true); return this.current; });
    }

    async source(action: SourceAction, version: number, revision: string, validate: () => void, signal?: AbortSignal): Promise<RoomStatus> {
        return this.run(async () => {
            const check = () => { this.requireVersion(version, signal); validate(); };
            check();
            const local = this.document.getText();
            let snapshot = await this.readSnapshot();
            if (snapshot.revision !== revision) throw new Error('The room version changed. Open the current source review.');
            check();
            const draft = normalizeLines(local, roomEndOfLine(snapshot.source));
            switch (action) {
                case 'apply': {
                    if (snapshot.rebase) {
                        this.status = await this.client.call({ action: 'resolve', revision, local });
                        const current = await this.readSnapshot();
                        if (current.source !== snapshot.source || current.rebase) throw new Error('The room source changed. Review the latest version.');
                        snapshot = current;
                    }
                    check();
                    this.status = await this.client.call({ action: 'prepare_apply', revision: snapshot.revision, before: local });
                    check();
                    await this.applySource(snapshot.source);
                    await this.saveBase(snapshot.source);
                    break;
                }
                case 'publish':
                    if (!this.canPublish(snapshot)) throw new Error('Review the current room source before publishing your draft.');
                    this.status = await this.client.call({ action, revision, source: draft });
                    this.submitted = draft;
                    break;
                case 'publish_resolution':
                    this.status = snapshot.rebase
                        ? await this.client.call({ action: 'resolve', revision, local, resolved: draft })
                        : await this.client.call({ action: 'publish', revision, source: draft });
                    this.submitted = draft;
                    break;
                case 'checkpoint': case 'checkpoint_resolution':
                    this.status = await this.client.call({ action: 'checkpoint', revision, local,
                        resolved: action === 'checkpoint_resolution' ? draft : undefined });
                    if (action === 'checkpoint_resolution') this.submitted = draft;
                    break;
            }
            this.changed();
            return this.current;
        });
    }

    async getRecovery(): Promise<string> { return this.run(() => this.client.call({ action: 'recovery' })); }

    async readRecovery(signal?: AbortSignal) {
        const version = this.document.version;
        const source = await this.getRecovery();
        this.requireVersion(version, signal);
        return { editor_version: version, source, backup: sourceHash(source) };
    }

    async applyRecovery(version: number, backup: string, signal?: AbortSignal): Promise<{ editor_version: number }> {
        return this.run(async () => {
            this.requireVersion(version, signal);
            const source = await this.client.call<string>({ action: 'recovery' });
            if (sourceHash(source) !== backup) throw new Error('The saved backup changed. Review it again.');
            this.requireVersion(version, signal);
            await this.applySource(source);
            this.changed();
            return { editor_version: this.document.version };
        });
    }

    async recover(version: number, source: string, validate: () => void): Promise<void> {
        return this.run(async () => {
            this.requireVersion(version);
            validate();
            await this.applySource(source);
            this.changed();
        });
    }

    async invite(role: GuestRole, signal?: AbortSignal): Promise<string> {
        return this.run(async () => { signal?.throwIfAborted(); return this.client.call({ action: 'invite', role }); });
    }

    async member(actor: string, role: GuestRole, revoked: boolean, signal?: AbortSignal): Promise<RoomStatus> {
        return this.run(async () => {
            signal?.throwIfAborted();
            this.status = await this.client.call({ action: 'member', actor, role, revoked });
            this.changed();
            return this.current;
        });
    }

    async stop(): Promise<void> {
        this.stopping = true;
        clearTimeout(this.timer);
        await this.client.stop();
    }

    private requireVersion(version: number, signal?: AbortSignal): void {
        signal?.throwIfAborted();
        if (this.stopping || !vscode.workspace.isTrusted || this.document.isClosed || this.document.version !== version) {
            throw new Error('The local editor changed. Open the current source review.');
        }
    }

    private canPublish(snapshot: RoomReview): boolean {
        return this.current.role !== 'viewer' && !snapshot.rebase && this.saved.base === sourceHash(snapshot.source);
    }

    private async readSnapshot(): Promise<RoomReview> {
        if (this.snapshot && this.snapshot.revision === this.current.revision) return this.snapshot;
        const snapshot = await this.client.call<RoomReview>({ action: 'review' });
        this.snapshot = snapshot;
        if (!snapshot.rebase && (this.submitted === snapshot.source || this.document.getText() === this.forEditor(snapshot.source))) {
            await this.saveBase(snapshot.source);
            this.submitted = undefined;
        }
        return snapshot;
    }

    private async saveBase(source: string): Promise<void> {
        const hash = sourceHash(source);
        if (this.saved.base !== hash) {
            this.saved.base = hash;
            await this.remember(this.saved);
        }
    }

    forEditor(source: string): string { return normalizeLines(source, this.document.eol); }

    private async applySource(source: string): Promise<void> {
        const text = this.forEditor(source);
        if (this.document.getText() === text) return;
        const last = this.document.positionAt(this.document.getText().length);
        const workspace = new vscode.WorkspaceEdit();
        workspace.replace(this.document.uri, new vscode.Range(new vscode.Position(0, 0), last), text);
        if (!await vscode.workspace.applyEdit(workspace) || this.document.getText() !== text) {
            throw new Error('VS Code could not install the exact source review in the editor.');
        }
    }

    private async poll(refresh: boolean): Promise<void> {
        const previous = JSON.stringify(this.status);
        const previousReview = this.snapshot?.revision;
        this.status = await this.client.call({ action: 'tick', refresh });
        if (this.status.revision && this.status.revision !== previousReview) await this.readSnapshot();
        if (previous !== JSON.stringify(this.status) || previousReview !== this.snapshot?.revision) this.changed();
    }

    private schedule(): void {
        this.timer = setTimeout(async () => {
            try {
                if (this.pending === 0 && vscode.workspace.isTrusted) await this.run(() => this.poll(false));
            } catch (error) {
                if (!this.stopping && this.client.closed) { this.processError = errorMessage(error); this.changed(); }
            } finally {
                if (!this.stopping && !this.client.closed) this.schedule();
            }
        }, 1800);
    }

    private async run<T>(operation: () => Promise<T>): Promise<T> {
        this.pending++;
        const result = this.task.then(() => {
            if (this.stopping || !vscode.workspace.isTrusted || this.document.isClosed) throw new Error('The room is no longer attached to the local editor.');
            return operation();
        });
        this.task = result.then(() => {}, () => {});
        try { return await result; }
        finally { this.pending--; }
    }
}

function roomEndOfLine(source: string): vscode.EndOfLine {
    return source.includes('\r\n') && !/(^|[^\r])\n/.test(source) ? vscode.EndOfLine.CRLF : vscode.EndOfLine.LF;
}

function normalizeLines(source: string, eol: vscode.EndOfLine): string {
    return source.replace(/\r\n|\r|\n/g, eol === vscode.EndOfLine.CRLF ? '\r\n' : '\n');
}

function sourceHash(source: string): string { return createHash('sha256').update(source).digest('hex'); }
