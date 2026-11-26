import { choice, integer, tool } from './agentSchema';

const path = { type: 'string', maxLength: 4096 };
const review = { type: 'string', minLength: 16, maxLength: 16 };
const args = {
    folder: path, template: choice('desktopRsx', 'desktopXaml'), checkout: path,
    name: { type: 'string', minLength: 1, maxLength: 64 },
};
const required = ['template', 'name'];
const cargo = { folder: path, manifest_path: path, package: path, binary: path };

export const projectSetupCatalog = {
    ferroui_project_templates: tool('List the runnable FerroUI project starters and the published framework and minimum Rust versions.', {}),
    ferroui_project_plan_new: tool('Review the files for a new standalone Cargo app under a workspace folder. Optional checkout is relative to that workspace folder.', args, required),
    ferroui_project_create: tool('Create a reviewed standalone Cargo app in its new, unoccupied folder. Pass the same arguments and review from ferroui_project_plan_new.', { ...args, review }, [...required, 'review'], true, false),
    ferroui_project_check_toolchain: tool('Check Cargo, the active Rust compiler, and installed Rust target libraries without building the project. Pass a workspace folder or choose a Cargo binary from ferroui_reload_targets.', {
        ...cargo, target: { type: 'string', maxLength: 100 },
    }),
    ferroui_project_run_cargo: tool('Launch an exact workspace Cargo binary in the VS Code task terminal. Select manifest, package and binary from ferroui_reload_targets.', cargo, ['manifest_path', 'package', 'binary'], true, false),
    ferroui_project_cargo_tasks: tool('List running FerroUI Cargo tasks launched in the VS Code task terminal.', {}),
    ferroui_project_stop_cargo: tool('Stop a running FerroUI Cargo task using its ID from ferroui_project_cargo_tasks.', { id: integer(1) }, ['id'], true, false),
};
