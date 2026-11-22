import { integer, tool, truth } from './agentSchema';
import { runtimeArgument as runtime } from './runtimeTargets';

const handle = { runtime, expected_generation: integer(0), profile_id: integer(1) };
const required = ['runtime', 'expected_generation', 'profile_id'];

export const runtimeProfileCatalog = {
    ferroui_runtime_profile_windows: tool('List the live windows of a profile-capable app and the connection generation. Uses the managed app or an available attached app by default.', { runtime }),
    ferroui_runtime_profile_start: tool('Record native layout and RSX component work in one window for a fixed time. Use the window ID and generation returned from ferroui_runtime_profile_windows.',
        { runtime, expected_generation: integer(0), window: integer(1), duration_ms: integer(100, 30_000) },
        ['runtime', 'expected_generation', 'window', 'duration_ms'], true, false),
    ferroui_runtime_profile_read: tool('Read native layout and RSX component timings. Pass the capture identity and app generation to read exactly that capture.', handle),
    ferroui_runtime_profile_stop: tool('End a specific UI profile early and return the final measurements.', handle, required, true, false),
    ferroui_runtime_profile_source: tool('Resolve a profile row’s live source and optionally open it in the editor. Uses the app generation, profile ID, and row ID from the capture.',
        { ...handle, row: integer(0), reveal_source: truth }, [...required, 'row'], true, false),
};
