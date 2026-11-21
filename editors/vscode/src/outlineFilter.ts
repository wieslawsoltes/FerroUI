interface ControlRow {
    id: number | string;
    parent: number | string | null;
    label: string;
    name: string | null;
}

/** Keep matching rows and the paths that explain where they are in the tree. */
export function filterOutline<Row extends ControlRow>(rows: readonly Row[], filter: string): readonly Row[] {
    const query = filter.trim().toLowerCase();
    if (!query) return rows;
    const byId = new Map(rows.map(row => [row.id, row]));
    const visible = new Set<ControlRow['id']>();
    for (const row of rows) {
        if (!`${row.label} ${row.name ?? ''}`.toLowerCase().includes(query)) continue;
        let current: Row | undefined = row;
        while (current && !visible.has(current.id)) {
            visible.add(current.id);
            current = current.parent === null ? undefined : byId.get(current.parent);
        }
    }
    return rows.filter(row => visible.has(row.id));
}
