import * as vscode from 'vscode';
import {
    stageDirectionLabels, stageThemeLabels, type StageComparison, type StageComparisonPlan,
    type StageInfo, type StageMatrix, type StageMatrixSize, type StageModel, type StageStateOptions,
} from './stagesProtocol';
import { dimensions } from './stageEnvironmentPicker';
import { sameOwner } from './stageStatePicker';
import { choose, editText } from './studioContext';

type Review = (comparison: StageComparison) => Promise<{ epoch: number; comparison: StageComparison } | undefined>;

export async function chooseStageComparison(stage: StageInfo, model: StageModel,
    groups: () => Promise<StageStateOptions[]>, plan: (comparison: StageComparison) => Promise<StageComparisonPlan>) {
    const name = stage.configuration.name;
    const choice = await choose(`Compare variants · ${name}`, [
        { label: '$(device-mobile) Phone, tablet and desktop', preset: 'sizes' },
        { label: '$(color-mode) Light and dark themes', preset: 'themes' },
        { label: '$(layout) Portrait and landscape', preset: 'orientations' },
        { label: '$(arrow-swap) Reading directions', preset: 'directions' },
        { label: '$(table) Compare matrix…', preset: 'matrix' },
    ] as const);
    if (!choice) return;
    const review: Review = async comparison => {
        const draft = await plan(comparison);
        const count = draft.configurations.length;
        const detail = draft.configurations.map(summary).join('\n');
        const confirmation = await vscode.window.showInformationMessage(
            `Add ${count} comparison ${count === 1 ? 'stage' : 'stages'} to ${name}?`, { modal: true, detail }, 'Add Stages');
        if (confirmation) return { epoch: draft.epoch, comparison };
    };
    return choice.preset === 'matrix'
        ? editMatrix(stage, model, groups, review)
        : review({ kind: 'preset', preset: choice.preset });
}

async function editMatrix(stage: StageInfo, model: StageModel, groups: () => Promise<StageStateOptions[]>, review: Review) {
    const title = `Compare matrix · ${stage.configuration.name}`;
    const matrix: StageMatrix = {};
    let states: StageStateOptions[] | undefined;
    for (;;) {
        const choice = await choose(title, [
            { label: '$(layout) Sizes', description: count(matrix.sizes), field: 'sizes' },
            { label: '$(add) Add a custom size…', field: 'custom' },
            { label: '$(color-mode) Themes', description: count(matrix.themes), field: 'themes' },
            { label: '$(arrow-swap) Reading directions', description: count(matrix.directions), field: 'directions' },
            { label: '$(database) Design data', description: count(matrix.design_time_data), field: 'data' },
            { label: '$(symbol-enum) Visual states', description: count(matrix.state_groups), field: 'states' },
            { label: '$(check) Review comparison…', field: 'review' },
        ] as const);
        switch (choice?.field) {
            case 'sizes': {
                const active = matrix.sizes ?? [];
                const { width, height } = stage.configuration.environment;
                const current = { name: 'Current size', width, height };
                const presets = [current, ...(model.comparisonSizes ?? []).filter(size => !sameSize(size, current))];
                const values = [...active, ...presets.filter(size => !active.some(item => sameSize(item, size)))];
                matrix.sizes = await pick('Comparison sizes', values.map(value => ({
                    label: value.name, description: `${value.width} × ${value.height}`, value,
                })), active) ?? active;
                break;
            }
            case 'custom': {
                const name = (await editText('Name the comparison size', undefined, undefined, title))?.trim();
                if (!name) break;
                const { width, height } = stage.configuration.environment;
                const bounds = await dimensions(width, height, title);
                if (bounds) {
                    const next = { name, ...bounds };
                    matrix.sizes = [...(matrix.sizes ?? []).filter(size => !sameSize(size, next)), next];
                }
                break;
            }
            case 'themes':
                matrix.themes = await pick('Comparison themes', (model.availableThemes ?? ['light', 'dark', 'inherit'] as const)
                    .map(value => ({ label: stageThemeLabels[value], value })), matrix.themes) ?? matrix.themes;
                break;
            case 'directions':
                matrix.directions = await pick('Comparison reading directions',
                    Object.entries(stageDirectionLabels).map(([value, label]) => ({ label, value: value as keyof typeof stageDirectionLabels })),
                    matrix.directions) ?? matrix.directions;
                break;
            case 'data':
                matrix.design_time_data = await pick('Comparison design data', [
                    { label: 'Application values', value: false }, { label: 'Design data', value: true },
                ], matrix.design_time_data) ?? matrix.design_time_data;
                break;
            case 'states':
                states ??= await groups();
                matrix.state_groups = await editStates(states, matrix.state_groups ?? []);
                break;
            case 'review':
                try {
                    const accepted = await review({ kind: 'matrix', matrix });
                    if (accepted) return accepted;
                } catch (error) {
                    await vscode.window.showWarningMessage(error instanceof Error ? error.message : String(error),
                        { modal: true }, 'Edit Matrix');
                }
                break;
            default: return;
        }
    }
}

async function editStates(groups: StageStateOptions[], active: NonNullable<StageMatrix['state_groups']>) {
    if (!groups.length) {
        void vscode.window.showInformationMessage('This stage has no authored visual states.');
        return active;
    }
    const choice = await choose('Comparison state group', groups.map(group => ({
        label: group.group, description: group.owner_name, group,
    })));
    if (!choice) return active;
    const { owner, group, states } = choice.group;
    const matches = (axis: (typeof active)[number]) => axis.group === group && sameOwner(axis.owner, owner);
    const values = await pick(`States for ${group}`, states.map(value => ({ label: value, value })),
        active.find(matches)?.states);
    if (!values) return active;
    const remaining = active.filter(axis => !matches(axis));
    if (values.length) remaining.push({ owner, group, states: values });
    return remaining;
}

async function pick<T>(title: string, items: Array<vscode.QuickPickItem & { value: T }>, current: readonly T[] = []) {
    const result = await vscode.window.showQuickPick(items.map(item => ({ ...item, picked: current.includes(item.value) })), {
        canPickMany: true, title, placeHolder: 'Leave empty to keep the base stage’s setting', ignoreFocusOut: true,
    });
    return result?.map(item => item.value);
}

function count(values?: readonly unknown[]): string {
    return values?.length ? `${values.length} selected` : 'Base stage';
}

function sameSize(a: StageMatrixSize, b: StageMatrixSize): boolean {
    return a.name === b.name || (a.width === b.width && a.height === b.height);
}

function summary({ name, environment }: StageComparisonPlan['configurations'][number]): string {
    return `${name} · ${environment.width} × ${environment.height}`;
}
