#!/usr/bin/env python3
"""Compile the timing driver with the Cargo dependency flags of ResourceBenchmarks."""
import os
import sys
from pathlib import Path

compiler, *args = sys.argv[1:]
if 'compact_template_bench' in args:
    index = args.index('tests/FerroUI.ResourceBenchmarks/program.rs')
    args[index] = str(Path(__file__).with_name('template_bench.rs').resolve())
os.execvp(compiler, [compiler, *args])
