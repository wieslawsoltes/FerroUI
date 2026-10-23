# Incomplete sequential browser run at c7

The **browser job was canceled with failed and incomplete coverage** for
`c7dbf26f1e184fc1113f9c96fcdc1dd0e2627834`.
[Run 39955698269, browser job 120423823862](https://github.com/wieslawsoltes/FerroUI/actions/runs/39955698269/job/120423823862)
started at 12:52:07 UTC on 2026-10-23 and completed with the API conclusion
`cancelled` at 14:52:44, after 120 minutes 37 seconds. Its configured job
limit was 120 minutes and `cancel-in-progress` was false. The interaction
step ran from 13:03:10 to 14:52:31; the subsequent text-pan benchmark was
skipped. The final artifact upload succeeded.

## Observed progress

The log shows continued WebGL2 scenario work close to cancellation. It does
not support a diagnosis of a stall in tracing or browser cleanup.

| Recorded event | UTC time |
| --- | --- |
| Software2D first scenario, vector network | 13:03:27.5065240 |
| Software2D final scenario started, bounded history | 13:09:07.4477772 |
| WebGL2 first scenario, vector network | 13:13:18.1302430 |
| WebGL2 local styles started | 14:23:32.6559379 |
| WebGL2 angular paints started | 14:35:50.4304090 |
| WebGL2 grid styles started | 14:39:56.6468159 |
| WebGL2 guide bindings started | 14:51:00.8057773 |
| Guide screenshot timeout logged; guarded retry began | 14:52:25.8314635 |
| Interaction step reported cancellation | 14:52:31.2096703 |

The retained [guide capture record](guide-capture.json) proves that the retry
also completed: its first 30,000 ms attempt failed after 30,053 ms, and its
60,000 ms retry passed after 1,652 ms. The record and PNG have a ZIP member
timestamp of 14:52:26, with two-second timestamp granularity. The actual pixel
and unchanged-document/view checks passed. The next native dialog-close,
history checks and later guide operations are not certified by this capture;
the exact operation active at interruption is unavailable.

Software2D logged starts for all 38 registered scenarios. It recorded failures
in polygon corners, the second paragraph-editing entry's caret-position check
(`caretIndex` was 1 instead of 3 after Control+Home and two ArrowRight presses,
before Enter at `paragraphs-browser.mjs:214`), and guide movement. WebGL2 logged 32 starts,
with polygon-corner and first-preedit paragraph failures. Guide bindings was
unfinished. Region paints, network bucket, compact autosave, prototype input,
prototype motion and bounded history were not reached in WebGL2.

The artifact contains the completed Software2D trace, but no WebGL2 trace,
final `report.json`, or final `index.html`. Start markers and the absence of a
failure message do not substitute for a completed report or certify all
assertion counts. The previously retained c549 results and later source fixes
remain separate evidence.

## Timing limits

The WebGL2 intervals from local styles to the next scenario and from grid
styles to the next scenario were 737.774 and 664.159 seconds. These intervals
include normal session-boundary, scenario and capture work; they are not
isolated feature timings.

[Capture timings](capture-timings.json) preserve each retained capture member's
hash and attempt durations. The 64 WebGL2 capture records contain 69 attempts,
including five successful guarded recoveries, totaling 1,461,816 ms of recorded
attempt duration. The 69 Software2D records contain 69 attempts totaling
11,022 ms. These are retained capture-operation measurements, including their
diagnostics, rather than isolated compositor, application CPU or hardware GPU
measurements. They do not explain all elapsed scenario time.

The supported conclusion is that this sequential job continued doing slow
WebGL2 work and did not complete its requested coverage within its job budget.
The underlying browser, renderer or host cause is not established here.
Splitting the CI workload can preserve coverage within bounded jobs; it does
not turn this result into a performance or visual-parity pass.

## Build cache result

This run **did exercise the new full-workflow cache bridge**. It restored the
independent browser-build cache ending in `c549f6bc…`, restored 10,464 matching
source timestamps, and skipped five changed inputs. The browser build step
took 9 minutes 54 seconds. It then recorded 10,501 source timestamps and
completed its early build-cache save at 13:02:53, before browser interactions.
The exact keys, lines and metadata are retained in
[summary.json](summary.json), [source-mtime-restore.json](source-mtime-restore.json)
and [source-mtime-snapshot.json](source-mtime-snapshot.json). Successful cache
reuse is independent of the failed/incomplete correctness result.

## Preserved evidence

- [job.json](job.json) contains the terminal browser job and every step.
- [run.json](run.json) preserves the workflow-run response as received. At
  14:59:19 UTC this endpoint still reported `in_progress`, even though both
  job responses were completed. That observation is retained without replacing
  it with an inferred workflow conclusion.
- [browser.log.gz](browser.log.gz) losslessly preserves the complete decoded
  job log: 139,023 bytes, SHA-256
  `1c60e820f24aaebc381ba3e86b9ddc01b5980f70925ecf70c9930dd2710c830a`.
  Decompression was checked against the original bytes.
- [artifact-api.json](artifact-api.json) and
  [artifact-members.json](artifact-members.json) identify artifact
  `12510460715` and its complete 408-member set. The downloaded
  146,122,436-byte ZIP matches the API SHA-256
  `1c06907bf00956f5b4543fa45b917e5acac43577f28a02c2332bcaad1e4e8f53`.
- [summary.json](summary.json) contains the complete observed scenario-start
  timeline, original failure lines, missing coverage, log hashes and exact
  source blob/content hashes.

The full ZIP is retained outside the repository. Only small JSON/log members
were extracted; their CRCs were verified during reading. No trace was expanded,
no browser or compiler was launched for this analysis, and no scenario,
assertion threshold or job timeout was changed.

