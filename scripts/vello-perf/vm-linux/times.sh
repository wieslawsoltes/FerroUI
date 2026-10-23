#!/bin/bash
# In the Linux virtual machine: the frame times and panics of every run of run.sh.
for f in /tmp/vrun-*.log; do
  echo "== $f"
  grep -E "frame times of|panicked at" "$f" | cut -c1-200
  grep -A3 "panicked at" "$f" | head -4 | cut -c1-200
done
