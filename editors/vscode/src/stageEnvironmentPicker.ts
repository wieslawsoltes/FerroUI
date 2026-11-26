import type { QuickPickItem } from 'vscode';
import { stageDirectionLabels, stagePointerLabels, stageThemeLabels, type StageConfiguration, type StageModel, type StagePointerProfile } from './stagesProtocol';
import { choose, editText } from './studioContext';

type Field = 'size' | 'theme' | 'direction' | 'locale' | 'textSize' | 'pointerInput';

export async function editStageEnvironment(configuration: StageConfiguration, model: StageModel): Promise<boolean> {
    const environment = configuration.environment;
    const title = `Environment · ${configuration.name}`;
    const options: Array<QuickPickItem & { field: Field }> = [
        { field: 'size', label: '$(layout) Logical size', description: `${environment.width} × ${environment.height}` },
        { field: 'theme', label: '$(color-mode) Theme', description: stageThemeLabels[environment.theme] },
        { field: 'direction', label: '$(arrow-swap) Direction', description: stageDirectionLabels[environment.flow_direction] },
    ];
    if (model.textEnvironment) {
        options.push(
            { field: 'locale', label: '$(globe) Locale', description: configuration.text?.locale ?? 'Application locale' },
            { field: 'textSize', label: '$(text-size) Text size', description: `${configuration.text?.scale_percent ?? 100}%` },
        );
    }
    if (model.pointerInput) options.push({ field: 'pointerInput', label: '$(cursor) Pointer input',
        description: stagePointerLabels[configuration.pointer_input ?? 'automatic'] });
    const choice = await choose(title, options);
    switch (choice?.field) {
        case 'size': {
            const size = await dimensions(environment.width, environment.height, title);
            if (!size) return false;
            Object.assign(environment, size);
            return true;
        }
        case 'theme': {
            const themes = model.availableThemes ?? ['light', 'dark', 'inherit'] as const;
            const theme = await choose('Stage theme', themes.map(value => ({ label: stageThemeLabels[value], value })));
            if (!theme) return false;
            environment.theme = theme.value;
            return true;
        }
        case 'direction': {
            type Direction = StageConfiguration['environment']['flow_direction'];
            const directions = Object.keys(stageDirectionLabels) as Direction[];
            const direction = await choose('Stage reading direction',
                directions.map(value => ({ label: stageDirectionLabels[value], value })));
            if (!direction) return false;
            environment.flow_direction = direction.value;
            return true;
        }
        case 'locale': {
            const input = await editText('Language tag such as fr-CA or ja-JP. Leave blank for the application locale.',
                configuration.text?.locale ?? '', localeError, title);
            if (input === undefined) return false;
            const tag = input.trim();
            configuration.text = { ...configuration.text, locale: tag ? new Intl.Locale(tag).toString() : undefined };
            return true;
        }
        case 'textSize': {
            const input = await editText('Font size from 50% to 300%', `${configuration.text?.scale_percent ?? 100}%`,
                value => textSize(value) === undefined ? 'Enter a whole percentage from 50% to 300%.' : undefined, title);
            if (input === undefined) return false;
            configuration.text = { ...configuration.text, scale_percent: textSize(input) };
            return true;
        }
        case 'pointerInput': {
            const profiles: Array<{ value: StagePointerProfile; label: string; description: string }> = [
                { value: 'automatic', label: '$(device-desktop) Current device', description: 'Use mouse, touch, or pen from the editor' },
                { value: 'touch', label: '$(device-mobile) Touch', description: 'Use native touch contacts in Interact' },
                { value: 'pen', label: '$(edit) Pen', description: 'Use native pen contacts in Interact' },
            ];
            const profile = await choose('Stage pointer input', profiles);
            if (!profile) return false;
            configuration.pointer_input = profile.value === 'automatic' ? undefined : profile.value;
            return true;
        }
        default: return false;
    }
}

function localeError(value: string): string | undefined {
    const tag = value.trim();
    if (!tag) return;
    try { if (tag.length <= 128) { new Intl.Locale(tag); return; } }
    catch { /* Show the accepted input format below. */ }
    return 'Enter a language tag such as fr-CA or ja-JP.';
}

function textSize(value: string): number | undefined {
    const match = /^(\d{1,3})\s*%?$/.exec(value.trim());
    if (!match) return;
    const percent = Number(match[1]);
    return percent >= 50 && percent <= 300 ? percent : undefined;
}

export async function dimensions(width: number, height: number, title?: string): Promise<{ width: number; height: number } | undefined> {
    const value = await editText('Logical size, for example 800 × 520', `${width} × ${height}`, undefined, title);
    if (value === undefined) return;
    const parts = /^\s*(\d{1,4})\s*[x×]\s*(\d{1,4})\s*$/i.exec(value);
    if (!parts || parts.slice(1).some(part => Number(part) < 64 || Number(part) > 2048)) throw new Error('Choose dimensions from 64 to 2048.');
    return { width: Number(parts[1]), height: Number(parts[2]) };
}
