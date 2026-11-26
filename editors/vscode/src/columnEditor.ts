import type { QuickPickItem } from 'vscode';
import type { Range } from './protocol';
import { choose, editText, type StudioContext } from './studioContext';
import { editItemTemplates } from './studioItemTemplates';

type ColumnKind = 'table' | 'text' | 'checkBox' | 'template' | 'rowHeader' | 'expander';
type ColumnFieldKind = 'text' | 'length' | 'boolean' | 'choice' | 'binding' | 'object';

export interface ColumnField {
    name: string;
    kind: ColumnFieldKind;
    value: string | null;
    choices: string[];
    range: Range | null;
    editable: boolean;
    canClear: boolean;
}

export interface AuthoredColumn {
    node: number;
    kind: ColumnKind;
    label: string;
    range: Range;
    fields: ColumnField[];
    children: AuthoredColumn[];
    canDuplicate: boolean;
}

export interface TableColumns {
    table: number;
    name: string | null;
    kind: 'TableView' | 'TreeDataGrid';
    columns: AuthoredColumn[];
    available: { kind: ColumnKind; label: string }[];
    reason: string | null;
    sourceRange?: Range | null;
    sourceUri?: string;
    factory?: string;
    model: string;
}

type Change =
    | { action: 'insert'; index: number; kind: ColumnKind }
    | { action: 'move'; column: number; index: number }
    | { action: 'duplicate' | 'remove'; column: number }
    | { action: 'set'; column: number; field: string; value: string }
    | { action: 'clear'; column: number; field: string };

export const columnKinds: Record<ColumnKind, { label: string; icon: string }> = {
    table: { label: 'Column', icon: 'table' },
    text: { label: 'Text', icon: 'symbol-string' },
    checkBox: { label: 'Checkbox', icon: 'check' },
    template: { label: 'Template', icon: 'symbol-structure' },
    rowHeader: { label: 'Row header', icon: 'list-ordered' },
    expander: { label: 'Hierarchy', icon: 'list-tree' },
};

export class ColumnEditor {
    constructor(readonly context: StudioContext, readonly table: TableColumns) {}

    reveal(range: Range): Promise<void> {
        return this.context.revealRustRange(range, this.table.sourceUri);
    }

    async change(change: Change): Promise<void> {
        if (this.table.reason) throw new Error(this.table.reason);
        await this.context.edit('designer_columns_edit', {
            table: this.table.table, expected_model: this.table.model, ...change,
        });
    }

    async add(): Promise<void> {
        this.context.ensure();
        const available = this.table.available;
        const kind = available.length === 1 ? available[0].kind : (await choose('Add Column', available.map(item => ({
            columnKind: item.kind, label: `$(${columnKinds[item.kind].icon}) ${item.label}`,
        }))))?.columnKind;
        if (kind) await this.change({ action: 'insert', index: this.table.columns.length, kind });
    }

    async field(column: AuthoredColumn, field: ColumnField): Promise<void> {
        this.context.ensure();
        const template = this.rustTemplate(field);
        if (template) {
            await editItemTemplates(this.context, column.node, template);
            return;
        }
        if (!field.editable || this.table.reason) {
            if (field.range) await this.reveal(field.range);
            return;
        }
        const label = `${column.label} · ${field.name}`;
        const value = field.choices.length
            ? (await choose<QuickPickItem>(label, field.choices.map(option => ({
                label: option, description: field.value === option ? 'Current' : undefined,
            }))))?.label
            : await editText(fieldPrompt(field, this.context.panel.language === 'rust'), field.value ?? undefined, undefined, label);
        if (value !== undefined) {
            await this.change({ action: 'set', column: column.node, field: field.name, value });
        }
    }

    rustTemplate(field: ColumnField): 'CellTemplate' | 'CellEditingTemplate' | undefined {
        if (this.context.panel.language === 'rust' && this.context.panel.supportsItemTemplates
            && (field.name === 'CellTemplate' || field.name === 'CellEditingTemplate')) {
            return field.name;
        }
    }
}

function fieldPrompt(field: ColumnField, rust: boolean): string {
    if (field.kind === 'length') return 'Enter Auto, a pixel width, or a star width such as 2*.';
    if (field.kind === 'binding') return rust ? 'Enter a row property path such as Account.Name.' : 'Enter a binding expression such as {Binding CustomerName}.';
    return field.name;
}
