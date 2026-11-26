import * as vscode from 'vscode';
import type { Position, Range, TextEdit } from './protocol';
import { choose } from './studioContext';
import { applyTextEdits } from './textDocument';
import type { XamlServer } from './xamlServer';

export type CommandSelection = { kind: 'keep' | 'clear' } | { kind: 'bind'; path: string };
export interface CommandBindingEdit {
    property: string;
    command: CommandSelection;
    parameter: CommandSelection;
}

interface CommandValue { authored: string | null; range: Range | null; editable: boolean }
interface CommandMember { path: string; typeName: string; command: boolean; children: boolean }
interface CommandSlot { property: string; binding: CommandValue; parameter: CommandValue | null }
interface CommandModel { slots: CommandSlot[]; members: CommandMember[] }
interface CommandPlan { documentChanges: { textDocument: { uri: string; version: number }; edits: TextEdit[] }[] }
export interface CommandSite {
    document: vscode.TextDocument;
    version: number;
    position: Position;
    validate(): void;
    reveal(range: Range): Promise<void>;
}

const keep: CommandSelection = { kind: 'keep' };
const clear: CommandSelection = { kind: 'clear' };
const title = 'FerroUI Command Binding';

export class CommandBindings {
    constructor(private readonly server: XamlServer) {}

    async inspect(site: CommandSite, prefix = '', signal = AbortSignal.timeout(15_000)) {
        this.check(site);
        const model = await this.server.commandBindingRequest<CommandModel>('commandBindings', this.arguments(site, prefix));
        signal.throwIfAborted();
        this.check(site);
        return { ...model, uri: site.document.uri.toString(), version: site.version };
    }

    async apply(site: CommandSite, change: CommandBindingEdit, signal = AbortSignal.timeout(15_000)): Promise<void> {
        this.check(site);
        const plan = await this.server.commandBindingRequest<CommandPlan>('planCommandBinding', {
            ...this.arguments(site), change,
        });
        signal.throwIfAborted();
        this.check(site);
        const [edit, ...other] = plan.documentChanges;
        if (!edit) return;
        if (other.length || edit.textDocument.version !== site.version
            || vscode.Uri.parse(edit.textDocument.uri).toString() !== site.document.uri.toString()) {
            throw new Error('The command binding edit targets a different source version.');
        }
        await applyTextEdits(site.document, site.version, edit.edits);
    }

    async open(site: CommandSite): Promise<void> {
        const model = await this.inspect(site);
        if (!model.slots.length) {
            void vscode.window.showInformationMessage('This control has no registered command property.');
            return;
        }
        const slot = model.slots.length === 1 ? model.slots[0] : (await choose(title, model.slots.map(slot => ({
            label: slot.property, description: slot.binding.authored ?? undefined, slot,
        }))))?.slot;
        if (!slot) return;
        const command = await this.pick(site, slot.property, slot.binding, model.members, true);
        if (!command) return;
        const parameter = slot.parameter
            ? await this.pick(site, `${slot.property}Parameter`, slot.parameter, model.members, false) : keep;
        if (parameter) await this.apply(site, { property: slot.property, command, parameter });
    }

    private async pick(site: CommandSite, property: string, value: CommandValue, root: CommandMember[], commands: boolean): Promise<CommandSelection | undefined> {
        type Item = vscode.QuickPickItem & { selection?: CommandSelection; prefix?: string; reveal?: Range };
        let prefix = '';
        for (;;) {
            this.check(site);
            const items: Item[] = prefix
                ? [{ label: '$(arrow-left) Back', prefix: prefix.split('.').slice(0, -1).join('.') }]
                : [{ label: '$(check) Keep current value', description: value.authored ?? 'No authored value', selection: keep }];
            if (!prefix && value.range) {
                if (value.editable) items.push({ label: '$(clear-all) Clear binding', selection: clear });
                items.push({ label: '$(go-to-file) Reveal in source', reveal: value.range });
            }
            if (value.editable) {
                const members = prefix ? (await this.inspect(site, prefix)).members : root;
                for (const member of members) {
                    if (!commands || member.command) {
                        items.push({ label: member.path, description: member.typeName, selection: { kind: 'bind', path: member.path } });
                    }
                    if (member.children) items.push({ label: `$(chevron-right) ${member.path}`, description: 'Browse members', prefix: member.path });
                }
            }
            const selection = await choose(`${title} · ${property}`, items);
            if (!selection) return;
            this.check(site);
            if (selection.reveal) {
                await site.reveal(selection.reveal);
                return;
            }
            if (selection.selection) return selection.selection;
            prefix = selection.prefix ?? '';
        }
    }

    private arguments(site: CommandSite, prefix?: string) {
        return { uri: site.document.uri.toString(), request: { version: site.version, position: site.position, prefix } };
    }

    private check(site: CommandSite): void {
        site.validate();
        if (site.document.isClosed || site.document.version !== site.version) {
            throw new Error('The XAML source changed. Reopen the command binding picker.');
        }
    }
}
