import { randomUUID } from 'node:crypto';
import { stat } from 'node:fs/promises';
import { basename, join } from 'node:path';
import * as vscode from 'vscode';
import { CollaborationClient } from './collaborationClient';
import { CollaborationDocument, type SavedRoom } from './collaborationDocument';
import type { GuestRole, RoomStart, RoomStatus } from './collaborationProtocol';
import { CollaborationReviews } from './collaborationReviews';
import { CollaborationView, RoomItem } from './collaborationView';
import { errorMessage } from './protocol';
import { choose } from './studioContext';
import { xamlLanguage } from './xamlServer';

const roomKey = 'ferroui.collaboration.rooms';
const title = 'FerroUI Collaboration';
type Start = Exclude<RoomStart['action'], 'resume'>;

export class Collaboration implements vscode.Disposable {
    private readonly saved: Map<string, SavedRoom>;
    private readonly rooms = new Map<string, CollaborationDocument>();
    private readonly starting = new Set<string>();
    private readonly startingClients = new Set<CollaborationClient>();
    private readonly view = new CollaborationView(() => [...this.rooms.values()]);
    private readonly reviews = new CollaborationReviews();
    private readonly commands: vscode.Disposable[];
    private disposed = false;

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        private readonly activeDocument: () => vscode.TextDocument | undefined) {
        this.saved = new Map((context.workspaceState.get<SavedRoom[]>(roomKey) ?? []).map(item => [item.id, item]));
        this.commands = [
            this.command('host', () => this.begin('host')), this.command('join', () => this.begin('join')),
            this.command('resume', () => this.openSaved()),
            this.command('review', item => this.reviews.open(this.target(item), 'apply')),
            this.command('publish', item => this.reviews.open(this.target(item), 'publish')),
            this.command('resolve', item => this.reviews.open(this.target(item), 'publish_resolution')),
            this.command('sync', async item => { await this.target(item).synchronize(); }),
            this.command('checkpoint', item => this.checkpoint(this.target(item))),
            this.command('checkpointResolution', item => this.reviews.open(this.target(item), 'checkpoint_resolution')),
            this.command('recover', item => this.reviews.recovery(this.target(item))),
            this.command('invite', item => this.invite(this.target(item))),
            this.command('member', item => this.member(item)),
            this.command('disconnect', item => this.disconnect(this.target(item))),
            vscode.workspace.onDidCloseTextDocument(document => {
                for (const room of this.rooms.values()) if (room.document === document) void this.disconnect(room);
            }),
        ];
    }

    list() {
        return {
            connected: [...this.rooms.values()].map(room => ({ ...room.current, session: room.saved.id, uri: room.saved.uri,
                editor_version: room.document.version, review_available: room.needsReview })),
            saved: [...this.saved.values()].map(({ id, uri, label }) => ({ session: id, uri, label })),
        };
    }

    find(id: string): CollaborationDocument {
        const room = this.rooms.get(id);
        if (!room) throw new Error('Choose an active FerroUI room session.');
        return room;
    }

    host(document: vscode.TextDocument, displayName: string): Promise<CollaborationDocument> {
        const settings = vscode.workspace.getConfiguration('ferroui.collaboration');
        return this.connect(document, { action: 'host', title: basename(document.fileName), source: document.getText(), display_name: displayName,
            public_url: settings.get<string>('publicEndpoint')?.trim() || undefined,
            bind: settings.get<string>('bindAddress')?.trim() || undefined });
    }

    join(document: vscode.TextDocument, invitation: string, displayName: string): Promise<CollaborationDocument> {
        return this.connect(document, { action: 'join', invitation, display_name: displayName });
    }

    async resume(id: string): Promise<CollaborationDocument> {
        const saved = this.saved.get(id);
        if (!saved) throw new Error('Choose a saved FerroUI room session.');
        const document = await vscode.workspace.openTextDocument(vscode.Uri.parse(saved.uri, true));
        return this.connect(document, { action: 'resume' }, saved);
    }

    async copyInvitation(room: CollaborationDocument, role: GuestRole, signal?: AbortSignal): Promise<void> {
        const code = await room.invite(role, signal);
        signal?.throwIfAborted();
        await vscode.env.clipboard.writeText(code);
    }

    async disconnect(room: CollaborationDocument): Promise<void> {
        if (this.rooms.get(room.saved.id) !== room) return;
        this.rooms.delete(room.saved.id);
        this.starting.add(room.saved.uri);
        this.view.refresh();
        try { await room.stop(); }
        finally { this.starting.delete(room.saved.uri); }
    }

    private async connect(document: vscode.TextDocument, start: RoomStart, previous?: SavedRoom): Promise<CollaborationDocument> {
        requireDocument(document);
        const saved = previous ?? { id: randomUUID(), uri: document.uri.toString(), label: basename(document.fileName) };
        if (this.disposed || saved.uri !== document.uri.toString() || this.starting.has(saved.uri)
            || [...this.rooms.values()].some(room => room.saved.uri === saved.uri)) throw new Error('Disconnect the current room for this file first.');
        if (!/^[a-f0-9-]{36}$/.test(saved.id)) throw new Error('The saved room identity is invalid.');
        const directory = join(this.context.globalStorageUri.fsPath, 'collaboration', saved.id);
        const version = document.version;
        const initialSource = document.getText();
        const initialEol = document.eol;
        let client: CollaborationClient | undefined;
        let connected = false;
        this.starting.add(saved.uri);
        try {
            await this.remember(saved);
            client = await CollaborationClient.start(this.context, this.output);
            this.startingClients.add(client);
            requireDocument(document);
            if (this.disposed || (start.action === 'host' && version !== document.version)) throw new Error('The source changed before opening the room.');
            const status = await client.call<RoomStatus>({ action: 'open', options: { directory, uri: saved.uri, start } });
            if (this.disposed || document.isClosed) throw new Error('The local editor closed while connecting. Resume the saved room when ready.');
            const room = new CollaborationDocument(saved, document, client, status, () => this.view.refresh(), state => this.remember(state));
            await room.start(initialSource, initialEol);
            if (this.disposed || document.isClosed) {
                await room.stop();
                throw new Error('The local editor closed while connecting. Resume the saved room when ready.');
            }
            this.rooms.set(saved.id, room);
            connected = true;
            this.view.refresh();
            return room;
        } finally {
            if (client) {
                this.startingClients.delete(client);
                if (!connected) await client.stop();
            }
            if (!connected && !previous && !await stat(join(directory, 'session.json')).catch(() => undefined)) {
                this.saved.delete(saved.id);
                await this.persist();
            }
            this.starting.delete(saved.uri);
        }
    }

    private async begin(action: Start): Promise<void> {
        const document = await this.chooseDocument();
        if (!document) return;
        requireDocument(document);
        const name = await vscode.window.showInputBox({ title, prompt: 'Display name in the room', value: process.env.USER || process.env.USERNAME || '',
            validateInput: value => !value.trim() ? 'Enter a display name.' : undefined });
        if (!name?.trim()) return;
        if (action === 'host') await this.host(document, name.trim());
        else {
            const invitation = await vscode.window.showInputBox({ title, prompt: 'Paste the room invitation', password: true, ignoreFocusOut: true });
            if (!invitation?.trim()) return;
            await this.join(document, invitation.trim(), name.trim());
        }
        await this.view.show();
    }

    private async chooseDocument(): Promise<vscode.TextDocument | undefined> {
        const active = this.activeDocument();
        if (active?.languageId === xamlLanguage && active.uri.scheme === 'file') return active;
        const [uri] = await vscode.window.showOpenDialog({ title: 'Choose the XAML file for this room', filters: { 'FerroUI XAML': ['xaml'] } }) ?? [];
        return uri && vscode.workspace.openTextDocument(uri);
    }

    private async openSaved(): Promise<void> {
        if (!this.saved.size) throw new Error('Host or join a room to create a saved connection.');
        const pick = await choose('Resume FerroUI Room', [...this.saved.values()].map(room => ({ label: room.label,
            description: vscode.workspace.asRelativePath(vscode.Uri.parse(room.uri)), room })));
        if (!pick) return;
        const room = await this.resume(pick.room.id);
        await vscode.window.showTextDocument(room.document, { preserveFocus: true });
        await this.view.show();
    }

    private async invite(room: CollaborationDocument): Promise<void> {
        const role = await choose('Copy Room Invitation', [{ label: 'Editor', role: 'editor' }, { label: 'Viewer', role: 'viewer' }] as const);
        if (!role) return;
        await this.copyInvitation(room, role.role);
        void vscode.window.showInformationMessage(`${role.label} invitation copied.`);
    }

    private async member(item: RoomItem | undefined): Promise<void> {
        const room = this.target(item);
        const member = item?.member ?? (await choose('Manage Room Members', room.current.members.filter(entry => entry.role !== 'owner')
            .map(entry => ({ label: entry.display_name, description: entry.role, entry }))))?.entry;
        if (!member || member.role === 'owner') return;
        const [keep, other] = member.role === 'viewer' ? ['viewer', 'editor'] as const : ['editor', 'viewer'] as const;
        const choice = await choose(`Manage ${member.display_name}`, [
            { label: `Set ${other} role`, role: other, revoked: false },
            { label: member.revoked ? 'Restore access' : 'Revoke access', role: keep, revoked: !member.revoked },
        ]);
        if (choice) await room.member(member.actor, choice.role, choice.revoked);
    }

    private async checkpoint(room: CollaborationDocument): Promise<void> {
        const snapshot = await room.read();
        const choice = await vscode.window.showInformationMessage('Create a new collaboration baseline for this room?', { modal: true }, 'Checkpoint');
        if (choice) await room.source('checkpoint', snapshot.editor_version, snapshot.revision, () => {});
    }

    private target(item?: RoomItem): CollaborationDocument {
        if (item?.room && this.rooms.get(item.room.saved.id) === item.room) return item.room;
        const document = this.activeDocument();
        const room = [...this.rooms.values()].find(current => current.document === document) ?? (this.rooms.size === 1 ? this.rooms.values().next().value : undefined);
        if (!room) throw new Error('Select a room in the FerroUI Collaboration view.');
        return room;
    }

    private command(id: string, action: (item?: RoomItem) => Promise<void>): vscode.Disposable {
        return vscode.commands.registerCommand(`ferroui.collaboration.${id}`, async (item?: RoomItem) => {
            try {
                if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before connecting to a collaboration room.');
                await action(item);
            } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        });
    }

    private async remember(room: SavedRoom): Promise<void> { this.saved.set(room.id, room); await this.persist(); }
    private async persist(): Promise<void> { await this.context.workspaceState.update(roomKey, [...this.saved.values()]); }

    async shutdown(): Promise<void> {
        this.disposed = true;
        await Promise.all([...this.rooms.values()].map(room => this.disconnect(room)).concat([...this.startingClients].map(client => client.stop())));
    }

    dispose(): void {
        void this.shutdown();
        this.commands.forEach(command => command.dispose());
        this.reviews.dispose();
        this.view.dispose();
    }
}

function requireDocument(document: vscode.TextDocument): void {
    if (!vscode.workspace.isTrusted || document.isClosed || document.uri.scheme !== 'file' || document.languageId !== xamlLanguage) {
        throw new Error('Open a local FerroUI XAML file in a trusted workspace to share its source.');
    }
}
