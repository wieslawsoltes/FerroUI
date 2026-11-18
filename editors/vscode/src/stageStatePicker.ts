import * as vscode from 'vscode';
import type { SourceAnchor, StageState, StageStateOptions } from './stagesProtocol';
import { choose } from './studioContext';

export async function chooseStageState(groups: StageStateOptions[], selected: StageState[]): Promise<StageState[] | undefined> {
    if (!groups.length && !selected.length) {
        void vscode.window.showInformationMessage('This stage has no authored visual states.');
        return;
    }
    const items: (vscode.QuickPickItem & { option: StageStateOptions | null })[] = groups.map(option => ({
        label: option.group, description: option.owner_name, option,
    }));
    if (selected.length) items.push({ label: 'Clear all state choices', description: 'Restore authored appearance', option: null });
    const group = await choose('Visual state group', items);
    if (!group) return;
    if (!group.option) return [];
    const { owner, group: name, states } = group.option;
    const matches = (selection: StageState) => selection.group === name && sameOwner(selection.owner, owner);
    const active = selected.find(matches)?.state ?? null;
    const choice = await choose(`State for ${name}`, [
        { label: 'Authored appearance', description: active === null ? 'Current state' : undefined, value: null },
        ...states.map(state => ({ label: state, description: active === state ? 'Current state' : undefined, value: state })),
    ]);
    if (!choice) return;
    const next = selected.filter(selection => !matches(selection));
    if (choice.value !== null) next.push({ owner, group: name, state: choice.value });
    return next;
}

function sameOwner(left: SourceAnchor, right: SourceAnchor): boolean {
    return left.path.length === right.path.length && left.path.every((segment, index) => {
        const other = right.path[index];
        return segment.namespace === other.namespace && segment.type_name === other.type_name
            && segment.name === other.name && segment.key === other.key;
    });
}
