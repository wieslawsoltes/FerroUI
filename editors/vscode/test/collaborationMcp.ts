import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RoomSourceReview, SourceAction } from '../src/collaborationDocument';
import type { RoomStatus } from '../src/collaborationProtocol';
import { runEditorHistory } from '../src/editorHistory';
import type { McpFixture } from './mcp';
import { until } from './wait';

const ownerSource = '<Border xmlns="https://github.com/ferroui">\r\n    <Button Content="Review ready · 🪴" />\r\n</Border>\r\n';
const localSource = '<Border xmlns="https://github.com/ferroui">\n    <TextBlock Text="Local draft · 🪴" />\n</Border>\n';
type Session = RoomStatus & { session: string };

export async function verifyCollaboration({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const ownerPath = 'RoomOwner.xaml';
    const guestPath = 'RoomReviewer.xaml';
    const create = async (path: string, source: string) => {
        const uri = vscode.Uri.joinPath(root, path);
        await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
        const document = await vscode.workspace.openTextDocument(uri);
        assert.equal(document.getText(), source);
        return document;
    };
    const ownerDoc = await create(ownerPath, ownerSource);
    const guestDoc = await create(guestPath, localSource);
    const sessions: string[] = [];
    const review = async (session: string) => content<RoomSourceReview>(await invoke('ferroui_room_review', { session }));
    const sync = async (session: string) => content<RoomStatus>(await invoke('ferroui_room_sync', { session }));
    const apply = (session: string, source: RoomSourceReview, action: SourceAction) => invoke('ferroui_room_source',
        { session, expected_version: source.editor_version, revision: source.revision, action });
    const replace = async (document: vscode.TextDocument, search: string, requested: string) => {
        const offset = document.getText().indexOf(search);
        assert.ok(offset >= 0);
        const edit = new vscode.WorkspaceEdit();
        edit.replace(document.uri, new vscode.Range(document.positionAt(offset), document.positionAt(offset + search.length)), requested);
        assert.equal(await vscode.workspace.applyEdit(edit), true);
    };
    try {
        const owner = content<Session>(await invoke('ferroui_room_host', { path: ownerPath, display_name: 'Maya Chen' }));
        sessions.push(owner.session);
        let invitation = '';
        const clipboard = await vscode.env.clipboard.readText();
        try {
            const invite = content<{ copied: boolean }>(await invoke('ferroui_room_invite', { session: owner.session, role: 'viewer' }));
            assert.ok(invite.copied);
            invitation = await vscode.env.clipboard.readText();
        } finally { await vscode.env.clipboard.writeText(clipboard); }
        const guest = content<Session>(await invoke('ferroui_room_join', { path: guestPath, display_name: 'Sam Lee', invitation }));
        sessions.push(guest.session);
        assert.equal(guest.role, 'viewer');
        assert.equal(guestDoc.getText(), localSource);
        const initial = await review(guest.session);
        assert.equal(initial.source, ownerSource);
        assert.equal(initial.can_publish, false);
        assert.ok((await apply(guest.session, initial, 'publish_resolution')).isError);

        await vscode.window.showTextDocument(guestDoc);
        await vscode.commands.executeCommand('ferroui.collaboration.review');
        await until(() => vscode.window.tabGroups.activeTabGroup.activeTab?.label.startsWith('Room Source · RoomReviewer.xaml') === true);
        await vscode.commands.executeCommand('ferroui.collaboration.incoming.apply');
        assert.equal(guestDoc.getText(), ownerSource.replaceAll('\r\n', '\n'));
        assert.equal(guestDoc.eol, vscode.EndOfLine.LF);
        const version = guestDoc.version;
        await runEditorHistory(guestDoc, 'undo', () => assert.equal(guestDoc.version, version), AbortSignal.timeout(6_000));
        assert.equal(guestDoc.getText(), localSource, 'One Undo restores the original text and line endings');

        const retired = await review(guest.session);
        await replace(guestDoc, 'Local draft', 'Draft typed during review');
        assert.match((await apply(guest.session, retired, 'apply')).content[0]?.text ?? '', /local editor changed/);
        const local = guestDoc.getText();
        const staleRoom = await review(guest.session);
        await replace(ownerDoc, 'Review ready', 'Owner update');
        await sync(owner.session);
        content(await apply(owner.session, await review(owner.session), 'publish'));
        await sync(owner.session);
        assert.ok((await apply(guest.session, staleRoom, 'apply')).isError);
        assert.equal(guestDoc.getText(), local);
        await sync(guest.session);
        content(await apply(guest.session, await review(guest.session), 'apply'));
        assert.equal(guestDoc.getText(), ownerDoc.getText().replaceAll('\r\n', '\n'));
        const backup = content<{ source: string }>(await invoke('ferroui_room_recovery', { session: guest.session }));
        assert.equal(backup.source, local);

        const members = await sync(owner.session);
        assert.ok(members.members.some(member => member.actor === guest.actor && member.role === 'viewer'));
        assert.ok(members.online.includes(guest.actor));
        content(await invoke('ferroui_room_member', { session: owner.session, actor: guest.actor, role: 'editor', revoked: false }));
        assert.equal((await sync(guest.session)).role, 'editor');
        await replace(guestDoc, 'Owner update', 'Editor update');
        content(await apply(guest.session, await review(guest.session), 'publish'));
        await sync(guest.session);
        await sync(owner.session);
        const reviewed = await review(owner.session);
        assert.equal(reviewed.source, guestDoc.getText().replaceAll('\n', '\r\n'));
        content(await apply(owner.session, reviewed, 'checkpoint'));
        await sync(guest.session);
        const baseline = await review(guest.session);
        assert.equal(baseline.rebase, true);
        content(await apply(guest.session, baseline, 'apply'));

        content(await invoke('ferroui_room_disconnect', { session: guest.session }));
        const resumed = content<Session>(await invoke('ferroui_room_resume', { session: guest.session }));
        assert.equal(resumed.actor, guest.actor);
        console.log('VS Code collaboration used native rooms, reviewed CRLF and Unicode, preserved Undo, rejected stale drafts, changed roles, and resumed a checkpoint');
    } finally {
        for (const session of sessions.reverse()) await invoke('ferroui_room_disconnect', { session });
    }
}
