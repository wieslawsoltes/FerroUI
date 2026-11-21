import type { OutputChannel } from 'vscode';

export type ProcessOutput = Pick<OutputChannel, 'append' | 'appendLine'>;

export class ReloadLog implements ProcessOutput {
    private recent = '';

    constructor(private readonly channel: OutputChannel) {}

    append(value: string): void {
        this.channel.append(value);
        this.recent = (this.recent + value).slice(-48_000);
    }

    appendLine(value: string): void { this.append(value + '\n'); }
    reset(): void { this.recent = ''; }

    tail(lines: number): string {
        return this.recent.replace(/(Authorization:\s*Bearer\s+)\S+/gi, '$1[redacted]')
            .trimEnd().split(/\r?\n/).slice(-lines).join('\n');
    }
}
