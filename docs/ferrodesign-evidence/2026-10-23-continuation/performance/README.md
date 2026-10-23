# Current native engine performance: 10,000 nodes

The existing release-mode performance example completed successfully on the
shared local execution host. All paired document/history, component identity,
Undo/Redo and serialization round-trip assertions passed. The complete results
are in [native-engine-10000.json](native-engine-10000.json); the exact source,
toolchain, host scope and result digest are in [provenance.json](provenance.json).

This measures native CPU engine work. Browser event dispatch, rendering,
compositing and frame rate are outside the measurement. Other work on the shared
host can affect timing; this is one captured execution of the existing example.

## Source and command

The checkout was `08e29e4f530c0ae65f6e2ac072f381605e23ed0c`, with repository tree
`99802b5a5b6db4d40109baee75e816303be056f0`. Its core source tree,
`3c91aeeed4ff3d22eff9eae1f63fd6aef1cd3f24`, exactly matches production checkpoint
`1637a093723a4f899b7f4e371fd53f0b91914a98`. The core, workspace profile and lockfile
were checked for source changes before and after the run. Rust/Cargo 1.99.0 was
used with one build job and incremental compilation disabled. The release build
took 1 minute 43 seconds; the program exited with status 0.

```sh
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 \
  cargo run --locked --offline --release -p ferrodesign-core \
  --example performance --jobs 1 -- 10000
```

## Scoped editing and history

Both paths operate on identical 10,000-rectangle documents in one process.
Nudge and gesture pairs alternate measurement order; Undo and Redo measure the
scoped path first. The reference performs full-document operations;
the scoped path retains changes to the affected node. The example asserts
matching authored state and exact history round trips after the paired work.

| Operation | Samples per path | Scoped p95 | Full-document reference p95 |
| --- | ---: | ---: | ---: |
| One-node nudge | 48 | 0.1250 ms | 34.9949 ms |
| Undo | 48 | 0.0878 ms | 16.7329 ms |
| Redo | 48 | 0.0813 ms | 8.6996 ms |
| One gesture update | 288 | 0.00228 ms | 11.7940 ms |
| Complete gesture, including 24 updates | 12 | 0.1658 ms | 266.6481 ms |

These comparisons describe the two implementations in this fixture. The raw
file retains medians, maxima, totals and separate gesture begin/update/commit
measurements for inspection.

## Queries, layout and serialization

All 10,000 point queries returned a hit in 22.94 ms total. The
1,000 viewport queries took 29.91 ms total. Each query operated on the same
10,000-node document.

| Work | Samples | Median | p95 / maximum |
| --- | ---: | ---: | ---: |
| Dense layout: 9,999 text children, three quarters using fill sizing | 5 | 13.69 ms | 14.50 ms |
| Layout pass with no auto-layout containers | 5 | 1.99 ms | 3.82 ms |
| Compact autosave serialization, 12,802,055 bytes | 5 | 26.89 ms | 69.06 ms |
| Portable pretty serialization, 20,152,206 bytes | 5 | 51.18 ms | 98.15 ms |
| Parse, validate and index the portable document | 3 | 91.66 ms | 110.78 ms |

For these small sample counts, the example's p95 sample selection is the
maximum observed value. These rows do not establish stable tail-latency bounds.
The dense layout fixture retains 448 bytes of text per child and reflows nearly
all children after each parent width change. Both serialized formats were
checked for equivalent document round trips.

The component fixture also verifies matching instance IDs and exact history
restoration across variant, exposed-text and visibility edits. Its editor path
retains whole-document history snapshots: the measured per-operation p95 values
were 42.27 ms for variants, 43.93 ms for text and 53.72 ms for visibility, with
eight samples each. The complete planned-change, clone-reference and history
measurements remain in the raw result. This identifies work that remains more
expensive than the scoped one-node operations above.
