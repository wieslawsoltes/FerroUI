import * as vscode from 'vscode';
import { managedProcessVariable } from './reloadDebugger';

export const androidDebugPortVariable = '${command:ferroui.android.debugPort}';

const targets = {
    desktop: (config: vscode.DebugConfiguration) => JSON.stringify(config).includes(managedProcessVariable),
    simulator: (config: vscode.DebugConfiguration) => config.pid === managedProcessVariable || config.processId === managedProcessVariable,
    android: (config: vscode.DebugConfiguration) => config.type === 'lldb-dap' && config['gdb-remote-port'] === androidDebugPortVariable
        && config.pid === undefined && config.attachCommands === undefined && config.coreFile === undefined,
};

interface DebugSelection { interactive?: boolean; name?: string; target?: keyof typeof targets }

export type MobileLaunchDebug = Pick<DebugSelection, 'interactive' | 'name'>;

export async function selectDebugConfiguration(folder: vscode.WorkspaceFolder,
    { interactive = true, name, target = 'desktop' }: DebugSelection = {}): Promise<vscode.DebugConfiguration | undefined> {
    const configurations = vscode.workspace.getConfiguration('launch', folder.uri)
        .get<vscode.DebugConfiguration[]>('configurations', [])
        .filter(configuration => configuration?.request === 'attach' && targets[target](configuration));
    if (!configurations.length) throw new Error(target === 'android'
        ? `Add an LLDB DAP attach configuration with ${androidDebugPortVariable} as its gdb-remote-port.`
        : `Add a Rust attach configuration with ${managedProcessVariable} as its process ID.`);
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
