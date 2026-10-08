import { ferrouiDb, fileBookmarksStore } from "./indexedDb";
import { StorageItem, StorageItems } from "./storageItem";
import { showOpenFilePicker, showDirectoryPicker, showSaveFilePicker, FileSystemFileHandle } from "native-file-system-adapter";
import { DownloadFileHandle } from "./downloadFileHandle";

declare global {
    type WellKnownDirectory = "desktop" | "documents" | "downloads" | "music" | "pictures" | "videos";
    interface FilePickerAcceptType {
        description?: string | undefined;
        accept: Record<string, string | string[]>;
    }
}

// Looked up when the bundle is loaded, which is when the polyfill looks for the native pickers.
const nativeSaveFilePicker = (globalThis as any).showSaveFilePicker;

export class StorageProvider {
    public static async selectFolderDialog(
        startIn: StorageItem | null,
        preferPolyfill: boolean): Promise<StorageItem> {
        // 'Picker' API doesn't accept "null" as a parameter, so it should be set to undefined.
        const options = {
            startIn: (startIn?.wellKnownType ?? startIn?.handle ?? undefined),
            _preferPolyfill: preferPolyfill
        };

        const handle = await showDirectoryPicker(options as any);
        return StorageItem.createFromHandle(handle);
    }

    public static async openFileDialog(
        startIn: StorageItem | null, multiple: boolean,
        types: FilePickerAcceptType[] | null, excludeAcceptAllOption: boolean,
        preferPolyfill: boolean): Promise<StorageItems> {
        const options = {
            startIn: (startIn?.wellKnownType ?? startIn?.handle ?? undefined),
            multiple,
            excludeAcceptAllOption,
            types: (types ?? undefined),
            _preferPolyfill: preferPolyfill
        };

        const handles = await showOpenFilePicker(options);
        return new StorageItems(handles.map((handle: FileSystemFileHandle) => StorageItem.createFromHandle(handle)));
    }

    public static async saveFileDialog(
        startIn: StorageItem | null, suggestedName: string | null,
        types: FilePickerAcceptType[] | null, excludeAcceptAllOption: boolean,
        preferPolyfill: boolean): Promise<StorageItem> {
        const options = {
            startIn: (startIn?.wellKnownType ?? startIn?.handle ?? undefined),
            suggestedName: (suggestedName ?? undefined),
            excludeAcceptAllOption,
            types: (types ?? undefined),
            _preferPolyfill: preferPolyfill
        };

        // Without the native picker (or when the polyfill is preferred) the polyfill returns a handle
        // that saves through a download. That handle is the port's own (downloadFileHandle.ts): the
        // polyfill's starts the download before the service worker is known to have the stream.
        const handle = nativeSaveFilePicker && !preferPolyfill
            ? await showSaveFilePicker(options)
            : new FileSystemFileHandle(new DownloadFileHandle(options.suggestedName));
        return StorageItem.createFromHandle(handle);
    }

    public static async openBookmark(key: string): Promise<StorageItem | null> {
        const connection = await ferrouiDb.connect();
        try {
            const handle = await connection.get(fileBookmarksStore, key);
            return handle && StorageItem.createFromHandle(handle, key);
        } finally {
            connection.close();
        }
    }

    public static createAcceptType(description: string, mimeTypes: string[], extensions: string[] | undefined): FilePickerAcceptType {
        const accept: Record<string, string[]> = {};
        mimeTypes.forEach(a => { accept[a] = extensions ?? []; });
        return { description, accept };
    }
}
