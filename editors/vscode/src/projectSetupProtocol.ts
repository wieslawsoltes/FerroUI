export type ProjectTemplate = 'desktopRsx' | 'desktopXaml';

export interface ProjectTemplateCatalog {
    templates: Array<{ id: ProjectTemplate; label: string; description: string; entry: string }>;
    publishedVersion: string;
    rustVersion: string;
}

export interface ProjectTemplateRequest {
    template: ProjectTemplate;
    name: string;
    checkout?: string;
}

export interface ProjectTemplatePlan {
    root: string;
    entry: string;
    manifest: string;
    files: Array<{ path: string; source: string }>;
    review: string;
}

export interface CreatedProject {
    root: string;
    entry: string;
    manifest: string;
}

export interface ProjectToolchainRequest {
    directory: string;
    minimumRust?: string;
    target?: string;
    cargo?: string;
}

interface ToolProbe {
    version: string | null;
    error: string | null;
}

export interface ProjectToolchainReport {
    directory: string;
    cargo: ToolProbe;
    rustc: ToolProbe;
    host: string | null;
    target: string | null;
    targetInstalled: boolean | null;
    targetError: string | null;
    minimumRust: string;
    meetsMinimum: boolean | null;
}
