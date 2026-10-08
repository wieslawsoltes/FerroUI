import type FileSystemWritableFileStream from "native-file-system-adapter/types/src/FileSystemWritableFileStream";
import { FerroExports } from "./ferroExports";

// Methods of FileSystemWritableFileStream and Blob for the streams of the storage items.
//
// Bytes cross the boundary through the memory of the module: the framework passes the address and
// length of its buffer and the data is copied out of or into the view FerroExports.heapU8() returns.
// The view is fetched on every call because the buffer of the memory is replaced when it grows. A
// writable stream queues the chunk until it is written, and refuses a view over the shared memory of
// a module with threads, so the bytes are copied out of the module memory right away (`slice` copies
// into an ordinary buffer).
export class StreamHelper {
    public static async seek(stream: FileSystemWritableFileStream, position: number): Promise<void> {
        return await stream.seek(position);
    }

    public static async truncate(stream: FileSystemWritableFileStream, size: number): Promise<void> {
        return await stream.truncate(size);
    }

    public static async close(stream: FileSystemWritableFileStream): Promise<void> {
        return await stream.close();
    }

    public static async write(stream: FileSystemWritableFileStream, pointer: number, count: number): Promise<void> {
        const buffer = FerroExports.heapU8().slice(pointer, pointer + count);
        return await stream.write(buffer);
    }

    public static byteLength(stream: Blob): number {
        return stream.size;
    }

    public static async sliceArrayBuffer(stream: Blob, offset: number, count: number): Promise<Uint8Array> {
        const buffer = await stream.slice(offset, offset + count).arrayBuffer();
        return new Uint8Array(buffer);
    }

    public static byteArrayLength(buffer: Uint8Array): number {
        return buffer.length;
    }

    // Copies `buffer` into the module memory at `pointer`; the framework reserved buffer.length bytes there.
    public static toMemoryView(buffer: Uint8Array, pointer: number): void {
        FerroExports.heapU8().set(buffer, pointer);
    }
}
