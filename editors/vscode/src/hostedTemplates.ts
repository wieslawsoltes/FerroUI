import * as path from 'node:path';
import * as vscode from 'vscode';
import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { HostedAuthoredPlan, HostedIncludeRevision, HostedSourceLocation, HostedSources } from './hostedSources';

interface HostedTemplate {
    owner: number;
    label: string;
    typeName: string;
    definition: HostedTemplateLocation;
    assignment?: HostedTemplateLocation;
    member?: string;
    canCopy: boolean;
}

interface HostedTemplateLocation extends HostedSourceLocation {
    address: string;
    node?: number;
}

export interface HostedTemplateSelection {
    member?: string;
    destination?: 'definition' | 'assignment' | 'design';
}

export interface HostedTemplateCatalog { templates: HostedTemplate[] }
export interface HostedTemplatePlan extends HostedAuthoredPlan { includes: HostedIncludeRevision[] }
export type NativeTemplateRequest = { action: 'catalog' } | { action: 'planCopy'; owner: number; source: string };

export class HostedTemplates {
    constructor(private readonly sources: HostedSources) {}

    catalog(frame: HostedAnimationFrame, signal: AbortSignal) {
        return this.sources.read(frame, signal, client => client.hostedTemplates<HostedTemplateCatalog>(frame, { action: 'catalog' }));
    }

    async locate(frame: HostedAnimationFrame, owner: number, signal: AbortSignal, selection: HostedTemplateSelection = {}) {
        const { templates } = await this.catalog(frame, signal);
        const matches = templates.filter(entry => entry.owner === owner && (!selection.member || selection.member === entry.member));
        const template = matches.length === 1 ? matches[0] : !selection.member ? matches.find(entry => entry.member === 'Template') : undefined;
        if (!template) throw new Error('Choose an owner and template property from the current App Preview.');
        const source = selection.destination === 'assignment' ? template.assignment : template.definition;
        const design = selection.destination === 'design';
        if (!source || (design && !isRust(template))) throw new Error('This template does not have the requested authored location.');
        const location = await this.sources.locate(frame, source.address, source.range, signal);
        if (design) {
            signal.throwIfAborted();
            await vscode.commands.executeCommand('ferroui.openDesigner', vscode.Uri.parse(location.uri, true));
        }
        return location;
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
            void vscode.window.showInformationMessage('The app has no templates mapped to the current preview source.');
            return;
        }
        const control = await vscode.window.showQuickPick(templates.map(template => ({
            label: template.label,
            description: template.member ? memberLabel(template.member)
                : template.typeName === template.label ? undefined : template.typeName,
            detail: `${path.basename(vscode.Uri.parse(template.definition.uri).fsPath)} · line ${template.definition.range.start.line + 1}`,
            template,
        })), { title: 'App Preview Templates', placeHolder: 'Choose a control or column template', matchOnDescription: true, matchOnDetail: true });
        if (!control) return;
        const actions: Array<{ label: string; action: 'copy' | NonNullable<HostedTemplateSelection['destination']> }> = [
            { label: '$(go-to-file) Open Template Source', action: 'definition' },
        ];
        if (control.template.assignment) actions.push({ label: '$(references) Open Template Assignment', action: 'assignment' });
        if (isRust(control.template)) actions.push({ label: '$(preview) Open Template in Design', action: 'design' });
        if (control.template.canCopy) actions.push({ label: '$(copy) Make Inline Copy', action: 'copy' });
        const action = await vscode.window.showQuickPick(actions, { title: control.label, placeHolder: 'Choose a template action' });
        if (!action) return;
        const current = currentFrame();
        if (current.revision !== frame.revision) throw new Error('App Preview source changed. Open Templates again.');
        if (action.action === 'copy') await this.copy(current, control.template.owner, true, signal);
        else await this.locate(current, control.template.owner, signal, { member: control.template.member, destination: action.action });
    }
}

function isRust(template: HostedTemplate) {
    return path.extname(vscode.Uri.parse(template.definition.uri).path).toLowerCase() === '.rs';
}

function memberLabel(member: string) {
    return member.replace(/[A-Z]/g, (character, offset) => offset ? ` ${character.toLowerCase()}` : character);
}
