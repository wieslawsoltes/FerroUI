# Shared runtime code for the editable WinUI size budget

The next release target is **10,000,000 bytes for the complete stripped Linux
x86-64 executable**, including the editable WinUI runtime and all theme
documents. The latest independently imported size checkpoint in this report is
**27,101,920 bytes** at `ad926b4d`, leaving **17,101,920 bytes** to remove.
Its matched size and targeted correctness gates passed; full release acceptance
remains blocked by the boxed-style performance regression described below.
The earlier **27,406,416-byte** stage5 checkpoint remains documented in
[the architecture report](compiled-theme-rust-architecture.md). Each shipping-size
comparison below uses fresh profiles and complete executable measurements;
fixture savings and compressed distribution sizes do not establish progress
against the shipping budget.

## Completed CI size comparisons

### Shared-runtime cohort

[CI run 41815903284](https://github.com/wieslawsoltes/FerroUI/actions/runs/41815903284)
completed successfully on 2026-11-04 using the frozen source pair below and a
fresh original-workload profile for each revision. The exact console result is:

| Complete stripped executable | Bytes | SHA-256 |
|---|---:|---|
| Matched reference, `be91062d` | 27,437,360 | `7d7c2b3a4efc863f3bbd8f2139eeb61f2bfb65f07083674a43090f56fe5a844e` |
| Shared-runtime candidate, `d2d2f83d` | 27,141,328 | `84b2f636324f3dc2222f39d64c4e36dd3c8103efd15e3d3fc5151d1e87b8c2a6` |
| **Saving against the matched reference** | **296,032 (1.07894%)** | |
| **Remaining to 10,000,000 bytes** | **17,141,328** | |

The target is not met. The fresh CI reference is a separate executable from the
retained 27,406,416-byte stage5 image. The main file-backed section changes show
both the code reduction and the costs that offset it:

| Section | Candidate minus matched reference |
|---|---:|
| `.text` | −359,168 bytes |
| `.data.rel.ro` | +42,712 bytes |
| `.eh_frame` | +22,828 bytes |
| `.eh_frame_hdr` | +3,560 bytes |
| `.gcc_except_table` | −8,108 bytes |
| `.relr.dyn` | +672 bytes |

Both arms completed all nine archive cases, 47 registry tests, the compact-unwind
check and 156 template activations, including the added resource value `31`.
These gates do not establish the separate full property/binding, receiver,
getter and animation comparisons or throughput parity. The
[CI checkpoint](benchmarks/compiled-theme-shared-runtime-ci-linux-x86_64.json)
preserves the complete job log, exact console summary and original artifact
identity. The first connector retrieval rejected the 1,261,081,055-byte evidence
ZIP because its per-file limit is 536,870,912 bytes. Independent artifact import
and production runtime acceptance remain pending at this checkpoint.

The subsequent [artifact import checkpoint](benchmarks/compiled-theme-shared-runtime-artifact-import-linux-x86_64.json)
records successful transport, reconstruction of the original ZIP with its
exact SHA-256, extraction of 35,532 files, and verification of 157 relocated
inputs. Both source snapshots contain 191 transported links; their resolved
contents match the recorded source graph. The actual binary hashes and all
16 recorded CI arm manifests were independently checked after extraction.
The separate production runtime comparison is still pending at this source
checkpoint.

### Completed production checks and the remaining regression

The subsequent [production runtime evidence](benchmarks/compiled-theme-shared-runtime-production-linux-x86_64.json)
completes that comparison for the exact imported `be91062d` and `d2d2f83d`
graphs. All 16 production companion links succeeded. Both arms passed 347
property tests (three timing cases ignored), all nine focused regressions,
eight receiver checks, 88 getter outputs, 44 animation tests and all four
keyframe checks. The unchanged shipping executables passed local loader checks
and the nine archive cases. Input identities were rechecked after the run.

The bounded PGO audit accepted all six selected root functions, including
notification entry count 14,164 on both arms and two retained target promotions.
This rules out the previously observed loss of those root keys; it is not an
audit of every profile record or every specialized binding body.

**Performance acceptance is blocked by boxed style delivery.** The following
ratios are paired candidate/reference medians with their original 95% bootstrap
intervals. Each boxed-binding cohort uses 31 measured pairs and three warmups;
the CPU7 confirmation uses the same executables and unchanged workloads.

| Boxed binding lane | CPU6 ratio [95% interval] | CPU7 ratio [95% interval] |
|---|---:|---:|
| Style notification | 1.13077 [1.12656, 1.13727] | 1.17237 [1.13761, 1.23521] |
| Style implicit conversion | 1.11648 [1.10424, 1.12421] | 1.14891 [1.13131, 1.24773] |
| Style `BindingNotification` | 1.09477 [1.09028, 1.11454] | 1.15771 [1.09146, 1.20733] |
| Style installation | 1.21471 [1.21129, 1.21830] | 0.99091 [0.95141, 1.01947] |

The CPU7 reference/reference control for style notification is 1.00080
[0.99467, 1.03063]. Other control lanes show shared-host/order variability;
the cohorts are not pooled and no universal control correction is applied.
The installation flag remains recorded but did not reproduce on CPU7. The
typed mixed style lane uses `LightweightSubject<i32>` on a derived host; its
near parity does not establish parity for `bind_property_untyped` with boxed
sources on a root `FerroObject`.

The evidence preserves all 44 CPU6 companion metrics, 20 startup diagnostics,
the CPU7 confirmation and the control. CPU6 startup-to-resource-readiness
ratios are 0.98631 [0.97510, 0.99829] for embedded resources, 0.99418
[0.98037, 1.00444] for scalar edits, 0.99635 [0.98590, 1.01578] for structural
v3 edits and 0.99763 [0.98380, 1.00822] for the v1 interpreter fallback.
This measures typed Button `ControlTheme` readiness, not construction or paint.

Allocation counts match. Styled binding installation requests 16 fewer bytes
per operation. Animation attachment keeps 34 allocation requests per sample
but increases requested bytes from 6,848 to 6,864. These costs are retained
separately from timing and from the measured full-executable size saving.

The source follow-up is investigating typed metadata lookup before allocation
and hot helper inlining. These remain hypotheses/corrections under evaluation;
the production result above does not establish their performance or accept
the `d2d2f83d` runtime as throughput-equivalent.

### Physical attribution and the next source changes

The [fresh size census](benchmarks/compiled-theme-shared-runtime-size-census-linux-x86_64.json)
reconciles the file delta exactly: code and unwind shrink by 340,888 bytes,
read-only tables and other data add 43,456 bytes, and file-layout gaps add
1,400 bytes. There are 445 additional physical function ranges, despite the
smaller aggregate code; their unwind search entries account for the 3,560-byte
index increase. These are complete physical counts, with aliased ranges counted
once.

The largest focused reduction is in styled binding installation: the selected
route and creation families shrink from 407,583 to 162,858 bytes, including
unwind. The two typed boxed-input delivery bodies, paired across their old and
new names, change from 495,734 to 497,109 bytes. These diagnostic family groups
overlap other category totals and must not be summed.

Two further cold source changes were frozen at
[`ab595dd9`](https://github.com/wieslawsoltes/FerroUI/commit/ab595dd9646d2e2e708f833bc17b7d1c4c5af4ad):

- Both `ferro_class!` forms keep the same static `OnceLock` and an initialized
  `get()` fast path. The slow path receives a `fn() -> VTable`, so its generic
  initialization machinery depends on the table type instead of each class's
  distinct builder function-item type. The helper preserves ordinary OnceLock
  synchronization and retry behavior.
- Direct properties construct `DirectPropertyBase<T>` from owner descriptors.
  The typed owner wrappers retain their getter and setter, allocation,
  accessor/routes installation and registration. Metadata merge, freeze and
  temporary-drop ordering are retained.

The preceding census gave a 142,598-byte gross vtable-initializer envelope,
before retaining builder code and the new shared helper. Named direct-property
construction families occupied 31,729 bytes, plus copies embedded in other
functions. Those envelopes were experiment bounds, not predicted savings.

### Completed cold-initialization cohort

[CI run 41830662707](https://github.com/wieslawsoltes/FerroUI/actions/runs/41830662707)
completed the separate `d2d2f83d` to `ab595dd9` comparison on 2026-11-04.
Each arm trained a fresh profile with the same original 13 workloads and built
the complete 54-library target graph in the same checkout, consumer and target
paths. The
[cold-initialization production evidence](benchmarks/compiled-theme-cold-initialization-production-linux-x86_64.json)
preserves the original comparison byte for byte, the artifact and import
identities, independent hashes of all 16 arm manifests and both executables,
and a complete ELF file accounting.

| Complete stripped executable | Bytes | SHA-256 |
|---|---:|---|
| Fresh matched reference, `d2d2f83d` | 27,141,408 | `e49128877da4d0e0ff9683313b87e5542e416a2c103a696335947f63510a718b` |
| Cold-initialization candidate, `ab595dd9` | 27,065,600 | `9725868b49f59c6d8cdf02785adb5fbabd6be340d84557271d7d1d3e7149c52e` |
| **Saving against this matched reference** | **75,808 (0.27931%)** | |
| **Remaining to 10,000,000 bytes** | **17,065,600** | |

The new `d2d2f83d` reference is **80 bytes larger** than the same-source
27,141,328-byte image in the preceding cohort. It has a different fresh profile
and binary identity. The 75,808-byte saving uses the new cohort's own reference;
the 80-byte difference is retained as observed cohort variation and is not
attributed to the source change or silently removed by combining the cohorts.

| File component | Candidate minus matched reference |
|---|---:|
| `.text` | −48,976 bytes |
| `.data.rel.ro` | −14,264 bytes |
| `.eh_frame` | −11,728 bytes |
| `.eh_frame_hdr` | −2,120 bytes |
| `.gcc_except_table` | +1,912 bytes |
| `.relr.dyn` | −224 bytes |
| `.rodata` | +168 bytes |
| `.data` | +8 bytes |
| File-layout gaps | −584 bytes |
| **Complete file** | **−75,808 bytes** |

Allocated file-backed sections account for −75,224 bytes. Nonallocated section
contents and ELF/program/section headers are unchanged in size; actual gaps
between file intervals account for the remaining −584 bytes. `NOBITS` reserves
and virtual-memory padding contribute no section file bytes. This reconciles
the stripped executable itself rather than summing overlapping symbol families.

Both arms passed all nine archive cases and rejected the impossible-resource
negative control. Each passed 47 registry tests, the production compact-unwind
check, and 156 real template activations across Button, ScrollViewer,
NavigationView and Slider. The structural archive explicitly resolved
`AdditionalEditProbe` to `31`. These are size and targeted correctness results.
The paired property/binding, receiver/getter, animation, allocation and startup
acceptance work for the selected final graph remains separate. The earlier
boxed-style delivery regression is still a release blocker, and the 10 MB
target is not met.

### Boxed-binding correction under validation

The reproduced boxed-delivery regression now has a separate two-file source
correction. `BoxedBindingEntry::new<T>` reads the typed property's validation
metadata before `Rc::new_cyclic`, then passes the boolean to the shared state
constructor. The temporary metadata `Rc` drops before entry allocation, matching
the baseline's construction and panic ordering. This does not establish an
explanation for the CPU6-only installation flag.

`BindingEntryState::bound_frame`, `BindingEntryState::set_validation` and
`BoundFrame::publish` now carry ordinary `#[inline]` hints. Their bodies are
unchanged. The owned input remains in `Observer::on_next`, `receive` borrows it,
and the upgraded entry drops before the input on return and unwind. No cache,
subscription, `RefCell` borrow or explicit clone/drop boundary changes.

The measured reference QA's native `BindingEntry<i32>::set_value` inlines all
three helpers. The candidate shipping image calls them out of line, but it is a
different LTO consumer. A diagnostic reference replay matches every allocated
code/data section and the load geometry; its GNU build-id and ELF section-table
bookkeeping differ. Its symbols therefore support scoped reference instruction
attribution, not full-file equality. The original stricter comparison remains
failed, and the candidate replay received SIGKILL before producing an image.

This is a bounded source experiment, not a claim that throughput is restored.
Inlining can increase application code and unwind size, and it does not address
the separately observed `TypeId` promotion difference. The existing production
correctness, allocation and paired delivery gates must pass before acceptance.
[Patch, review and retained diagnostic evidence](benchmarks/compiled-theme-boxed-binding-correction-review-linux-x86_64.json).

### Comparing subsequent production graphs

The boxed correction is committed at
[`ad926b4d`](https://github.com/wieslawsoltes/FerroUI/commit/ad926b4d10a68465b5bbc9fe203fa908dc785f59).
Its separate [CI comparison](https://github.com/wieslawsoltes/FerroUI/actions/runs/41837864124)
completed with `ab595dd9` as reference, using fresh original-workload profiles
for both complete applications. The original artifact has now been independently
downloaded, reassembled, imported and verified. The
[boxed-correction production evidence](benchmarks/compiled-theme-boxed-binding-correction-production-linux-x86_64.json)
preserves the original comparison byte for byte and verifies all 16 arm
manifests, source graphs, profiles, executable identities and targeted checks.

| Complete stripped executable | Bytes | SHA-256 |
|---|---:|---|
| Fresh matched reference, `ab595dd9` | 27,065,536 | `4fdf8011516fdcd7c98a906a649a781071c4a200b4efafcf65db75036e069504` |
| Boxed correction, `ad926b4d` | 27,101,920 | `0870efcdde1e90f29e87d4c2861a5dfa3fc65a0415e6c0d85d64f41501eaada9` |
| **Increase against this matched reference** | **36,384 (0.13443%)** | |
| **Remaining to 10,000,000 bytes** | **17,101,920** | |

Allocated file-backed sections add 34,216 bytes: `.text` +24,144,
`.data.rel.ro` +3,536, `.eh_frame` +8,468, `.eh_frame_hdr` +1,088,
`.gcc_except_table` −2,940, `.relr.dyn` +56 and `.rodata` −136.
Actual file-layout gaps add another 2,168 bytes; ELF/program/section headers
and nonallocated section contents are unchanged. The new reference is 64 bytes
smaller than the earlier cold-cohort `ab595dd9` candidate, with different fresh
profile and binary identities. That separate cohort difference is not assigned
to a source change or combined with the measured correction delta.

Both arms passed all nine archive cases and the negative control, 47 registry
tests, compact-unwind checks and 156 template activations, including the added
resource value `31`. The increase is a measured size cost; throughput recovery
is still pending. The metadata receiver-preparation and ControlTheme/style-region
changes retain separate actual-source correctness gates. Their optimized
shipping size and performance are outside this source pair.

The runtime importer now supports a derived arm selection across completed CI
imports. Each arm retains its original comparison, source, graph, profile and
tool identities. Relocations remain in separate origin namespaces because two
CI runs can use the same absolute build paths for different files. The derived
record does not pretend to be a newly matched build. Twenty read-only checks
passed, including rejection of altered identities, graph inventories, source
associations and ambiguous origins. The positive two-origin tooling test uses
two imports of the same old evidence, not an uncompleted new build.
[Tooling checks and execution plan](benchmarks/compiled-theme-runtime-cohort-tooling-linux-x86_64.json).

Receiver policies are explicit: the original `be91062d` reference clones exact
shared receivers, while `ab595dd9` and `ad926b4d` borrow them. The public callable
fixture keeps its recorded strong-count expectation for the chosen policy.
The full runtime runner retains the existing sixteen companion links and all
checks/timing protocols. Reusing an already verified reference executable is
a separate, explicitly recorded procedure through the individual producers;
historical timings are never substituted for fresh paired samples.

```sh
python3 scripts/themes/bench/run_winui_runtime_comparison.py compose \
  --reference-import /evidence/original/import.json \
  --reference-arm reference --reference-receiver-policy cloned \
  --candidate-import /evidence/corrected/import.json \
  --candidate-arm candidate --candidate-receiver-policy borrowed \
  --output /evidence/acceptance-selection
```

### Completed construction source-validation gates

The next construction changes have passed their separate actual-source
development gates, recorded in the
[construction source-validation evidence](benchmarks/compiled-theme-construction-source-validation-linux-x86_64.json).
These runs compile the actual repository source and tests; they do not measure
an optimized shipping executable or establish paired throughput.

Metadata preparation at `50d7caef` passed
[CI run 41850078862](https://github.com/wieslawsoltes/FerroUI/actions/runs/41850078862):
66 metadata tests, 16 object-smoke tests and 307 property tests, with no failures
or ignored tests. All three focused accessor oracles executed, downstream Base
and Controls checks passed, and the frozen source stayed unchanged. Getters
share receiver preparation while retaining the owned receiver and complete
callable/result expression in the original thunk. This keeps custom cloning and
temporary cleanup in their original scopes. Setter and static-accessor helpers
retain the checked argument and invocation order. The earlier failed cleanup
oracle and the later test-only function-pointer coercion failure remain in the
evidence; their results are not counted as passes.

The whole-factory ControlTheme prototype at `af8a87ff` passed
[CI run 41847594355](https://github.com/wieslawsoltes/FerroUI/actions/runs/41847594355):
two runtime cleanup tests, six source-grammar tests and 24 differential tests,
with two pre-existing differential cases ignored. All ten required new cases
executed. A fresh WinUI check generated 18 construction programs. Per-factory
operation and import tables share typed construction helpers; the caller owns
the local array in reverse declaration order so Rust retains the original
cleanup order. Only complete factories already using all six stored roles are
accepted, and unsupported factories keep their original generated Rust.

These source checks do not replace the full-size and production performance
comparisons. The two-role style-region extension has now passed a separate
combined actual-source gate. Its source is frozen at
[`dc93a16d`](https://github.com/wieslawsoltes/FerroUI/commit/dc93a16d88c8d8e6aedaa852f1106a856ab5993b),
including the validated metadata and whole-factory changes. Its executed
standalone grammar gate passed all 13 cases. Running the actual parsers over
the retained generated source accepts 18 whole factories first, then 34 regions
across 24 additional factories, with 1,217 instructions and 332 local slots.
Each region keeps unsupported operations in place, borrows one original parent,
and retains its reversed two-role local array in the caller's original scope.

The corresponding 24 enclosing native functions occupy 316,624 bytes, including
unwind and exception data. That is a gross scope bound: the regions cover only
parts of those functions, and instruction tables and shared helpers have their
own costs. It is not a predicted saving. The new actual-source gate requires
all 21 named regressions, exact runtime/grammar/differential summaries and fresh
WinUI generation with both program forms.
[Source, grammar, coverage and gate plan](benchmarks/compiled-theme-style-region-source-prototype-linux-x86_64.json).

[CI run 41855481313](https://github.com/wieslawsoltes/FerroUI/actions/runs/41855481313)
completed successfully against the exact combined source: two whole-program
runtime tests, three region runtime tests, six whole-program grammar tests,
seven region grammar tests and 25 differential tests passed. Two existing
differential cases remain ignored. All 21 required cases executed, including
interleaved cleanup, return-payload lifetime, panic/drop behavior, lazy imports,
parent-stack visibility and rejection of escaped region locals. Fresh WinUI
generation compiled with 18 whole programs and 34 region programs. The source
tree, tracked inputs and lockfile stayed unchanged.

The [combined source-validation evidence](benchmarks/compiled-theme-style-region-source-validation-linux-x86_64.json)
preserves the exact original validation record, verifies every command and
resource log and the generated WinUI file, and identifies all six integrated
source files. The production comparison is pinned to `ad926b4d` as reference
and `dc93a16d` as candidate. Both arms use the original 13-workload training,
features, compilation paths, native/link policy and complete editable executable.
Its measured size and runtime acceptance remain pending.

### Remote companion linking and subsequent local measurement

The boxed-correction artifact import and independent verification are complete,
as recorded above. Its performance remains unresolved.

The companion pipeline downloads the exact original release artifacts and
links all eight reference and eight candidate companions together on one CI
native environment, plus a separate `ab595dd9` QA companion for correction
attribution. It uses the existing fixtures, profiles and production producers,
without retraining or timing on that runner. The previous local-linked reference
is not mixed into this new cohort. Compiler, CRT and native-link inputs are
recorded around the sequence; the bounded PGO audit remains required.

A bounded bundle transports executable bytes, exact fixtures and original
records. Data-only import verifies their source/profile associations and makes
separate runtime views whose changes are limited to file locators. Local ELF
and loader proof precedes checks and fresh paired measurements. Shipping startup
remains a separate required gate. Thirteen importer checks passed using retained
real binaries and explicitly synthetic transport wrappers; these tests do not
claim a new production link or performance result.
[Tooling, independent review and frozen input contract](benchmarks/compiled-theme-runtime-companion-transport-tooling-linux-x86_64.json).

The first [companion CI run](https://github.com/wieslawsoltes/FerroUI/actions/runs/41855481325)
completed its 17 links, correctness and bounded PGO checks. Its data-only import
then correctly rejected an incomplete execution snapshot: the exporter had
updated the final native-input proof in memory but copied the persisted record
before saving that update. The separately uploaded final status confirms that
native inputs were unchanged; the incomplete original bundle and failed import
remain retained. The exporter now persists the completed native check before
copying the bundle. The strict importer is unchanged, and
[the corrected export run](https://github.com/wieslawsoltes/FerroUI/actions/runs/41860870363)
uses the same frozen origins, source revisions, profiles and production policy.
No paired companion performance result is accepted from the incomplete bundle.

### Fresh corrected-shipping startup comparison

The unchanged shipping images from the original `be91062d` reference and the
new `ad926b4d` correction have completed a separate fresh CPU6 startup cohort.
Their original CI builds are explicitly distinct origins. Recorded compiler,
native-tool, training-input, helper and policy identities match; both exact-byte
images were staged on one local filesystem and passed loader/ABI proof, the
impossible-font negative control and all nine archive cases before timing.

| Startup-to-resource-readiness case | Pairs | Candidate/reference median [95% interval] |
|---|---:|---:|
| Embedded resources | 101 | 0.996856 [0.992104, 1.003704] |
| Scalar v3 edits | 41 | 0.993226 [0.985424, 1.002816] |
| Structural v3 edits | 41 | 0.996330 [0.981400, 1.016628] |
| Structural v1 fallback | 21 | 1.002506 [0.987691, 1.014796] |

All four intervals include parity: no regression was detected in these retained
shared-host cohorts. Embedded readiness medians are 9.499002 ms and 9.476428 ms.
These are fresh alternating paired launches with the original warmup, archive
and bootstrap protocol. They stop at typed Button `ControlTheme` readiness;
control construction, first paint and complete runtime throughput remain outside
this measurement. The
[corrected-shipping startup evidence](benchmarks/compiled-theme-boxed-correction-startup-linux-x86_64.json)
retains all raw rows and internal diagnostics, original orchestration and plan,
native/compatibility evidence and independently rechecked identities. It does
not remove the earlier boxed-style delivery blocker or establish the pending
construction wave's performance.

## Source checkpoint

### Demand-rooted typed recoercion

The next property-system prototype is frozen at
[`bbff5971`](https://github.com/wieslawsoltes/FerroUI/commit/bbff597167c8fe72f815dba2daffe62e1a99a4de)
on `perf/demand-rooted-coercion`. Explicit `with_coerce`/`coerce` registration
stores the original callback together with its typed recoercion algorithm.
Options conversion and metadata cloning/merging transfer that pair without
naming the algorithm. This removes the unconditional reference to the heavy
two-value algorithm from every `EffectiveValueDyn` vtable specialization.

The private effective-value type declaration is unbounded so public Clone-only
metadata can name the algorithm. Constructors and property operations keep
their existing `PropertyValue` bounds. The algorithm still owns its values in
the original two call frames; only equality and the existing notification
functions are supplied as typed function pointers. `UncommonFields`, typed
payload storage and ordinary read/write/notification method bodies are unchanged.
There is no new allocation or unsafe cast.

The historical linked `coerce_value` and `set_and_raise_core_with_base` families
occupy 333,413 bytes, including unwind/exception data. This is a gross bound.
Registered coercer types still need their algorithms; standalone typed leaves,
indirect explicit-recoercion calls and one extra metadata/options function
pointer have costs. No saving or throughput result is claimed for this prototype.

Five new actual-source tests retain the old algorithms as their oracle. They
compare eight priority/equality configurations and every observed Clone,
equality, coercion, Drop and notifying panic point; reentrant reads/writes and
priority changes; Clone-only metadata and callback identity/lifetimes; callback
replacement during unwind; and NaN equality. The frozen gate also requires
307 property tests, 14 property-metadata tests, the registration identity test,
66 metadata tests and downstream Base/Controls checks. Its eight required
named tests must execute before the downstream check. The
[source review and plan](benchmarks/compiled-theme-demand-rooted-coercion-source-prototype-linux-x86_64.json)
are complete; compiler execution and production measurement remain pending.

### Earlier source history

The runtime and focused regression tests are committed through
[`6401e654`](https://github.com/wieslawsoltes/FerroUI/commit/6401e654c148a473f33358b79f5cf886d5f9d86b),
with test-only corrections through
[`d2d2f83d`](https://github.com/wieslawsoltes/FerroUI/commit/d2d2f83d606f547b4ebda2323a145b14ca8379ad).
The first shared-runtime production comparison froze that revision on the
existing PR branch. The completed cold-initialization comparison then measured
it against `ab595dd9`, as recorded above.
The reference revision is
`be91062df9bb018de78113c34a1967c60038f9b6`. The source phase does not merge current
main into the measured cohort.

### Boxed binding installation

Boxed local and styled bindings now share source subscription, completion and
installation code. Each binding still stores its value or fallback as its
original `T` in the binding allocation. The source observer holds a typed
`Weak<Value<T>>`, so delivery upgrades the typed owner directly and does not
introduce another dynamic call or box for each notification. The typed `T` and
`BindingValue<T>` source overloads retain their existing delivery paths.

Ownership boundaries are part of the implementation. `on_next` owns the input
`BoxedValue` and calls `receive(&value)`. On return and unwinding, the upgraded
binding drops before the input. An input destructor can dispose a lease and
reenter a source-retained observer; that observer must not recover a binding
whose last strong owner has already been released. The regression test delivers
the sole input owner directly to a retained observer, covering local and styled
bindings both with successful conversion and with a conversion panic.

Subscription guards retain their previous destruction order, before the source
and typed cache. Replacing a cached value releases the `RefCell` guard before
the old value's destructor runs. A cached fallback remains associated
with the binding across hiding, unsubscribe and resubscribe.

### Property conversion

Public setters keep the borrowed `&dyn Any` path, reserved-marker precedence
and exact-`T` branch. Implicit conversion lookup and ownership of its temporary
box move into a common cold helper; only the typed payload copy remains in the
small generic callback. That callback borrows stack state and does not allocate.
The converted owner drops before validation, coercion or publication, including
when a custom `Clone` panics.

Binding notification handling shares the error/marker/conversion policy while
preserving the snapshot order: error, error type, value presence, then payload.
No registry or notification borrow spans a user callback. The added tests cover
custom clone snapshots, overridden default metadata, reentrant notification
mutation, conversion temporary destruction, marker routing and panic cleanup.

### Metadata receivers

Generated instance invokers borrow exact `Ref<T>`, `Rc<T>`, `FerroList<T>`,
`FerroDictionary<K,V>` and `MediaCollection<T>` receiver arguments. A converted
receiver remains owned for the call through `Cow`. Declaration-time autoref
selection keeps arbitrary user-defined receiver types on the original owned
`Clone` route; that fallback returns `This` directly.

All eleven instance-member invocation forms call their original typed member
directly through `&This`. Public function-pointer signatures, argument
diagnostics and the `BoxedValue` shortcut are unchanged. Exact shared receivers
intentionally avoid the former temporary strong-reference increment; the tests
assert this ownership change explicitly. They also cover custom `Clone` side
effects, conversion ownership, callback panic cleanup and registry reentry.

### Animation lifecycle

Match transitions, subscription installation and teardown share common code.
A small typed subject retains `Rc<A>` and passes a borrowed runner to the common
state. It does not allocate a separate runner or clone the animator across a
custom `bind_animation` call. The subject retains its original field destruction
order, and constructor temporaries release before synchronous match delivery.

Visibility, detachment and base-property callbacks use weak private lifecycle
actions. The weak captures occur at their original points; the animator's
property is read only after the target upgrades successfully. Final fill still
precedes property subscription disposal, timer disposal, visibility disposal,
detachment disposal and clock stop, in that order. Each subscription is taken
from its slot immediately before disposal, without retaining a `RefCell` guard
across callbacks.

The per-frame observer, timeline arithmetic, interpolation, typed value fields
and publication remain typed. Erased weak handles make lifecycle callback
captures wider; allocation verification must report both allocation count and
requested bytes rather than infer equality from the source.

## Validation policy

The initial normal-library check passed with Rust 1.99.0 and LLVM 23.1.1, using
`markup-functions`. Independent source review found no API, ownership or
reentrancy blocker. These checks do not certify runtime behavior or release size.

The development Base library subsequently built successfully with debug
assertions, `hot-reload` and `markup-functions`. All 1,683 Base source hashes
remained unchanged during compilation. The library uses `opt-level=0`, 128
codegen units and bitcode emission; development companions use ThinLTO. This
combination bounds compilation memory while exercising the actual runtime
implementation. It is not the production optimization policy and supplies no
release-size or throughput result.

The completed development checks are:

| Check | Actual result |
|---|---|
| Property and binding correctness | 348 passed, 0 failed, 3 ignored; all nine new regressions and the debug hot-reload rollback case passed |
| Metadata receiver ownership and callbacks | 8 checks passed |
| Generated getters | 88 getter results and 352 associated argument-diagnostic assertions passed |
| Animation iteration, style and preview | 44 tests passed |
| Animation frame checksums | Normal, reverse, alternate and attachment modes passed |
| Animation attachment allocation observations | 16 rows, each with 34 allocation requests and 6,960 requested bytes; no reference parity claim |
| Property registration | 47 tests passed |
| Ordinary unwinding | Typed panic payload, nested destructor order, property-store reuse and binding reuse passed |

The animation allocation rows report requested bytes, not retained memory or
allocator overhead. The unwind check uses ordinary development unwind tables;
the separately linked production compact-unwind check remains required.

[Development evidence](benchmarks/compiled-theme-shared-runtime-development-linux-x86_64.json)
records the actual logs, immutable fixture identities and the bridge from all
20 changed runtime files to the successful Base build. It also preserves the
earlier unsuccessful attempts. The final property fixture corrects two
assumptions about unchanged behavior: a local write masks an existing style
binding without unsubscribing it, and `set_current_value` applies coercion
without invoking validation. A template-priority lease now exercises the real
unsubscribe/resubscribe transition. Conversion-temporary destruction assertions
remain inside the validation, coercion and direct-setter callbacks.

The remaining gates use real Base libraries and separately recorded fixtures:

| Area | Evidence required |
|---|---|
| Property and binding correctness | Public property suite plus the nine new binding/setter regressions; private-only exclusions listed explicitly |
| Metadata | Eight receiver cases, actual generated getters and diagnostics; exact shared receiver counts distinguished from the reference |
| Animation | Public iteration and style/preview cases, lifecycle ownership, frame workloads and attachment allocation count/bytes |
| Registry and unwinding | Public registration suite; typed panic payload, nested destructor order and reuse of property/binding state under compact unwind |
| Editable theme | Embedded archive, installed archive, scalar edits, structural edits and all three supported archive encodings |
| Release size | Entire shipping executable, its SHA-256 and allocated section census |
| Performance | Paired held-out workloads against the matched reference, after compilation stops, with raw samples and CPU affinity recorded |

The release uses the existing thirteen training launches. Held-out correctness,
allocation and timing fixtures do not enter the profile. The optimized shipping
entry point must match `scripts/themes/bench/winui.rs`. Each revision receives a
fresh whole-graph profile, and both arms use the same toolchain, features and
link policy: size optimization, final FatLTO, safe ICF, packed relative
relocations, compact unwind and panic unwinding.

## Reproduction helpers

The existing [profile driver](../scripts/themes/bench/profile_winui.py) remains
the source of the release training and link policy. The
[toolchain bootstrap](../scripts/themes/bench/bootstrap_toolchain.py) installs
checksummed official components into an explicitly selected benchmark work
directory. It records compiler and LLVM identities without changing a system
toolchain.

The [isolated archive-tool producer](../scripts/themes/bench/prepare_portable_packer.py)
copies the production portable codecs, XamlX and CLI byte for byte. Its small
parser facade retains the exact lexical parser method and both namespace
compatibility mappings. This lets archive preparation compile without the
framework's Base library. The resulting tool reproduced all eight retained
archive byte hashes, including each supported scalar/structural encoding;
[the executed evidence](benchmarks/compiled-theme-portable-packer-linux-x86_64.json)
records the dependency lock, source copies, compiler and output identities.
Only that tool executable is supplied through `--packer`. The application still
compiles the original Loader and Base packages. The validation edit now requires
all three normal-density font definitions in `Generic.xaml`, matching the
original training edit, while Compact edits still require exactly one anchor.

The [whole-graph replay driver](../scripts/themes/bench/rebuild_winui_pgo.py)
executes that policy for one frozen revision. It preserves the original thirteen
training launches and verifies the three training-source hashes and shipping
entry-point hash. Cargo's retained ThinLTO profile makes library dependencies
emit bitcode; the recorder verifies that actual compiler option. Final shipping
code uses the original `opt-z`, FatLTO and one-codegen-unit link policy. A
separate small PGO interoperability check establishes that profile counts can
reach an upstream bitcode library; it does not predict the full application's
memory use or performance.

Each revision has a separate result directory containing immutable source,
training inputs, dependency libraries, manifests, compiler records and its own
profile. The compiler, native driver and LLVM tools are identified by content
hash. Both revisions use the same disposable compilation checkout, consumer
directory and target directory, preventing an accidental path-policy change
between arms. Extracted source snapshots remain available after the compilation
checkout changes revision.

For example, with a clean frozen checkout and the checksummed toolchain already
installed:

```sh
python3 scripts/themes/bench/rebuild_winui_pgo.py \
  --source /path/to/measurement-checkout \
  --work-dir /path/to/evidence/candidate \
  --shared-work /path/to/shared-build \
  --toolchain-prefix /path/to/rust-1.99.0 \
  --cargo-home /path/to/cargo-cache \
  --cpus 4

python3 scripts/themes/bench/collect_winui_pgo.py \
  --build /path/to/evidence/candidate/build.json \
  --attribute
```

Choose a CPU permitted by the host. Run
[artifact collection](../scripts/themes/bench/collect_winui_pgo.py) before
switching the disposable checkout to the reference. It verifies the source,
profile and library association, copies generated XAML/FXB/FXP artifacts, and
relinks preserved native objects into a symbol-bearing companion. Every
allocated section except the build-ID note, and the executable entry point,
must match the shipping binary before that companion can supply attribution.
The shipping executable's bytes remain unchanged. Reuse the same compilation
paths for the reference, with a fresh reference result directory and a freshly
trained reference profile.

The [comparison orchestrator](../scripts/themes/bench/run_winui_release_comparison.py)
performs this sequence for both revisions and then checks nine archive cases,
47 registration tests, compact unwinding and all 156 template activations
against each real production graph. Its
[CI workflow](../.github/workflows/winui-release-size.yml) provides a separate
runner when local shared-memory pressure prevents the full build. Manual inputs
select exact revisions; the PR branch's automatic trigger is limited to changes
to that workflow file. The job has read-only repository permissions and uploads
evidence on success or failure. This comparison does not yet execute the full
paired property, binding, metadata and animation performance matrix; its final
report keeps that acceptance work explicitly pending.

The [production runtime comparison](../scripts/themes/bench/run_winui_runtime_comparison.py)
continues from a downloaded, completed CI evidence bundle. Its `import` command
verifies the original source, compiler, dependency, archive and profile identities
and writes a separate map from the recorded paths to their local copies. It
does not rewrite the CI reports. Dependency inventories must describe the exact
directories supplied to rustc; the property companion selects the `bitflags`
archive named by the actual Base compiler record.

The `run` command first proves the local Rust/LLVM and target sysroot identities
against the recorded distribution, then checks whether the imported host
procedural macros can load. It links both sets of production companions before
running the property, binding, receiver, getter, animation and allocation checks.
Both revisions use the current regression fixtures; only the reference receiver
fixture retains the original temporary strong-reference count. A failed gate
prevents timing. The `--checks-only` option runs these correctness and allocation
checks while explicitly leaving performance pending.

Before paired measurements, the
[bounded profile audit](../scripts/themes/bench/verify_winui_profile_acceptance.py)
inspects at most two Base bitcode members per revision. It resolves six current
notification/root-callback symbols, queries their exact current profile keys,
and requires genuine optimized function-entry metadata. An accepted zero-entry
validation callback is distinct from missing metadata. Historical entry counts
are not thresholds: inlining can change those counts. Root-handler target
promotions are recorded as diagnostics; this audit does not replace the paired
workloads or establish final native-code layout.

For example, after extracting the complete workflow artifact into a fresh
directory and installing the matching checksummed toolchain:

```sh
python3 scripts/themes/bench/run_winui_runtime_comparison.py import \
  --evidence /path/to/downloaded/evidence \
  --output /path/to/import

python3 scripts/themes/bench/run_winui_runtime_comparison.py run \
  --import-manifest /path/to/import/import.json \
  --toolchain-prefix /path/to/rust-1.99.0/install \
  --toolchain-manifest /path/to/rust-1.99.0/installation.json \
  --cpu 4 \
  --output /path/to/runtime-comparison
```

Choose an allowed CPU and new output directories. The existing producers retain
their workload sources, allocation counters and 31-pair timing policies; these
runs do not train a profile or rebuild Base. Startup is a separate cohort using
unchanged CI-linked executables staged locally. It requires host ELF-loader and
symbol-version compatibility plus the editable-archive smoke checks. If that
compatibility cannot be proved, startup remains pending even when locally linked
runtime companions can run. Preparing these helpers supplies no new executable
size, production correctness or performance result.

The [executed tooling preflight](benchmarks/compiled-theme-production-gate-preflight-linux-x86_64.json)
retains the local distribution/loader self-check and the profile parser's real
tiny-compiler input plus negative controls. It verifies 18 recorded tool
identities and the installed union of 27 target libraries, 27 metadata files
and three shared libraries. The earlier incomplete component-inventory check
remains in the record. These are preparation checks, separate from accepting
the completed CI graphs or measuring the application.

The [template validation producer](../scripts/themes/bench/prepare_template_validation.py)
retains the four original control factories, loader paths, font assertions and
visual-tree expectations. Its structural case additionally requires the edited
archive's `AdditionalEditProbe` resource to equal `31.0`. The complete release
gate requires 156 activations across embedded, scalar-edit and structural-edit
archives. Preparing the fixture does not count as executing that gate.

The [runtime validation driver](../scripts/themes/bench/validate_runtime_release.py)
materializes retained public registry/unwind fixtures with verified source
hashes, links them against an explicitly supplied real Base library, builds the
editable archive cases, and records paired startup samples. Its subcommands are
`fixtures`, `registry`, `unwind`, `archives`, `compat` and `startup`. `unwind`
requires the matching production profile; fixture binaries never count toward
the shipping executable budget. Startup measures typed Button `ControlTheme`
readiness and records internal create/lookup clocks separately. It instantiates
no Button control and does not measure the first painted frame. The archive
check first rejects an impossible expected resource value, so a probe that
omits editable archive support cannot pass merely by ignoring its arguments.
