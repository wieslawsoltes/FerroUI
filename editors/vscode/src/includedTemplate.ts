import type { IncludedSource } from './includedSource';

export interface IncludedTemplateSource extends IncludedSource {
    key: string;
}

export interface IncludedTemplatePreview {
    source: IncludedTemplateSource;
    sample: unknown;
    consumer: string;
}
