// Serves a built browser site on 127.0.0.1, to open it in a browser by hand.
//
//   node scripts/browser/serve.mjs <site directory> [--isolated] [--port <number>]     default port: 8080
//
// --isolated sends `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy:
// require-corp` with every response, which makes the pages cross-origin isolated: a site built with
// `scripts/build-browser.sh --threads` needs that. Without the option such a site falls back to its
// service worker, as on a static host (docs/porting/browser-platform.md, "Threads (opt-in)").
import { serve } from "./harness.mjs";

let site; let isolated = false; let port = 8080;
const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
    if (args[i] === "--isolated") { isolated = true; } else if (args[i] === "--port") { port = Number(args[++i]); } else if (args[i].startsWith("-")) { site = undefined; break; } else { site = args[i]; }
}
if (!site || !Number.isInteger(port)) {
    console.error("usage: node scripts/browser/serve.mjs <site directory> [--isolated] [--port <number>]");
    process.exit(2);
}

const server = await serve(site, { isolated, port });
console.log(`http://127.0.0.1:${server.address().port}/ serves ${site}${isolated ? ", cross-origin isolated" : ""} (Ctrl+C stops it)`);
