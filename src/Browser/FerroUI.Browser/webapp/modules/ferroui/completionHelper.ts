import { FerroExports } from "./ferroExports";

// Not in the original, where the runtime turns a promise of the page into a task of the
// framework. A function of this module that answers with a promise returns the promise; the
// framework gives it the id of its request and hands both to track, and the outcome comes back
// through the exports CompletionHelper_OnResolved and CompletionHelper_OnRejected with that id.
export class CompletionHelper {
    public static track(promise: unknown, requestId: number): void {
        Promise.resolve(promise).then(
            (value: unknown) => {
                FerroExports.CompletionHelper?.OnResolved(requestId, value);
            },
            (reason: unknown) => {
                const name = reason instanceof Error ? reason.name : "";
                const message = reason instanceof Error ? reason.message : String(reason);
                FerroExports.CompletionHelper?.OnRejected(requestId, name, message);
            });
    }
}
