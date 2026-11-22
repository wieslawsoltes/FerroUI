import * as vscode from 'vscode';
import type { CollaborationDocument } from './collaborationDocument';
import type { RoomMember, RoomStatus } from './collaborationProtocol';

const phases: Record<RoomStatus['phase'], [string, string]> = {
    connected: ['Connected', 'pass'], synchronizing: ['Synchronizing', 'sync'], reconnecting: ['Reconnecting', 'debug-disconnect'],
    review: ['Review baseline', 'diff'], conflict: ['Conflict', 'warning'], blocked: ['Disconnected', 'debug-disconnect'],
};

export class RoomItem extends vscode.TreeItem {
    constructor(label: string, readonly room: CollaborationDocument, readonly member?: RoomMember) { super(label); }
}

export class CollaborationView implements vscode.Disposable {
    private readonly updated = new vscode.EventEmitter<void>();
    private readonly view: vscode.TreeView<RoomItem>;

    constructor(private readonly rooms: () => readonly CollaborationDocument[]) {
        this.view = vscode.window.createTreeView<RoomItem>('ferroui.collaboration', { showCollapseAll: false, treeDataProvider: {
            onDidChangeTreeData: this.updated.event,
            getChildren: item => item ? this.details(item.room) : this.rooms().map(room => this.roomItem(room)),
            getTreeItem: item => item,
        } });
    }

    refresh(): void {
        const count = this.rooms().length;
        this.view.badge = count ? { value: count, tooltip: `${count} connected or saved room sessions` } : undefined;
        this.updated.fire();
    }

    show(): Thenable<unknown> { return vscode.commands.executeCommand('ferroui.collaboration.focus'); }

    private roomItem(room: CollaborationDocument): RoomItem {
        const status = room.current;
        const [label, icon] = phases[status.phase];
        const item = new RoomItem(status.title, room);
        item.id = room.saved.id;
        item.description = `${status.role} · ${label}`;
        item.tooltip = status.message ?? vscode.workspace.asRelativePath(room.document.uri);
        item.iconPath = new vscode.ThemeIcon(icon);
        item.contextValue = `room-${status.role}`;
        item.collapsibleState = vscode.TreeItemCollapsibleState.Expanded;
        return item;
    }

    private details(room: CollaborationDocument): RoomItem[] {
        const status = room.current;
        const source = new RoomItem('Room source', room);
        source.id = `${room.saved.id}:source`;
        source.description = status.pending ? `${status.pending} queued` : room.needsReview ? 'Review available' : vscode.workspace.asRelativePath(room.document.uri, false);
        source.iconPath = new vscode.ThemeIcon('diff');
        source.command = { command: 'ferroui.collaboration.review', title: 'Review Room Source', arguments: [source] };
        const results = [source];
        if (status.message) {
            const note = new RoomItem(status.message, room);
            note.id = `${room.saved.id}:status`;
            note.iconPath = new vscode.ThemeIcon('info');
            results.push(note);
        }
        const online = new Set(status.online);
        const order = (member: RoomMember) => member.role === 'owner' ? 0 : online.has(member.actor) ? 1 : member.revoked ? 3 : 2;
        const members = [...status.members].sort((first, second) => order(first) - order(second) || first.display_name.localeCompare(second.display_name));
        for (const member of members) {
            const present = online.has(member.actor);
            const row = new RoomItem(member.display_name, room, member);
            row.id = `${room.saved.id}:member:${member.actor}`;
            row.description = `${member.role} · ${member.revoked ? 'revoked' : present ? 'online' : 'offline'}`;
            row.iconPath = new vscode.ThemeIcon(present ? 'account' : 'person');
            if (status.role === 'owner' && member.role !== 'owner') row.contextValue = 'room-participant';
            results.push(row);
        }
        return results;
    }

    dispose(): void { this.view.dispose(); this.updated.dispose(); }
}
