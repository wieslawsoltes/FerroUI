import { strict as assert } from 'node:assert';
import { link, mkdtemp, rm } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import * as vscode from 'vscode';

export async function verifyRsxRecovery(workspace: vscode.Uri): Promise<void> {
    const settings = vscode.workspace.getConfiguration('ferroui.preview');
    const helper = settings.get<string>('path');
    assert.ok(helper, 'Use the native helper configured by the extension-host runner');
    const previous = settings.inspect<string>('path')?.globalValue;
    const directory = await mkdtemp(join(dirname(helper), 'ferroui-rsx-recovery-'));
    const replacement = join(directory, basename(helper));
    try {
        await settings.update('path', replacement, vscode.ConfigurationTarget.Global);
        const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace, 'views.rs'));
        const version = document.version;
        const field = document.positionAt(document.getText().indexOf('padding: 36.0') + 3);
        const hasPadding = async () => {
            const results = await vscode.commands.executeCommand<vscode.CompletionList>(
                'vscode.executeCompletionItemProvider', document.uri, field,
            );
            return results?.items.some(item => item.label === 'padding' && item.detail?.includes('Border.Padding')) ?? false;
        };
        assert.equal(await hasPadding(), false, 'The configured helper is temporarily unavailable');
        // Restore the executable in place, without changing the setting or copying the binary.
        await link(helper, replacement);
        assert.equal(await hasPadding(), true, 'A new request retries analysis on the same Rust buffer version');
        const hovers = await vscode.commands.executeCommand<vscode.Hover[]>('vscode.executeHoverProvider', document.uri, field);
        assert.ok(hovers?.some(hover => hover.contents.some(content =>
            (typeof content === 'string' ? content : content.value).includes('Border.Padding'))));
        assert.equal(document.version, version);
        console.log('VS Code retried RSX completion and hover without editing the Rust buffer');
    } finally {
        try { await settings.update('path', previous, vscode.ConfigurationTarget.Global); }
        finally { await rm(directory, { recursive: true, force: true, maxRetries: 3 }); }
    }
}
