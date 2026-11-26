import * as path from 'node:path';
import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import { WorkspaceTools } from './workspaceTools';
import { localFolders } from './workspacePaths';

type ScaffoldLanguage = 'xaml' | 'rsx';
type ScaffoldKind = 'view' | 'component';

export class ProjectScaffold implements vscode.Disposable {
    private readonly files = new WorkspaceTools();
    private readonly commands = [
        vscode.commands.registerCommand('ferroui.project.newView', () => this.choose('view')),
        vscode.commands.registerCommand('ferroui.project.newComponent', () => this.choose('component')),
    ];

    async create(kind: ScaffoldKind, language: ScaffoldLanguage, destination: string, folder: string | undefined, signal: AbortSignal) {
        if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before creating a FerroUI view.');
        const extension = language === 'xaml' ? '.xaml' : '.rs';
        const provided = path.extname(destination);
        if (provided && provided.toLowerCase() !== extension) throw new Error(`Choose a ${extension} destination.`);
        const name = path.basename(destination, provided);
        if (!/^[A-Za-z][A-Za-z0-9_]*$/.test(name)) throw new Error('Start the file name with a letter and use letters, digits or underscores.');
        return this.files.call('ferroui_workspace_create', { path: provided ? destination : destination + extension, folder, open: true,
            source: scaffold(kind, language, name) }, signal);
    }

    private async choose(kind: ScaffoldKind): Promise<void> {
        const folders = localFolders();
        const folder = folders.length === 1 ? folders[0] : await vscode.window.showWorkspaceFolderPick({ placeHolder: 'Select a FerroUI workspace folder' });
        if (!folder) return;
        const language = await vscode.window.showQuickPick(['XAML', 'RSX'] as const, { title: `New FerroUI ${kind}` });
        if (!language) return;
        const destination = await vscode.window.showInputBox({
            title: `New FerroUI ${language} ${kind}`, prompt: 'Enter a path relative to the selected workspace folder',
            placeHolder: language === 'XAML' ? 'Views/OrdersView.xaml' : 'src/orders_view.rs',
        });
        if (!destination) return;
        try { await this.create(kind, language === 'XAML' ? 'xaml' : 'rsx', destination, folder.uri.toString(), AbortSignal.timeout(15_000)); }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    dispose(): void { this.commands.forEach(command => command.dispose()); }
}

function scaffold(kind: ScaffoldKind, language: ScaffoldLanguage, name: string): string {
    const words = name.replace(/([a-z0-9])([A-Z])/g, '$1 $2').split(/[ _]+/).filter(Boolean);
    const heading = words.map(word => word[0].toUpperCase() + word.slice(1)).join(' ');
    if (language === 'xaml') return `<UserControl xmlns="https://github.com/ferroui">\n    <StackPanel Spacing="12">\n        <TextBlock Text="${heading}"/>\n    </StackPanel>\n</UserControl>\n`;
    const functionName = kind === 'view' ? `${words.join('_').toLowerCase()}_element` : heading.replaceAll(' ', '');
    const component = kind === 'component' ? '#[component(rerender)]\n' : '';
    return `use ferroui_view::prelude::*;\n\n${component}pub(crate) fn ${functionName}() -> Element {\n    rsx! {\n        StackPanel { spacing: 12.0,\n            TextBlock { text: "${heading}" }\n        }\n    }\n}\n`;
}
