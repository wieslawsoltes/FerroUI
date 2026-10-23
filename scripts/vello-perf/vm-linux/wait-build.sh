#!/bin/bash
# In the Linux virtual machine: waits for build.sh and prints its errors or its end.
until grep -qE "Finished|^error" /tmp/vbuild.log; do sleep 15; done
grep -E "^error|Finished" -A10 /tmp/vbuild.log | head -40
