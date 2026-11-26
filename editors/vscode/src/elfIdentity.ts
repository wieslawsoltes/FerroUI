// Only program headers are needed. Stripping an APK library can remove section
// metadata and debug information while retaining its load segments and build ID.
export interface ElfIdentity { machine: number; elfClass: number; virtualBase: bigint; buildId: string }

export async function elfIdentity(read: (offset: number, length: number) => Promise<Buffer>): Promise<ElfIdentity | undefined> {
    const header = await read(0, 64);
    const elfClass = header[4];
    if (header.length < 64 || header.readUInt32BE(0) !== 0x7f454c46
        || (elfClass !== 1 && elfClass !== 2) || header[5] !== 1 || header.readUInt16LE(16) !== 3) return;
    const elf64 = elfClass === 2;
    const word = (data: Buffer, offset: number) => elf64 ? data.readBigUInt64LE(offset) : BigInt(data.readUInt32LE(offset));
    const tableOffset = word(header, elf64 ? 32 : 28);
    const entrySize = header.readUInt16LE(elf64 ? 54 : 42);
    const tableSize = entrySize * header.readUInt16LE(elf64 ? 56 : 44);
    if (entrySize !== (elf64 ? 56 : 32) || !tableSize || !boundedRead(tableOffset, BigInt(tableSize))) return;
    const table = await read(Number(tableOffset), tableSize);
    if (table.length !== tableSize) return;

    let virtualBase: bigint | undefined;
    let buildId: string | undefined;
    for (let offset = 0; offset < tableSize; offset += entrySize) {
        const type = table.readUInt32LE(offset);
        const fileOffset = word(table, offset + (elf64 ? 8 : 4));
        if (type === 1 && fileOffset === 0n) virtualBase ??= word(table, offset + (elf64 ? 16 : 8));
        if (type === 4 && !buildId) {
            const length = word(table, offset + (elf64 ? 32 : 16));
            if (boundedRead(fileOffset, length)) buildId = gnuBuildId(await read(Number(fileOffset), Number(length)));
        }
    }
    if (virtualBase !== undefined && buildId) return { machine: header.readUInt16LE(18), elfClass, virtualBase, buildId };
}

function boundedRead(offset: bigint, length: bigint): boolean {
    return length > 0n && length <= 65_536n && offset + length <= BigInt(Number.MAX_SAFE_INTEGER);
}

function gnuBuildId(notes: Buffer): string | undefined {
    let cursor = 0;
    while (cursor + 12 <= notes.length) {
        const nameSize = notes.readUInt32LE(cursor);
        const length = notes.readUInt32LE(cursor + 4);
        const type = notes.readUInt32LE(cursor + 8);
        const nameOffset = cursor + 12;
        const valueOffset = nameOffset + Math.ceil(nameSize / 4) * 4;
        const end = valueOffset + length;
        if (end > notes.length) return;
        if (type === 3 && nameSize === 4 && length > 0 && length <= 64 && notes.readUInt32BE(nameOffset) === 0x474e5500) {
            return notes.subarray(valueOffset, end).toString('hex');
        }
        cursor = valueOffset + Math.ceil(length / 4) * 4;
    }
}
