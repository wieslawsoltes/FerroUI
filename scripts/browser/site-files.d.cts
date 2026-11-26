import type { IncomingMessage, ServerResponse } from 'node:http';

export function serveSiteFile(root: string, pathname: string, request: IncomingMessage, response: ServerResponse): Promise<void>;
