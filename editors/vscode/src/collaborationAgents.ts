import * as vscode from 'vscode';
import { choice, integer, tool, truth } from './agentSchema';
import type { Collaboration } from './collaboration';
import type { SourceAction } from './collaborationDocument';
import type { GuestRole } from './collaborationProtocol';
import { resolveWorkspaceFile } from './workspacePaths';

const title = { type: 'string', maxLength: 256 };
const path = { type: 'string', maxLength: 4096 };
const session = { session: { type: 'string', maxLength: 36 } };
const role = choice('editor', 'viewer');
const document = { path, folder: path, display_name: title };
const roomTool: typeof tool = (...input) => {
    const definition = tool(...input);
    return { ...definition, annotations: { ...definition.annotations, openWorldHint: true } };
};

export const collaborationCatalog = {
    ferroui_rooms: tool('List active and saved FerroUI XAML rooms, roles, presence, and editor revisions.'),
    ferroui_room_host: roomTool('Host the selected trusted workspace XAML buffer in a FerroUI room.', document, ['path', 'display_name'], true),
    ferroui_room_join: roomTool('Join a FerroUI room using a user-provided invitation. The local source remains available for review.',
        { ...document, invitation: { type: 'string', maxLength: 16384 } }, ['path', 'display_name', 'invitation'], true),
    ferroui_room_resume: roomTool('Resume a saved FerroUI room and its durable outbox.', session, ['session'], true),
    ferroui_room_disconnect: tool('Disconnect from a FerroUI room and retain its saved connection.', session, ['session'], true),
    ferroui_room_sync: roomTool('Synchronize a room and its queued changes with the room service.', session, ['session'], true, false),
    ferroui_room_review: tool('Read the current room source and the local editor version for a guarded review.', session, ['session']),
    ferroui_room_source: roomTool('Apply reviewed room source or publish a reviewed editor draft. Supply both versions from ferroui_room_review.', {
        ...session, expected_version: integer(1), revision: { type: 'string', maxLength: 64 },
        action: choice('apply', 'publish', 'publish_resolution', 'checkpoint', 'checkpoint_resolution'),
    }, ['session', 'expected_version', 'revision', 'action'], true),
    ferroui_room_recovery: tool('Read the saved source backup and its destination editor version.', session, ['session']),
    ferroui_room_recover: tool('Restore the reviewed source backup into the local editor as one Undo step.', {
        ...session, expected_version: integer(1), backup: { type: 'string', maxLength: 64 },
    }, ['session', 'expected_version', 'backup'], true),
    ferroui_room_invite: roomTool('Copy an editor or viewer invitation to the user’s clipboard.', { ...session, role }, ['session', 'role'], true, false),
    ferroui_room_member: roomTool('Change a participant role or revoke or restore their access to this room.', {
        ...session, actor: title, role, revoked: truth,
    }, ['session', 'actor', 'role', 'revoked'], true),
};

export type CollaborationTool = keyof typeof collaborationCatalog;

export function isCollaborationTool(tool: PropertyKey): tool is CollaborationTool { return Object.hasOwn(collaborationCatalog, tool); }

interface Input {
    session: string;
    path: string;
    folder?: string;
    display_name: string;
    invitation: string;
    actor: string;
    role: GuestRole;
    revoked: boolean;
    action: SourceAction;
    expected_version: number;
    revision: string;
    backup: string;
}

export async function collaborationTool(collaboration: Collaboration, tool: CollaborationTool, args: unknown, signal: AbortSignal): Promise<unknown> {
    const input = args as Input;
    if (tool === 'ferroui_rooms') return collaboration.list();
    if (tool === 'ferroui_room_host' || tool === 'ferroui_room_join') {
        const document = await vscode.workspace.openTextDocument(await resolveWorkspaceFile(input.path, input.folder));
        signal.throwIfAborted();
        const room = tool === 'ferroui_room_host' ? await collaboration.host(document, input.display_name)
            : await collaboration.join(document, input.invitation, input.display_name);
        return { session: room.saved.id, ...room.current };
    }
    if (tool === 'ferroui_room_resume') {
        const room = await collaboration.resume(input.session);
        return { session: room.saved.id, ...room.current };
    }
    const room = collaboration.find(input.session);
    switch (tool) {
        case 'ferroui_room_disconnect':
            await collaboration.disconnect(room);
            return { disconnected: true };
        case 'ferroui_room_sync': return room.synchronize(signal);
        case 'ferroui_room_review': return room.read(signal);
        case 'ferroui_room_source': return room.source(input.action, input.expected_version, input.revision, () => {}, signal);
        case 'ferroui_room_recovery': return room.readRecovery(signal);
        case 'ferroui_room_recover': return room.applyRecovery(input.expected_version, input.backup, signal);
        case 'ferroui_room_invite':
            await collaboration.copyInvitation(room, input.role, signal);
            return { copied: true, role: input.role };
        case 'ferroui_room_member': return room.member(input.actor, input.role, input.revoked, signal);
    }
}
