import { constants } from 'node:fs';
import { access } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';

type Tool = 'languageServer' | 'preview';
const binaries: Record<Tool, string> = {
    languageServer: 'ferroui-xaml-lsp',
    preview: 'ferroui-designer-preview',
};

export async function commandFor(tool: Tool, context: Pick<vscode.ExtensionContext, 'extensionPath'>): Promise<string> {
    const config = `ferroui.${tool}.path`;
    const configured = vscode.workspace.getConfiguration().get<string>(config, '').trim();
    if (configured) {
        if (!path.isAbsolute(configured)) {
            throw new Error(`Set ${config} to an absolute executable path.`);
        }
        await access(configured, constants.X_OK);
        return configured;
    }
    const name = binaries[tool] + (process.platform === 'win32' ? '.exe' : '');
    const bundled = path.join(context.extensionPath, 'bin', name);
    try {
        await access(bundled, constants.X_OK);
        return bundled;
    } catch {
        return name;
    }
}
