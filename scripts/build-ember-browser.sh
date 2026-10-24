#!/bin/bash
# The shared framework builds the WebAssembly application; this wrapper also
# gives the default, non-threaded site a coherent offline application shell.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
"$ROOT/scripts/build-browser.sh" ember-browser "$@"
node "$ROOT/samples/Ember.Browser/write-offline-manifest.mjs" "$ROOT" "$@"
