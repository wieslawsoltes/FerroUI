import type { PrototypeScene, StageFocus, StageImage, StageInfo, StagesRequest } from '../src/stagesProtocol';
import { create, icon, iconButton, labelButton } from './dom';
import { StageKeyboard } from './stageKeyboard';
import type { StageField, StageMenu } from './stageMenu';

export class StageCard {
    readonly element = create('article');
    readonly heading = create('h2');
    private readonly image = create('img');
    private readonly placeholder = create('div', 'Rendering stage…');
    private readonly status = create('p');
    private readonly size = create('button');
    private readonly environment = iconButton('paintcan', 'Environment', () => this.edit('environment'));
    private readonly fixture = create('button');
    private readonly states = create('button');
    private readonly mode = iconButton('cursor', 'Interact', () => this.edit('mode'));
    private readonly pause = iconButton('debug-pause', 'Pause', () => this.edit('pause'));
    private readonly checks = iconButton('check-all', 'Preview checks', () => this.send({ type: 'checks', id: this.stage.id, epoch: this.stage.epoch }));
    private readonly more = iconButton('ellipsis', 'Stage actions', () => this.menu.prepare(this.more, this.stage, field => this.edit(field)));
    private readonly keyboard: StageKeyboard;
    private displayedEpoch?: number;
    private pendingImage = 0;
    private keyboardSupported = false;
    private keyboardCurrent = false;

    constructor(public stage: StageInfo, private readonly menu: StageMenu, private readonly send: (request: StagesRequest) => void,
        private readonly scene: () => PrototypeScene | null) {
        this.element.className = 'stage-card';
        this.heading.tabIndex = -1;
        this.heading.id = `stage-heading-${stage.id}`;
        this.element.setAttribute('aria-labelledby', this.heading.id);
        const header = create('header');
        header.className = 'stage-card-header';
        const tools = create('div');
        tools.className = 'tool-group';
        tools.setAttribute('role', 'group');
        tools.setAttribute('aria-label', 'Stage tools');
        this.menu.bind(this.more);
        const settings = create('div');
        settings.className = 'stage-settings';
        for (const [button, field] of [[this.size, 'layout'], [this.fixture, 'designData'], [this.states, 'state']] as const) {
            button.type = 'button';
            button.className = 'metric stage-setting';
            button.onclick = () => this.edit(field);
        }
        settings.append(this.size, this.environment, this.fixture, this.states);
        header.append(this.heading, tools, settings);
        const artboard = create('div');
        artboard.className = 'stage-artboard preview-surface';
        this.placeholder.className = 'stage-placeholder';
        this.status.className = 'stage-frame-status';
        this.status.setAttribute('role', 'status');
        this.status.hidden = true;
        this.image.draggable = false;
        this.image.hidden = true;
        this.image.onclick = event => this.pointer(event);
        artboard.append(this.placeholder, this.image, this.status);
        this.keyboard = new StageKeyboard(this.element, artboard, stage.id, this.mode, input =>
            this.send({ type: 'keyboard', id: this.stage.id, epoch: this.stage.epoch, input }));
        tools.append(this.mode, this.keyboard.button, this.pause, this.checks, this.more);
        this.element.append(header, artboard);
        this.update(stage);
    }

    update(stage: StageInfo): void {
        this.stage = stage;
        const { configuration, status, layout_size: layout } = stage;
        const { name, environment } = configuration;
        this.heading.textContent = this.heading.title = name;
        this.image.alt = `${name} preview`;
        this.element.dataset.mode = configuration.mode;
        this.element.dataset.state = status.state;
        const dimensions = `${Math.round(layout.width)} × ${Math.round(layout.height)}`;
        setting(this.size, 'layout', dimensions, `Sizing for ${name} · ${dimensions}`);
        const theme = { light: 'Light', dark: 'Dark', inherit: 'Inherited theme' }[environment.theme];
        const direction = { inherit: 'Inherited direction', left_to_right: 'Left to right', right_to_left: 'Right to left' }[environment.flow_direction];
        labelButton(this.environment, `Environment for ${name} · ${theme} · ${direction}`,
            environment.theme === 'dark' ? 'color-mode' : 'paintcan');
        const fixture = configuration.design_time_data ? configuration.design_fixture : 'Application values';
        this.fixture.hidden = configuration.design_time_data && !configuration.design_fixture;
        setting(this.fixture, 'database', fixture ?? 'Design defaults', `Design data for ${name} · ${fixture ?? 'View default'}`);
        const states = configuration.states.map(({ group, state }) => `${group} · ${state}`);
        this.states.hidden = states.length === 0;
        if (states.length) setting(this.states, 'symbol-enum', states.length === 1 ? configuration.states[0].state : `${states.length} states`,
            `Visual states for ${name} · ${states.join(' | ')}`);
        labelButton(this.mode, `Interact with ${name}`, configuration.mode === 'select' ? 'cursor' : 'run');
        this.mode.setAttribute('aria-pressed', String(configuration.mode !== 'select'));
        labelButton(this.pause, configuration.auto_update ? `Pause ${name}` : `Resume ${name}`, configuration.auto_update ? 'debug-pause' : 'play');
        labelButton(this.checks, `Preview checks for ${name}`);
        labelButton(this.more, `More actions for ${name}`);
        this.checks.disabled = status.state !== 'ready';
        const stateMessages: Record<string, string> = {
            paused: 'Updates paused', pending_source: 'Updating preview…', invalid_source: 'Fix the source to resume preview',
            missing_document: 'Source document is unavailable', failed: 'Preview unavailable',
        };
        const message = status.message ?? stateMessages[status.state];
        if (this.status.textContent !== (message ?? '')) this.status.textContent = message ?? '';
        this.status.hidden = !message || this.image.hidden;
        this.placeholder.textContent = message ?? 'Rendering stage…';
        this.updateKeyboard();
    }

    get imageSource(): string | undefined { return this.image.hidden ? undefined : this.image.src; }

    setKeyboard(supported: boolean, current: boolean): void {
        this.keyboardSupported = supported;
        this.keyboardCurrent = current;
        this.updateKeyboard();
    }

    keyboardFocus(epoch: number, focus: StageFocus | null): void {
        if (this.stage.epoch === epoch) this.keyboard.focused(focus);
    }

    async present(epoch: number, image: StageImage): Promise<string | undefined> {
        if (this.stage.epoch !== epoch) return;
        const version = ++this.pendingImage;
        const next = new Image();
        next.src = `data:image/png;base64,${image.png}`;
        try { await next.decode(); } catch { return; }
        if (this.stage.epoch !== epoch || version !== this.pendingImage || !this.element.isConnected) return;
        this.image.src = next.src;
        this.placeholder.hidden = true;
        this.image.hidden = false;
        this.status.hidden = !this.status.textContent;
        this.displayedEpoch = epoch;
        this.updateKeyboard();
        return next.src;
    }

    private updateKeyboard(): void {
        const { configuration, epoch, status } = this.stage;
        const offered = this.keyboardSupported && !this.scene() && configuration.mode === 'live';
        const enabled = offered && this.keyboardCurrent && status.state === 'ready' && this.displayedEpoch === epoch;
        this.keyboard.update(configuration.name, offered, enabled);
    }

    private edit(field: StageField): void {
        this.send({ type: 'edit', id: this.stage.id, epoch: this.stage.epoch, field });
    }

    private pointer(event: MouseEvent): void {
        const { stage } = this;
        if (stage.status.state !== 'ready' || this.displayedEpoch !== stage.epoch) return;
        const scene = this.scene();
        if (scene && scene.current !== stage.id) {
            if (scene.overlays.at(-1)?.options.dismiss_on_outside) this.send({ type: 'prototype', command: { action: 'closeOverlay' } });
            return;
        }
        const bounds = this.image.getBoundingClientRect();
        this.send({ type: 'pointer', id: stage.id, epoch: stage.epoch,
            x: (event.clientX - bounds.left) / bounds.width, y: (event.clientY - bounds.top) / bounds.height });
        this.keyboard.enter();
    }
}

function setting(button: HTMLButtonElement, glyph: string, value: string, label: string): void {
    if (button.dataset.value !== value) {
        button.dataset.value = value;
        button.replaceChildren(icon(glyph), create('span', value));
    }
    labelButton(button, label);
}
