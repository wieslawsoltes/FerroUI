import type { DebugAdapterTracker } from 'vscode';
import type { DebuggerPause } from './debuggerPause';

const resumeCommands = new Set(['continue', 'next', 'stepIn', 'stepOut', 'stepBack', 'reverseContinue', 'goto', 'restartFrame']);

type Stops = Map<number | undefined, symbol>;
interface Resume { thread?: number; singleThread?: boolean; before: Stops }

interface AdapterMessage {
    type?: string;
    event?: string;
    command?: string;
    seq?: number;
    request_seq?: number;
    success?: boolean;
    arguments?: { threadId?: number; singleThread?: boolean };
    body?: { threadId?: number; allThreadsContinued?: boolean; reason?: string };
}

export function trackDebuggerPause(pause: DebuggerPause, current: () => boolean): DebugAdapterTracker {
    const adapter = Symbol();
    const stopped: Stops = new Map();
    const resumes = new Map<number, Resume>();
    const update = () => pause.set(adapter, stopped.size > 0 && current());
    const clear = () => { stopped.clear(); resumes.clear(); update(); };
    const resume = (thread: number | undefined, all: boolean, before: Stops) => {
        for (const [id, marker] of before) {
            if ((all || id === thread) && stopped.get(id) === marker) stopped.delete(id);
        }
        update();
    };
    return {
        onWillReceiveMessage(message: AdapterMessage) {
            const { arguments: args, command, seq } = message;
            if (!stopped.size || message.type !== 'request' || seq === undefined || !command || !resumeCommands.has(command)) return;
            resumes.set(seq, { before: new Map(stopped), thread: args?.threadId, singleThread: args?.singleThread });
        },
        onDidSendMessage(message: AdapterMessage) {
            if (!current()) { clear(); return; }
            const { body, event, request_seq: sequence } = message;
            if (message.type === 'response' && sequence !== undefined) {
                const operation = resumes.get(sequence);
                resumes.delete(sequence);
                // Adapters can acknowledge a step after its next stop has already arrived.
                if (message.success && operation) resume(operation.thread,
                    body?.allThreadsContinued ?? !operation.singleThread, operation.before);
            } else if (event === 'stopped') {
                stopped.set(body?.threadId, Symbol());
                update();
            } else if (event === 'continued') {
                resume(body?.threadId, body?.allThreadsContinued !== false, stopped);
            } else if (event === 'thread' && body?.reason === 'exited') {
                resume(body.threadId, false, stopped);
            } else if (event === 'terminated' || event === 'exited') clear();
        },
        onWillStopSession: clear,
        onExit: clear,
        onError: clear,
    };
}
