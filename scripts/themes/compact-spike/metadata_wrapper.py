#!/usr/bin/env python3
"""Use a private copy of FerroUI.Base with the optional metadata fast path omitted."""
import os
import sys
from pathlib import Path

compiler, *args = sys.argv[1:]
for index, arg in enumerate(args):
    if arg.endswith('src/FerroUI.Base/lib.rs'):
        args[index] = str(Path(os.environ['SPIKE_BASE_COPY']) / 'lib.rs')
os.execvp(compiler, [compiler, *args])
