import * as path from 'node:path';
import * as vscode from 'vscode';
import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { HostedAuthoredPlan, HostedIncludeRevision, HostedSourceLocation, HostedSources } from './hostedSources';

interface HostedTemplate {
    owner: number;
    label: string;
    typeName: string;
    definition: HostedSourceLocation & { address: string; node: number };
    canCopy: boolean;
}

export interface HostedTemplateCatalog { templates: HostedTemplate[] }
export interface HostedTemplatePlan extends HostedAuthoredPlan { includes: HostedIncludeRevision[] }
export type NativeTemplateRequest = { action: 'catalog' } | { action: 'planCopy'; owner: number; source: string };

export class HostedTemplates {
    constructor(private readonly sources: HostedSources) {}

    catalog(frame: HostedAnimationFrame, signal: AbortSignal) {
        return this.sources.read(frame, signal, client => client.hostedTemplates<HostedTemplateCatalog>(frame, { action: 'catalog' }));
    }

    async locate(frame: HostedAnimationFrame, owner: number, signal: AbortSignal) {
        const { templates } = await this.catalog(frame, signal);
        const source = templates.find(entry => entry.owner === owner)?.definition;
        if (!source) throw new Error('Choose a control template from the current App Preview.');
        return this.sources.locate(frame, source.address, source.range, signal);
    }

    copy(frame: HostedAnimationFrame, owner: number, apply: boolean, signal: AbortSignal) {
        return this.sources.edit(frame, undefined, apply, signal, async (client, source) => {
            const plan = await client.hostedTemplates<HostedTemplatePlan>(frame, { action: 'planCopy', owner, source });
            await this.sources.verifyIncludes(plan.includes);
            return plan;
        });
    }

    async choose(frame: HostedAnimationFrame, currentFrame: () => HostedAnimationFrame, signal: AbortSignal) {
        const { templates } = await this.catalog(frame, signal);
        if (!templates.length) {
            void vscode.window.showInformationMessage('The loaded application view has no unambiguous templates in its workspace XAML.');
            return;
        }
        const control = await vscode.window.showQuickPick(templates.map(template => ({
            label: template.label,
            description: template.typeName === template.label ? undefined : template.typeName,
            detail: `${path.basename(vscode.Uri.parse(template.definition.uri).fsPath)} · line ${template.definition.range.start.line + 1}`,
            template,
        })), { title: 'App Preview Templates', placeHolder: 'Choose a control', matchOnDescription: true, matchOnDetail: true });
        if (!control) return;
        const actions = [{ label: '$(go-to-file) Open Template Source', copy: false }];
        if (control.template.canCopy) actions.push({ label: '$(copy) Make Inline Copy', copy: true });
        const action = await vscode.window.showQuickPick(actions, { title: control.label, placeHolder: 'Choose a template action' });
        if (!action) return;
        const current = currentFrame();
        if (current.revision !== frame.revision) throw new Error('App Preview source changed. Open Templates again.');
        if (action.copy) await this.copy(current, control.template.owner, true, signal);
        else await this.locate(current, control.template.owner, signal);
    }
}
