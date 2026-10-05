import { FerroExports } from "./ferroExports";

// Settles promises awaited by the framework: the framework registers a promise under a request id
// and receives its outcome through PromiseHelper_OnResolved or PromiseHelper_OnRejected with that id.
// Not an upstream module; it replaces the Task marshalling of the .NET runtime.
export class PromiseHelper {
    public static track(promise: any, requestId: number): void {
        Promise.resolve(promise).then(
            value => {
                FerroExports.PromiseHelper?.OnResolved(requestId, value);
            },
            error => {
                FerroExports.PromiseHelper?.OnRejected(requestId, PromiseHelper.errorMessage(error));
            });
    }

    private static errorMessage(error: any): string {
        if (error instanceof Error || error instanceof DOMException) {
            return error.message;
        }
        return String(error);
    }
}
