import { downloadAndUnzipVSCode } from '@vscode/test-electron';
import { getDefaultAutoSelectFamilyAttemptTimeout, setDefaultAutoSelectFamilyAttemptTimeout } from 'node:net';
import { setTimeout as wait } from 'node:timers/promises';

const retryDelays = [1_000, 3_000];
const connectionErrors = new Set(['EAI_AGAIN', 'ECONNRESET', 'ENETUNREACH', 'ETIMEDOUT']);

export async function testVSCodeExecutable(supplied, downloadOptions) {
    if (supplied) return supplied;

    // Give each address time to connect while resolving the version from the update service.
    const familyTimeout = getDefaultAutoSelectFamilyAttemptTimeout();
    setDefaultAutoSelectFamilyAttemptTimeout(Math.max(familyTimeout, 1_000));
    try {
        for (let attempt = 0; ; attempt++) {
            try {
                return await downloadAndUnzipVSCode(downloadOptions);
            } catch (error) {
                // The upstream archive retry starts after version resolution.
                const delay = retryDelays[attempt];
                if (delay === undefined || !connectionErrors.has(error?.code)) throw error;
                console.warn(`VS Code download connection failed (${error.code}). Retrying in ${delay / 1_000}s.`);
                await wait(delay);
            }
        }
    } finally {
        setDefaultAutoSelectFamilyAttemptTimeout(familyTimeout);
    }
}
