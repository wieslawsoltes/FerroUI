import { FerroExports } from "./ferroExports";

// Settles promises awaited by the framework: the framework registers a promise under a request id
// and receives its outcome through PromiseHelper_OnResolved or PromiseHelper_OnRejected with that id.
// Not an upstream module; it replaces the Task marshalling of the .NET runtime.
export class PromiseHelper {
    public static track(promise: any, requestId: number): void {
        Promise.resolve(promise).then(
            value => {
                PromiseHelper.exports(requestId)?.OnResolved(requestId, value);
            },
            error => {
                PromiseHelper.exports(requestId)?.OnRejected(requestId, PromiseHelper.errorMessage(error));
            });
    }

    // The completion exports; without them the request can never complete, which is reported.
    private static exports(requestId: number) {
        const exports = FerroExports.PromiseHelper;
        if (!exports?.OnResolved || !exports?.OnRejected) {
            console.error(`The promise of request ${requestId} settled, but the module exports no PromiseHelper completions: the request never completes.`);
            return undefined;
        }
        return exports;
    }

    private static errorMessage(error: any): string {
        if (error instanceof Error || error instanceof DOMException) {
            return error.message;
        }
        return String(error);
    }
}
