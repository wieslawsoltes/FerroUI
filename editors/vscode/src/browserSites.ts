import { randomUUID } from 'node:crypto';
import { realpath } from 'node:fs/promises';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import type { AddressInfo } from 'node:net';
import { serveSiteFile } from '../../../scripts/browser/site-files.cjs';

export interface PublishedBrowserSite {
    url: string;
    unpublish(): void;
}

/** Each page uses an immutable site path at the managed Chromium profile's loopback origin. */
export class BrowserSites {
    private readonly sites = new Map<string, { directory: string; isolated: boolean }>();
    private readonly server = createServer((request, response) => {
        void this.serve(request, response).catch(error => response.destroy(error));
    });
    private address?: string;
    private closing?: Promise<void>;

    async listen(): Promise<void> {
        await new Promise<void>((resolve, reject) => this.server.once('error', reject).listen(0, '127.0.0.1', resolve));
        this.address = `127.0.0.1:${(this.server.address() as AddressInfo).port}`;
    }

    async publish(site: string, isolated: boolean): Promise<PublishedBrowserSite> {
        const directory = await realpath(site);
        if (!this.address || this.closing) throw new Error('The managed browser site server is unavailable.');
        const id = randomUUID();
        this.sites.set(id, { directory, isolated });
        return { url: `http://${this.address}/${id}/`, unpublish: () => { this.sites.delete(id); } };
    }

    close(): Promise<void> {
        return this.closing ??= new Promise(resolve => {
            this.sites.clear();
            this.server.close(() => resolve());
            this.server.closeAllConnections();
        });
    }

    private async serve(request: IncomingMessage, response: ServerResponse): Promise<void> {
        const url = request.url?.split('?', 1)[0] ?? '';
        if (request.headers.host === this.address && url === '/favicon.ico') {
            response.statusCode = 204;
            response.end();
            return;
        }
        const slash = url.indexOf('/', 1);
        const site = slash > 1 && request.headers.host === this.address && this.sites.get(url.slice(1, slash));
        if (!site) {
            response.statusCode = 404;
            response.end();
            return;
        }
        response.setHeader('cache-control', 'no-store');
        if (site.isolated) {
            response.setHeader('cross-origin-opener-policy', 'same-origin');
            response.setHeader('cross-origin-embedder-policy', 'require-corp');
        }
        await serveSiteFile(site.directory, url.slice(slash), request, response);
    }
}
