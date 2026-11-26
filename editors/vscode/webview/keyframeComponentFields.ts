import type { CurveChannel, KeyframeComponent, KeyframeCurveValue, KeyframeValueDraft } from '../src/timelineProtocol';
import { create, element } from './dom';
import { channelComponent, channelLabel, isColorChannel, normalizedChannel } from './timelineCurves';

const brushLabels: Record<string, string> = {
    start_point: 'Start',
    end_point: 'End',
    gradient_origin: 'Origin',
};

function fieldGroup(channel: CurveChannel): { group: string; label: string } {
    const [family, index, kind] = channel.split('/');
    const component = channelComponent(channel);
    const label = channelLabel(component);
    if (family === 'gradient_stop') return { group: `Stop ${index}`, label };
    if (family === 'box_shadow') return { group: `Shadow ${index}`, label };
    if (family === 'transform_operation') {
        return { group: `Operation ${index} · ${kind[0].toUpperCase() + kind.slice(1)}`, label };
    }
    if (family === 'effect_parameter' && component !== 'value') {
        return { group: channelLabel(channel).slice(0, -label.length).trimEnd(), label };
    }
    if (family === 'gradient') {
        if (index === 'transform_origin') return { group: 'Transform origin', label };
        const short = brushLabels[index];
        return { group: 'Brush', label: short ? `${short} ${label}` : channelLabel(channel) };
    }
    return { group: family === 'effect_parameter' ? 'Parameters' : 'Value',
        label: channelLabel(channel) };
}

/** Keep inactive value groups and the source text draft while the user switches modes. */
export class KeyframeComponentFields {
    private readonly mode = element<HTMLButtonElement>('keyframe-text-mode');
    private readonly groupField = element('keyframe-component-group-field');
    private readonly group = element<HTMLSelectElement>('keyframe-component-group');
    private readonly fields = element('keyframe-component-fields');
    private readonly textField = element('keyframe-value-field');
    private readonly groups = new Map<string, KeyframeCurveValue[]>();
    private readonly entries = new Map<CurveChannel, string>();
    private asText = false;

    constructor(private readonly text: HTMLInputElement, changed: () => void) {
        this.group.onchange = () => this.render();
        this.mode.onclick = () => {
            this.asText = !this.asText;
            this.render();
            changed();
        };
    }

    fill(values: KeyframeCurveValue[] | null | undefined, preserveMode: boolean): void {
        const previousGroup = this.group.value;
        this.groups.clear();
        this.entries.clear();
        for (const value of values ?? []) {
            if (value.channel === 'value') continue;
            const { group } = fieldGroup(value.channel);
            const components = this.groups.get(group) ?? [];
            components.push(value);
            this.groups.set(group, components);
        }
        this.asText = preserveMode && this.asText;
        const groups = [...this.groups.keys()];
        this.group.replaceChildren(...groups.map(group => new Option(group, group)));
        this.group.value = preserveMode && this.groups.has(previousGroup) ? previousGroup : this.groups.has('Brush') ? 'Brush' : groups[0] ?? '';
        this.render();
    }

    enable(enabled: boolean): void { this.mode.disabled = !enabled; }

    draft(original: string): KeyframeValueDraft {
        if (this.fields.hidden) return { value: this.text.value };
        const components: KeyframeComponent[] = [];
        for (const [group, fields] of this.groups) {
            for (const { channel, value, step } of fields) {
                const raw = this.entries.get(channel);
                if (raw === undefined) continue;
                const next = raw === '' ? NaN : Number(raw);
                const label = channelLabel(channel);
                const name = group === 'Value' || group === 'Brush' || group === 'Parameters' ? label : `${group} · ${fieldGroup(channel).label}`;
                if (!Number.isFinite(next)) throw new Error(`Enter a number for ${name}.`);
                if (step === 'integer' && (!Number.isInteger(next) || next < -2147483648 || next > 2147483647)) {
                    throw new Error(`Enter a signed 32-bit integer for ${name}.`);
                }
                if (normalizedChannel(channel) && (next < 0 || next > 1)) {
                    throw new Error(`Enter a value from 0 to 1 for ${name}.`);
                }
                if (next !== value) components.push({ channel, value: next });
            }
        }
        return components.length ? { components } : { value: original };
    }

    private render(): void {
        const compound = this.groups.size > 0;
        const typed = compound && !this.asText;
        this.mode.hidden = !compound;
        this.mode.setAttribute('aria-pressed', String(this.asText));
        this.textField.hidden = typed;
        this.groupField.hidden = !typed || this.groups.size < 2;
        this.fields.hidden = !typed;
        this.fields.replaceChildren(...(typed ? this.groups.get(this.group.value) ?? [] : []).map(value => this.field(value)));
    }

    private field({ channel, value, step }: KeyframeCurveValue): HTMLLabelElement {
        const label = create('label');
        const caption = create('span');
        const name = fieldGroup(channel).label;
        const normalized = normalizedChannel(channel);
        const units = normalized ? ' (0–1)' : '';
        caption.textContent = name + units;
        label.title = channelLabel(channel) + units;
        const input = create('input');
        input.id = `keyframe-component-${channel}`;
        input.setAttribute('aria-label', label.title);
        // Four decimals distinguish every authored 8-bit channel value.
        const display = isColorChannel(channel) ? Math.round(value * 10_000) / 10_000 : value;
        const draft = this.entries.get(channel) ?? String(display);
        if (step === 'boolean') {
            input.type = 'checkbox';
            input.checked = draft === '1';
            label.className = 'keyframe-component-flag';
        } else {
            input.type = 'number';
            input.step = step === 'integer' ? '1' : 'any';
            input.value = draft;
            if (normalized) { input.min = '0'; input.max = '1'; }
        }
        input.oninput = () => this.entries.set(channel, step === 'boolean' ? String(Number(input.checked)) : input.value);
        label.append(caption, input);
        return label;
    }
}
