import { FerroExports } from "./ferroExports";

// Not in the original, where the runtime turns a promise of the page into a task of the
// framework. A function of this module that answers with a promise returns the promise; the
// framework gives it the id of its request and hands both to track, and the outcome comes back
// through the exports CompletionHelper_OnResolved and CompletionHelper_OnRejected with that id.
export class CompletionHelper {
    public static track(promise: unknown, requestId: number): void {
        Promise.resolve(promise).then(
            (value: unknown) => {
                CompletionHelper.exports(requestId)?.OnResolved(requestId, value);
            },
            (reason: unknown) => {
                CompletionHelper.exports(requestId)?.OnRejected(
                    requestId, CompletionHelper.errorName(reason), CompletionHelper.errorMessage(reason));
            });
    }

    // The completion exports; without them the request can never complete, which is reported.
    private static exports(requestId: number) {
        const exports = FerroExports.CompletionHelper;
        if (!exports?.OnResolved || !exports?.OnRejected) {
            console.error(`The promise of request ${requestId} settled, but the module exports no CompletionHelper completions: the request never completes.`);
            return undefined;
        }
        return exports;
    }

    // A DOMException is not an Error in every browser; both carry a name and a message.
    private static errorName(reason: unknown): string {
        if (reason instanceof Error || reason instanceof DOMException) {
            return reason.name;
        }
        return "";
    }

    private static errorMessage(reason: unknown): string {
        if (reason instanceof Error || reason instanceof DOMException) {
            return reason.message;
        }
        return String(reason);
    }
}
