import * as vscode from 'vscode';
import { managedProcessVariable } from './reloadDebugger';

interface DebugSelection { interactive?: boolean; name?: string; exactPid?: boolean }

export async function selectDebugConfiguration(folder: vscode.WorkspaceFolder,
    { interactive = true, name, exactPid = false }: DebugSelection = {}): Promise<vscode.DebugConfiguration | undefined> {
    const configurations = vscode.workspace.getConfiguration('launch', folder.uri)
        .get<vscode.DebugConfiguration[]>('configurations', [])
        .filter(configuration => configuration?.request === 'attach' && (exactPid
            ? configuration.pid === managedProcessVariable || configuration.processId === managedProcessVariable
            : JSON.stringify(configuration).includes(managedProcessVariable)));
    if (!configurations.length) throw new Error(`Add a Rust attach configuration with ${managedProcessVariable} as its process ID.`);
    if (name !== undefined) {
        const matches = configurations.filter(configuration => configuration.name === name);
        if (matches.length !== 1) throw new Error('Choose one attach configuration by its name in launch.json.');
        return matches[0];
    }
    if (configurations.length === 1) return configurations[0];
    if (!interactive) throw new Error('Specify debug_configuration using the attach configuration name in launch.json.');
    return (await vscode.window.showQuickPick(configurations.map(configuration => ({ label: configuration.name, configuration })),
        { placeHolder: 'Debug this FerroUI application session' }))?.configuration;
}
