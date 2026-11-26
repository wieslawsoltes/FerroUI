import * as vscode from 'vscode';
import { showPropertyTrace, type PropertyTrace } from './propertyTrace';
import type { RuntimeClient } from './runtimeClient';
import { readRuntimeDiagnostics } from './runtimeDiagnostics';
import type { RuntimeInspectionResult, RuntimeNode, RuntimeSelection, RuntimeSource } from './runtimeProtocol';
import type { RuntimeSourceEditor } from './runtimeSource';
import type { AppRuntime } from './runtimeTargets';
import { readRuntimeTemplates, runtimeTemplateLabel } from './runtimeTemplates';

const tools = { properties: 'get_properties', styles: 'get_styles', resources: 'get_resources' } as const;

export async function readRuntimeInspection(client: RuntimeClient, sourceEditor: RuntimeSourceEditor,
    runtime: AppRuntime, selection: RuntimeSelection, signal?: AbortSignal): Promise<RuntimeInspectionResult> {
    const { node, view } = selection;
    const [data, source, canTrace, canInspectTemplates] = await Promise.all([
        view === 'diagnostics' ? readRuntimeDiagnostics(client, signal) : client.call(tools[view], { node_id: node }, signal),
        sourceEditor.read(client, node, signal),
        view === 'properties' && client.supports('get_property_trace'),
        client.supports('get_template_sources'),
    ]);
    return { ...selection, data, canReveal: Boolean(source),
        canApply: Boolean(view === 'properties' && source && sourceEditor.canApply(source, runtime)), canTrace, canInspectTemplates };
}

export async function showLiveTemplates(client: RuntimeClient, sourceEditor: RuntimeSourceEditor,
    node: RuntimeNode, current: () => boolean): Promise<void> {
    const templates = await readRuntimeTemplates(client, node.node_id);
    if (!current()) return;
    const mapped = templates.filter(template => template.source);
    if (!mapped.length) {
        void vscode.window.showInformationMessage('The app has no source-backed templates for this element.');
        return;
    }
    const choice = await vscode.window.showQuickPick(mapped.map(template => ({
        label: runtimeTemplateLabel(template),
        description: template.column ? template.column.header ?? `Column ${template.column.index + 1}` : node.name || node.type,
        template,
    })), { title: 'Live App Templates', placeHolder: 'Choose an effective template to open its source', matchOnDescription: true });
    if (choice && current()) {
        await sourceEditor.revealTemplate(client, node.node_id, choice.template.id, AbortSignal.timeout(15_000), current);
    }
}

export async function showLivePropertyTrace(client: RuntimeClient, sourceEditor: RuntimeSourceEditor,
    nodeId: number, property: string, current: () => boolean): Promise<void> {
    if (!await client.supports('get_property_trace')) throw new Error('This app does not expose value traces.');
    const trace = await client.call<PropertyTrace<RuntimeSource>>('get_property_trace', { node_id: nodeId, property });
    if (!current()) return;
    await showPropertyTrace(trace, async anchor => {
        const source = await sourceEditor.resolve(anchor);
        if (!current()) return;
        if (!source) throw new Error('The contributing source is unavailable in the current workspace or revision.');
        const { document, position } = source;
        await vscode.window.showTextDocument(document, {
            selection: new vscode.Range(position, position), viewColumn: vscode.ViewColumn.Beside,
        });
    });
}
