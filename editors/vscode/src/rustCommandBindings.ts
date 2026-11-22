import * as vscode from 'vscode';
import type { Range } from './protocol';
import { choose, editText, type StudioContext } from './studioContext';

interface CommandValue {
    property: string;
    typeName: string;
    editable: boolean;
    authored: string | null;
    range: Range | null;
}

interface CommandSlot {
    property: string;
    binding: CommandValue;
    parameter: CommandValue | null;
}

type Selection = { kind: 'keep' | 'clear' } | { kind: 'set'; expression: string };

const title = 'FerroUI Command Binding';
const keep: Selection = { kind: 'keep' };

export async function openRustCommandBinding(context: StudioContext, node: number): Promise<void> {
    const model = await context.read<{ slots: CommandSlot[]; model: string }>('designer_rsx_commands', { node });
    if (!model.slots.length) {
        void vscode.window.showInformationMessage('This control has no mapped Rust command property.');
        return;
    }
    const slot = model.slots.length === 1 ? model.slots[0] : (await choose(title, model.slots.map(slot => ({
        label: slot.property, description: slot.binding.typeName, slot,
    }))))?.slot;
    if (!slot) return;
    const command = await pick(context, slot.binding);
    if (!command) return;
    const parameter = slot.parameter ? await pick(context, slot.parameter) : keep;
    if (!parameter || command.kind === 'keep' && parameter.kind === 'keep') return;
    await context.edit('designer_rsx_command_edit', {
        node, expected_model: model.model, change: { property: slot.property, command, parameter },
    });
}

async function pick(context: StudioContext, value: CommandValue): Promise<Selection | undefined> {
    type Item = vscode.QuickPickItem & { action: 'keep' | 'set' | 'clear' | 'reveal' };
    context.ensure();
    const caption = `${title} · ${value.property}`;
    const items: Item[] = [{
        label: '$(check) Keep current value', action: 'keep',
        description: value.authored?.replace(/\s+/g, ' ').slice(0, 120) ?? 'No authored expression',
    }];
    if (value.editable) {
        items.push({ label: '$(edit) Set Rust expression…', action: 'set', detail: value.typeName });
        if (value.authored !== null) items.push({ label: '$(clear-all) Clear expression', action: 'clear' });
    }
    if (value.range) items.push({ label: '$(go-to-file) Reveal in source', action: 'reveal' });
    const choice = await choose(caption, items);
    if (!choice) return;
    context.ensure();
    if (choice.action === 'reveal' && value.range) {
        await context.revealRustRange(value.range);
    } else if (choice.action === 'set') {
        const initial = value.authored && !/[\r\n]/.test(value.authored) ? value.authored : undefined;
        const expression = await editText(`Rust expression for ${value.typeName}`, initial,
            input => input.trim() ? undefined : 'Enter a Rust expression.', caption);
        if (expression !== undefined) return { kind: 'set', expression };
    } else if (choice.action === 'clear' || choice.action === 'keep') {
        return { kind: choice.action };
    }
}
