# Shared Rust implementations for the editable WinUI runtime

This continuation reduces repeated registration, metadata, compiler and adapter code while retaining the existing typed public APIs. The work changes which details are represented as data and which require a Rust specialization. Property values, custom validation and coercion, object projections, boxing, comparison and ownership still use their original concrete types.

## Stage5 executable and validation checkpoint

Stage5 adds a focused weak-reference conversion change to the broad shared-implementation work described below. Its complete Linux x86-64 executable is **27,406,416 bytes**, SHA-256 `91cc98828f5ba54dd9c465e7406f198fe44560c0a087f65eb948038a757ffb90`. It saves **680,808 bytes** against the retained verified release. All prescribed local correctness checks, native-code audits and runtime comparisons are complete. The recorded cohorts support proceeding, with one original short CPU7 mixed-property slowdown flag that was not reproduced in a separate longer controlled follow-up. This is not a universal zero-regression or formal equivalence claim. The retained release remains the preserved comparison baseline.

| Executable or target | Bytes | Relationship to stage5 |
|---|---:|---:|
| Original ordinary release | 54,577,848 | Stage5 is 27,171,432 B smaller (**49.784725847%**) |
| Retained verified compact-unwind release | 28,087,224 | Stage5 is **680,808 B smaller** |
| Matched rebuild used for runtime comparisons | 28,089,456 | Stage5 is 683,040 B smaller |
| Stage4 source cohort | 27,403,616 | Stage5 is 2,800 B larger; the source and fresh profile both differ |
| Stage5, freshly profiled candidate | **27,406,416** | Complete editable executable; prescribed local validation complete |
| Half of the original ordinary release | 27,288,924 | Stage5 is **117,492 B above** this threshold |
| Historical further-30% target | 19,827,298 | Stage5 is **7,579,118 B above** this target |

The completed runtime comparisons support only their measured workloads. They do not meet either remaining size target or retroactively change the failed historical cohorts. The broad source changes cover inherited dispatch, property/nullable/reference registration, static interfaces, typed bindings, XamlX transforms and decoding, and native resource emission. The linked-size census covers all linked categories; it does not mean that every repository source file was rewritten.

### Current linked sections and resource inventory

The stage5 inventory retains all 894 FXB blocks byte-for-byte, totaling 180,091 bytes, and the complete 296,347-byte FXP archive remains byte-identical and occurs once in read-only data. Generated Rust is 4,879,766 bytes, down 81,898 bytes from the matched reference's 4,961,664 bytes. No archive block is missing or added. The resource representation remains editable; this reduction does not remove the interpreter or unwind support.

| File-backed section group | Retained release | Stage5 | Change |
|---|---:|---:|---:|
| Machine instructions, `.text` | 18,592,421 | 18,086,341 | −506,080 |
| Unwind frames, index and exception tables | 5,822,232 | 5,622,932 | −199,300 |
| Read-only data, `.rodata` | 1,722,967 | 1,705,240 | −17,727 |
| Relocated read-only data, `.data.rel.ro` | 1,711,288 | 1,752,520 | +41,232 |
| Packed relative relocations, `.relr.dyn` | 27,240 | 27,896 | +656 |
| Other sections, headers and file alignment | 211,076 | 211,487 | +411 |
| **Complete file** | **28,087,224** | **27,406,416** | **−680,808** |

All seven compared instruction, unwind and allocated-data sections of the symbol-bearing attribution image match the exact stage5 shipping image. The `.cfi_reserve` allocation is 209,684 bytes with zero file bytes, so it is excluded from the file-backed totals. The following census includes every nonzero disjoint code category and its associated unwind metadata; shared instruction ranges count once even when they have multiple Rust names.

| Linked code and associated unwind category | Stage5 bytes |
|---|---:|
| Property registration, stores and observers | 6,036,568 |
| Other framework and application code | 4,402,036 |
| Callable entries and trait shims | 3,108,554 |
| Generated native XAML functions | 2,303,458 |
| Other XAML runtime, theme and codecs | 1,773,613 |
| Bindings, reactive operators and events | 1,268,165 |
| Compiler-generated destruction | 1,023,099 |
| Other Rust and native code | 778,753 |
| Reflection and value conversion | 768,246 |
| Other closure bodies | 682,550 |
| Class projection, initialization and virtual dispatch | 449,046 |
| Reference counting and weak handles | 443,242 |
| Collection primitives | 385,503 |
| Thread-local and once initialization | 194,702 |
| Aliases from multiple categories | 67,344 |
| Other explicit Drop implementations | 14,209 |
| Other async state machines | 3,948 |
| No sized symbol | 3,825 |
| Shared exception table | 2,340 |
| Common unwind header and CIEs | 72 |
| **Disjoint code and unwind total** | **23,709,273** |

This total is exactly `.text` plus the three unwind/exception sections; the remaining data and file overhead stay in the preceding table. The source of these values is `stage5.sections`, `stage5.attribution` and `stage5.generated`. The census describes all linked categories of this executable and does not assign all changes to the two weak-root callbacks or to one profiling difference.

### Exact source, fresh training and publication mapping

The measured source is published at [b2812cbad58bdb8c8c681e21fc60ea6765f44ac8](https://github.com/wieslawsoltes/FerroUI/commit/b2812cbad58bdb8c8c681e21fc60ea6765f44ac8), tree `e7067b0d7d1745392ac73fa558739f1a68a1febd`. Its original capture records `98e060758e528234bd653af3ab4c5a506e44953d` plus the dirty weak-root change. The separate publication bridge verifies all 13 captured source/test hashes against b281 without rewriting that observed capture. The separate scanner test-only repair is published at [cc094747800432df5fe2b4922bb31599762794f1](https://github.com/wieslawsoltes/FerroUI/commit/cc094747800432df5fe2b4922bb31599762794f1), tree `9b39cdda022345f4f51cb62e68ea3f590ce2446f`. Its bridge verifies exactly the two test-file changes and the successful local replay, without replacing the measured b281 mapping.

| Stage5 artifact | SHA-256 |
|---|---|
| Captured source manifest | `7516224993f8f8bd313ce6ccd8b2f9cf7db7713fb1961b9ee94a04a19da30678` |
| Changed `value_type.rs` | `d6550b20e448ac9b46b40b8f5fb0b7654e872446fe5adbffca476d2b51ee6f39` |
| Fresh merged profile | `25fc53e08b44991a7a8dbbca5b08049b712ad0b3f4760021dae7b3a0e518fda3` |
| Optimized release Base library | `d07a0b4178a8d661db6f70428adab941368f4e0b22ddfe1850e049ef9061388b` |

The instrumented build took 1,166.013 seconds. The original 13 training launches produced 13 raw profiles totaling 424,307,376 bytes, followed by a 717.692-second optimized whole-graph build. These are individual build durations, not a paired compile-time comparison. The shipping link uses `opt-level=z`, FatLTO, safe identical-code folding, packed relative relocations and guarded unwind compaction. Complete commands, controlled environment, raw-profile identities and the frozen dependency graph are retained by the producer.

The optimized-Base audit has passed 20 targeted checks covering current profile identities, selected root definitions and notification targets, folded hashing, conversion-clone handling and the weak-root leaves. This is a bounded audit of specified functions, not a claim that every profile record matched. Its hash-entry count is 65,774, compared with 115,042 in stage4; fresh profiles are not interchangeable weighted inputs. The final captured registration object also passes 19 identity checks and eight callback/descriptor checks. It has the inspected folded hash lookups and no outlined outer or nested conversion clone. The 80-byte weak-reference descriptor has ten relocations and points at two direct 169-byte root conversion leaves with verified root TypeId bytes. The audit ties these observations to the exact source, optimized Base, profile, object and fixture binary. [Stage5 profile and native-code audit](benchmarks/compiled-theme-stage5-codegen-linux-x86_64.json)

### The focused weak-root source change

The source delta from the published stage4 evidence checkpoint changes only `src/FerroUI.Base/data/core/value_type.rs`. Two built-in routes, from `Ref<FerroObject>` to `ElementRef<T>` and `Option<ElementRef<T>>`, now use `BorrowedConversion::Static` callbacks. Each concrete callback accepts `&dyn Any`, checks the root-handle type, performs the class cast, and boxes the correctly typed weak result. The shared installer still performs registry insertion. It keeps the existing `set_conversion` calls, the independent assignability and borrowed views when keys overlap for `T = FerroObject`, and the separate `from_object` metadata operation.

This restores the borrowed callback boundary for those routes while retaining static callbacks and shared registration. The retained reference implemented the original routes as `BorrowedConversion::Shared`; it did not use the private `ConvertFn::Boxed` variant. Stage4 routed those two operations through `FromRoot`. Other conversion routes and the public APIs remain unchanged. Converter ownership is still cloned out of the registry before invoking callbacks, and element references remain weak. Re-registering class built-ins restores the declared routes; replacing a conversion preserves the independently registered views.

The preceding Base-only weak-root experiment is a separate diagnostic. Its short boxed-rejection interval included parity, and the longer boxed-rejection intervals also included parity on both CPU pins. Its short borrowed-miss lane still flagged a slowdown: 1.0819 [1.0440, 1.1002]. The experiment reused the stage4 profile and moved the root code from compiler unit `.14` to `.04`, losing all 81 prior root-profile matches and the two observed notification-target promotions. Those results justified testing this source with fresh training; they did not establish whole-application acceptance. Stage5's final short cohorts and separate longer controls do not retain either rejected-boxed or borrowed-miss slowdown in their reported intervals. Those results belong to the freshly trained stage5 source and binaries. The original diagnostic flag and profile drift remain in the [weak-root trial audit](benchmarks/compiled-theme-weak-root-trial-codegen-linux-x86_64.json).

### Completed local checks and measurement boundaries

| Gate or companion | Stage5 result and boundary |
|---|---|
| Release public property/binding suite | **338 passed**, zero failed; one benchmark-only test ignored in correctness mode |
| Class, dispatch, nullable and weak-reference registry suite | **47 passed**, zero failed |
| Compact-unwind runtime probe | Passed against the stage5 optimized Base library with compact unwind applied |
| Full editable archives | **9 cases passed**, including embedded/installed, scalar edits and structural edits across supported archive versions |
| Development hot-reload configuration | **339 passed**, zero failed, one ignored in a separate Base/harness build with both debug assertions and `hot-reload` enabled |
| Required rollback/subscription/context test | Explicitly listed and passed: `typed_binding_reload_preserves_subscription_and_tracks_context_during_rollback` |
| Template-activation companion | Embedded, scalar and structural cases all passed; separate ThinLTO correctness scope |
| Registration/interface fixture assertions | All four production-policy reference/candidate links passed their assertions with compact unwind applied |
| Profile and final native proof | All 20 targeted optimized-Base checks, 19 final identity checks and eight weak-root callback/descriptor checks pass |
| Short registration/interface comparisons | All ten primary metrics on both CPU pins have intervals including parity or improvement |
| Longer conversion controls | Separate boxed-rejection and borrowed-miss cohorts on both pins show parity or improvement; same-image controls include parity |
| Eleven-lane property/dispatch companions | Both production-policy links and all smoke/checksum checks pass; one original short CPU7 mixed-property flag is retained with the separate follow-up below |
| Seven binding lanes and full-application startup | All primary readiness and seven binding intervals include parity; counts and protocols remain separate below |
| Local repository source guards | View-builder and designer-schema checks passed; the original idioms failure and subsequent successful repair check are both retained |
| Test-only scanner repair | **48 passed**, zero failed/ignored; idioms `--check` exits 0 after the two-file mock-trait rename |

The release and development counts belong to different configurations and must not be added into one release-test total. The development Base and harness use separate output paths and no PGO; the manifest verifies that the measured release graph and Base remain unchanged.

The final template companion activates Button, ScrollViewer, NavigationView and Slider against the preserved stage5 libraries. Each of the embedded/scalar/structural cases has 13 activations per control, one root per activation, and 2/8/26/2 nodes including the control. Scalar font sizes 14/17, structural font sizes 19/29 and the added structural resource value 31 all pass. The companion uses a separate ThinLTO correctness policy; its 41,783,536-byte executable is not the shipping executable or a timing result. Its SHA-256 is `f8952f31dc7377adcd9099290b8d55dbaeac2fec3f5098387a71ee4d9e60197a`, and its link took 464.275 seconds. It adds no training and leaves the release graph unchanged.

The new property comparison uses the exact 5,651-byte source retained at `compiled-theme-binding-dispatch-linux-x86_64.json#/property/property_workload_source`, SHA-256 `2758bd8c66978d414e74ff0a2dad52442ecf0d5d03a22d77df6d4381acf11b70`. It covers metadata/default lookup, inherited and mixed-host reads, effective local values, mixed reads/writes, style installation/disposal, style binding emission/readback and coercion. This workload appeared in the instrumented training main. The optimized shipping main matches `winui.rs` and does not execute it. The two new standalone fixtures use the matched-reference/stage5 libraries and profiles with production `opt-level=z`, FatLTO and compact unwind. Their completed 31-pair CPU6/CPU7 results remain distinct from full-executable startup and the held-out binding/registration timing workloads. Inclusion in training alone does not establish profile-key coverage for every standalone fixture function.

The idioms failure concerns the mock trait declaration in the scanner-only nullable-registration fixture. The published test-only repair renames it to `TestProvider` and updates the canonical-path assertion, leaving the real public `IServiceProvider` API and idioms baseline unchanged. Both fixture readers are gated under `cfg(test)`. The separate local replay passes all 48 scanner tests, and the idioms check reports no new .NET-shaped declarations against its unchanged 219-entry baseline. The original failure, repaired file hashes, commands and successful output remain separate from the b281 runtime snapshot. These are local checks, not a GitHub Actions green result.

### Final registration and conversion results

The short fixtures each retain 31 alternating pairs after two warmups on their own CPU pin. The following cells are median paired candidate/reference time ratios with bootstrap 95% intervals. A ratio below one means less measured time. Element/property/weak-conversion rows use thread CPU; interface rows use wall time. No row pools the two pins.

| Workload | CPU6 ratio and 95% interval | CPU7 ratio and 95% interval |
|---|---:|---:|
| Repeated element registration | 0.43218 [0.41219, 0.46435] | 0.44080 [0.43477, 0.46787] |
| Repeated property registration | 1.00050 [0.93645, 1.05451] | 0.96404 [0.92049, 1.03776] |
| Successful boxed weak conversion | 0.94629 [0.90443, 0.98424] | 0.97372 [0.94007, 0.99039] |
| Successful borrowed weak conversion | 0.95478 [0.92766, 1.00052] | 0.94594 [0.92319, 0.96159] |
| Rejected boxed conversion | 0.98695 [0.94382, 1.02288] | 0.96926 [0.91468, 1.01864] |
| Borrowed missing-key lookup | 0.99328 [0.96373, 1.02682] | 0.95685 [0.93628, 0.98941] |
| Expired boxed conversion | 0.96963 [0.91507, 0.99178] | 0.94699 [0.91753, 0.98593] |
| Expired borrowed conversion | 0.92460 [0.91240, 0.96532] | 0.95603 [0.93899, 0.97260] |
| Default interface registration | 0.69967 [0.67038, 0.73068] | 0.74341 [0.70705, 0.75980] |
| Successful interface conversion | 0.97173 [0.95136, 0.99394] | 0.97756 [0.95424, 0.99965] |

One warmed replacement pass over 32 default interface declarations uses zero adapter allocations/bytes in the candidate, compared with 32 allocations and 768 bytes in the reference. This measures replacement cost after registry capacity is warm; it does not describe first-time capacity allocation.

The longer conversion controls are distinct 31-block cohorts after three warmups, with five million operations per process, all six candidate/reference/same-image-control orders, and unchanged fixture binaries. The boxed run also retains its successful-conversion control lane; the borrowed run explicitly selects only the previously flagged missing-key lane. The key rejection results are:

| Lane and CPU | Candidate/reference | Same-image control/reference |
|---|---:|---:|
| Boxed rejection, CPU6 | 0.97223 [0.93444, 0.99470] | 1.00684 [0.96005, 1.01833] |
| Boxed rejection, CPU7 | 0.98180 [0.96221, 1.00027] | 0.98720 [0.97160, 1.03073] |
| Borrowed miss, CPU6 | 0.97034 [0.95061, 0.99633] | 1.00283 [0.97671, 1.02046] |
| Borrowed miss, CPU7 | 0.97587 [0.95138, 0.98480] | 0.98749 [0.96103, 1.00932] |

The earlier rejected-boxed and borrowed-miss regressions are not reproduced in these final comparisons. Native-code correctness and these throughput results establish their respective scoped findings; the individual source and profile contributions to the whole-program result remain unisolated.

### Property workloads and the original mixed-lane flag

The eleven-lane companion uses the trained workload described above, with 31 alternating process pairs and two warmups on each pin. All checksum/operation tuples agree. These are thread-CPU ratios from the original short cohorts:

| Property workload | CPU6 ratio and 95% interval | CPU7 ratio and 95% interval |
|---|---:|---:|
| Default metadata | 1.02413 [0.96474, 1.14342] | 1.03510 [0.98312, 1.09354] |
| Scoped metadata | 1.01640 [0.90814, 1.06911] | 0.97551 [0.90389, 1.07176] |
| Cached inherited metadata override | 1.01529 [0.91760, 1.12709] | 0.92552 [0.82265, 0.95986] |
| Plain typed read | 0.95772 [0.90406, 1.04104] | 1.03991 [0.93076, 1.07674] |
| Derived default read | 1.00408 [0.93547, 1.06724] | 1.02087 [0.94373, 1.04386] |
| Mixed-host read | 1.04726 [0.88139, 1.08528] | 1.00584 [0.95876, 1.04167] |
| Effective local read | 0.95262 [0.88326, 1.02172] | 1.05175 [0.96148, 1.07737] |
| Mixed 75% read / 25% write | 0.99819 [0.91002, 1.07723] | 1.06291 [1.02256, 1.08351] |
| Style set/read/dispose | 0.97757 [0.92766, 1.06144] | 1.07611 [0.98642, 1.12873] |
| Style binding emit/read | 0.99871 [0.91198, 1.07254] | 0.95072 [0.91808, 1.03791] |
| Scoped local coercion | 0.99079 [0.87842, 1.02884] | 1.00527 [0.98278, 1.05162] |

The original CPU7 mixed-workload interval is above one and remains a recorded slowdown flag. The other initial property intervals include parity or improvement. A bounded follow-up used the same binaries and source, three arms, all six rotated arm orders, three warmup blocks and 31 measured blocks on each pin. Each arm/block invokes the existing fixture at all 11 original lane rotations and aggregates only the mixed lane: **2.2 million operations per arm/block**, compared with 200,000 operations in the original single-process sample. This is a longer aggregate experiment, not an unchanged-duration repeat. The other ten output lanes are correctness checks only in this follow-up.

| Longer mixed-workload comparison | CPU6 ratio and 95% interval | CPU7 ratio and 95% interval |
|---|---:|---:|
| Candidate/reference | 1.01183 [0.98787, 1.02875] | 0.99070 [0.95661, 1.02784] |
| Candidate/same-image reference control | 1.00063 [0.95956, 1.01942] | 0.99479 [0.93059, 1.01697] |
| Same-image reference control/reference | 1.00558 [0.97130, 1.05411] | 1.02038 [0.96347, 1.04169] |

All three thread-CPU and wall-time intervals include parity on both pins. Each pin retains all 1,122 invocations and 12,342 checked output rows, including warmups, with matching per-rotation correctness signatures. The longer controlled experiment did not reproduce the original CPU7 flag. The original result is neither discarded nor pooled with the follow-up, and these observations do not establish a causal explanation for it. The recorded cohorts support proceeding; they do not prove universal zero regression or formal equivalence.

### Final binding and full-application startup comparisons

All seven held-out binding lanes have intervals including parity in their 25-pair thread-CPU cohort on CPU6:

| Binding workload | Paired ratio and 95% interval |
|---|---:|
| Local installation | 0.98238 [0.90937, 1.02335] |
| Style installation | 0.97547 [0.92684, 1.02914] |
| Local notification | 1.04359 [0.90609, 1.21394] |
| Style notification | 1.03565 [0.89582, 1.10000] |
| Typed notification | 0.97465 [0.88132, 1.06739] |
| Typed write-back | 0.99328 [0.96457, 1.02365] |
| Typed data-context change | 0.99260 [0.90398, 1.08898] |

The shipping executable uses the clean startup protocol: both expected-font variables are unset equally, while separate compatibility launches check edited values. CPU6 process readiness retains four independent workloads:

| Startup workload | Pairs | Paired ratio and 95% interval |
|---|---:|---:|
| Embedded archive | 101 | 0.98735 [0.96761, 1.02213] |
| Scalar-edited v3 archive | 41 | 1.02074 [0.97241, 1.06820] |
| Structurally edited v3 archive | 41 | 0.98556 [0.95927, 1.00972] |
| Legacy v1 fallback | 21 | 0.98819 [0.95394, 1.00564] |

The separate 201-pair embedded repeat on CPU7 reports process readiness at **1.00716 [0.98530, 1.04056]** and internal theme creation plus typed Button lookup at **1.01060 [0.98632, 1.04571]**. Both metrics describe the same 201 process pairs. These intervals include parity; inclusion of parity is not proof of identical throughput. No painted-frame or GPU-performance claim follows from these headless workloads.

### Evidence, publication and integration scope

The [complete continuation evidence](benchmarks/compiled-theme-rust-idioms-linux-x86_64.json) retains the original samples, commands, per-build source/profile/graph identities, native-capture records and a source-text appendix deduplicated by SHA-256. It extends the immutable [stage4 checkpoint at 98e06075](https://github.com/wieslawsoltes/FerroUI/blob/98e060758e528234bd653af3ab4c5a506e44953d/docs/benchmarks/compiled-theme-rust-idioms-linux-x86_64.json), preserving every earlier failure and diagnostic rather than rewriting them as stage5 results. The final completeness flag means all required evidence is present; the scoped runtime interpretation is recorded separately.

The measured runtime source is b281; cc094 contains only the separately verified two-file scanner repair. Local tests and source guards do not establish GitHub Actions status or compatibility with a later main-branch revision. The dated repository observation is retained independently and records no merge into advancing main. Both numerical size targets remain unmet despite the completed runtime validation.

## Historical stage4 checkpoint

The following stage4 tables, source/profile identities and failed performance gate are preserved from the published 98e checkpoint. They remain historical results for source 415c and must not be relabeled as stage5 measurements.

The stage4 executable, release correctness checks, development hot-reload checks and separate template-activation companion are complete. It saves 683,608 bytes against the retained verified release. Its startup and seven binding comparisons detect no new regression in their recorded cohorts, but two initial registration cohorts flag slower rejected boxed conversions and a longer follow-up on CPU 6 confirms an unresolved slowdown. Stage4 is not accepted as the performance replacement; the retained release remains the reference. Earlier candidates exposed notification, lookup and boxed-conversion regressions; their source, profile and measurement cohorts remain separate below.

The [complete stage4 evidence checkpoint at 98e06075](https://github.com/wieslawsoltes/FerroUI/blob/98e060758e528234bd653af3ab4c5a506e44953d/docs/benchmarks/compiled-theme-rust-idioms-linux-x86_64.json) retains the original samples, artifact identities, separate audits and exact reproduction sources. Its completeness flag does not declare performance acceptance.

### Complete executable and resource inventory

The stage4 candidate builds to **27,403,616 bytes** on Linux x86-64 with whole-graph PGO, `opt-level=z`, FatLTO, safe identical-code folding, packed relative relocations and guarded unwind compaction. Its SHA-256 is `55379554fe20c4f5b1b635e44c32f60afef8d4517996a65969117300c339f159`. This source includes the shared registration changes, the inline TypeId hashing operation and the manual always-inline conversion-descriptor clone. Stage4 uses a newly trained profile for that exact source.

| Executable or target | Bytes | Relationship to stage4 |
|---|---:|---:|
| Original ordinary release | 54,577,848 | Stage4 is 27,174,232 B smaller (**49.789856%**) |
| Retained verified compact-unwind release | 28,087,224 | Stage4 is **683,608 B smaller** (2.43%) |
| Matched rebuild of reference source | 28,089,456 | Stage4 is 685,840 B smaller (2.44%) |
| Stage4, freshly profiled candidate | **27,403,616** | Complete editable executable; rejected-conversion gate unresolved |
| Half of the original ordinary release | 27,288,924 | Stage4 is **114,692 B above** this threshold |
| Historical further-30% target | 19,827,298 | Stage4 is **7,576,318 B above** this target |

All 894 FXB blocks remain byte-identical, totaling 180,091 bytes. The complete 296,347-byte FXP archive also remains byte-identical and occurs once in read-only data. Generated Rust falls from 4,961,664 to 4,879,766 bytes, a reduction of 81,898 bytes. There are no missing or added archive blocks. All nine embedded, installed, scalar-edited and structurally edited archive cases pass, including FXP versions 1, 2 and 3.

The size reduction comes mainly from instructions and their unwind metadata. Some descriptor and virtual-table data grows as the source changes. File-backed section counts make both effects visible:

| File-backed section group | Retained release | Stage4 | Change |
|---|---:|---:|---:|
| Machine instructions, `.text` | 18,592,421 | 18,084,885 | −507,536 |
| Unwind frames, index and exception tables | 5,822,232 | 5,621,860 | −200,372 |
| Read-only data, `.rodata` | 1,722,967 | 1,705,208 | −17,759 |
| Relocated read-only data, `.data.rel.ro` | 1,711,288 | 1,752,328 | +41,040 |
| Packed relative relocations, `.relr.dyn` | 27,240 | 27,888 | +648 |
| Other sections, headers and file alignment | 211,076 | 211,447 | +371 |
| **Complete file** | **28,087,224** | **27,403,616** | **−683,608** |

The symbol-bearing attribution image matches all seven compared executable, unwind and allocated-data sections of the stage4 shipping image. The `.cfi_reserve` range occupies 209,636 allocated bytes and zero file bytes, so it is not added to the file-backed totals. The census below lists every nonzero category of disjoint linked instruction ranges and their attributable unwind metadata. Its 23,706,745-byte total is exactly `.text` plus `.eh_frame`, `.eh_frame_hdr` and `.gcc_except_table`; other application data remains in the section table above.

| Remaining code and associated unwind category | Bytes |
|---|---:|
| Property registration, stores and observers | 6,037,161 |
| Other framework and application code | 4,403,055 |
| Callable entries and trait shims | 3,107,916 |
| Generated native XAML functions | 2,303,427 |
| Other XAML runtime, theme and codecs | 1,773,188 |
| Bindings, reactive operators and events | 1,267,282 |
| Compiler-generated destruction | 1,022,527 |
| Other Rust and native code | 778,457 |
| Reflection and value conversion | 768,152 |
| Other closure bodies | 682,403 |
| Class projection, initialization and virtual dispatch | 447,752 |
| Reference counting and weak handles | 443,317 |
| Collection primitives | 385,392 |
| Thread-local and once initialization | 195,075 |
| Aliases from multiple categories | 67,242 |
| Other explicit Drop implementations | 14,209 |
| Other async state machines | 3,948 |
| No sized symbol | 3,862 |
| Shared exception table | 2,308 |
| Common unwind header and CIEs | 72 |
| **Disjoint code and unwind total** | **23,706,745** |

The embedded archive is about 1.08% of the stage4 file. Its size cannot explain the remaining multi-megabyte gap. Further reductions must address duplicated runtime implementations and cleanup paths while preserving their measured typed operations. The sizes, section identities, category census and generated inventories above come from the separate `stage4.build_provenance`, `stage4.sections`, `stage4.attribution` and `stage4.generated` evidence records.

#### Source and profile provenance

The stage4 capture records source HEAD `a1e622c5a86433dcfe0a2b2d7b1f76a57dc06122` plus hashes of 13 explicitly captured files. The HEAD alone is not the measured source state. The only production change from the stage3 capture is the manual `Clone` implementation in `value_type.rs`, whose stage4 SHA-256 is `9d581166d51d0bf7a9a84d9b1a68fb558c00459d5546ae88ae3df89e9ecfb427`. All 13 captured file hashes match the published source checkpoint [415ccebe8f8fe9988e05716e84019dd92eb2c523](https://github.com/wieslawsoltes/FerroUI/commit/415ccebe8f8fe9988e05716e84019dd92eb2c523). The separate publication bridge preserves this mapping without rewriting the original HEAD/dirty-tree snapshot or declaring performance acceptance.

| Stage4 artifact | SHA-256 |
|---|---|
| Source snapshot JSON | `4c8a114775ee942f84c6bfb83b992c0367dc1162bd99ecabaafbaeaa0fc0f9bf` |
| Fresh merged profile | `ec080cdfdf0077d7def03773baa4643b06a1b85d453f18cb35c00dc967d5e329` |
| Optimized Base rlib | `2ba02254bc929307b70f06ee5bb37c41b372c77c831bcb51737358e8d10f772c` |

The instrumented build completed in 979.83 seconds, followed by the original 13 training launches and a 697.99-second optimized whole-graph build. The merged profile comes from 13 raw files totaling 424,292,632 bytes. The build records retain their complete commands and the resulting dependency graph; these elapsed times describe those captured builds, not a paired compile-time benchmark.

#### Earlier executable cohorts remain separate

| Cohort | Complete file bytes | Profile and measured limitation |
|---|---:|---|
| Combined source, fixed original profile | 26,941,144 | Rejected after notification regressions; moved internal profile keys lost accepted metadata and common-target promotion |
| Stage2, fresh profile before hashing annotation | 27,404,624 | Notification code generation recovered; weak-reference conversion and startup concerns remained |
| Stage3, fresh profile with hashing annotation | 27,403,320 | Hash lookup recovered; boxed conversion regressed on both measured CPU pins, and clean startup measurements showed a smaller concern |
| Stage4, fresh profile with manual conversion clone | 27,403,616 | Current release and development correctness checks pass; original and longer follow-up results retain an unresolved rejected-boxed slowdown |

The Base-only hashing and cloning experiments have separate fixture results. Their inherited profiles stopped matching other changed internal keys, so those diagnostic measurements do not replace the complete-application cohorts. The startup protocol also changed between stage2 and the later clean cohorts, as described below.

### Correctness checks

The stage4 complete consumer, release property/binding suite, registry/dispatch suite and compact-unwind probe pass against its recorded source and optimized Base library. The earlier scanner run covers scanner files whose captured hashes are unchanged. The development hot-reload suite and template companion have also completed in separate configurations. Completed performance measurements retain the unresolved rejected-conversion gate:

| Check | Result and scope |
|---|---|
| Public property and binding suite | 338 passed; one benchmark-only test is ignored in correctness mode and supplies the separate seven-lane timing entry |
| Class, virtual dispatch, nullable and weak-reference registration | 47 passed, including default versus explicit interface projections, most-derived selection, replacement, one-time expression evaluation and inherited slot behavior |
| Source scanner | 48 passed, including descriptor batches, aliases, macro expansion, unread-registration accounting and metadata serialization |
| Compact-unwind runtime probe | Passed with the typed panic payload, nested cleanup order, property-store reuse and binding reuse preserved; the linked image contains the compact-unwind reserve |
| Full editable archive compatibility | 9 passed, including embedded and installed defaults, scalar edits in v1/v2/v3, combined edits and structural edits in v1/v2/v3 |
| Compiled resource inventory | All 894 FXB blocks and the complete FXP archive have the reference hashes |
| Production-policy registration and interface fixtures | All four reference/candidate links applied guarded unwind compaction and passed their assertions |
| Final conversion machine code | The captured stage4 registration object has folded hashing in both lookups and no outlined outer or nested conversion clone; the audit binds the native object and link command to the measured fixture manifest |
| Development hot-reload rollback and context tracking | 339 passed, zero failed, one ignored in a separate Base/harness build with both `debug_assertions` and `feature="hot-reload"`; the gated rollback/subscription/context test was explicitly listed and executed |
| Final template-activation companion | All three embedded/scalar/structural archive cases pass: Button, ScrollViewer, NavigationView and Slider have 2/8/26/2 nodes including the control, one root, and 13 activations each per case |
| Startup, binding and registration/conversion timing | Complete for stage4: startup and all seven binding intervals include parity or improve; initial registration cohorts and the longer CPU 6 follow-up retain a rejected boxed-conversion slowdown |

The test and fixture manifests record the Base archive hash, source hashes, compiler command and binary identity. The release checks above use the stage4 Base identity listed in the provenance table. The feature-specific verification has separate metadata and output paths, uses no PGO, and does not modify the measured release graph. Its Base SHA-256 is `53f71a0ce87b151f62e244abc16f3f8da702e36ab05b10faab147c15a67b8e50`, and its QA executable SHA-256 is `ffeab1a00e32b462d5def0afdab6ef0a58e07ff39bb13fe3691e7214753718d2`. The release count of 338 and development count of 339 are distinct runs. The additional development test is `typed_binding_reload_preserves_subscription_and_tracks_context_during_rollback`; its list and execution logs are retained.

The archive checks resolve the shipped themes and verify edited values. Instrumented training activates three template types; it does not establish final optimized execution of all four. The separate template companion exercises the four controls against the preserved stage4 libraries, including held-out Slider activation. It also asserts normal/compact font values 14/17 for the scalar archive and 19/29 for the structural archive, plus the added structural resource value 31. Its ThinLTO link with ordinary unwind has a separate correctness scope and policy from the shipping FatLTO executable. It adds no training or timing result, and the release graph and Base hashes remain unchanged. Panic unwinding and the external interpreter remain enabled throughout the release checks.

#### Registration results and the unresolved rejection path

The original stage4 registration runs each retain 31 alternating pairs. The following results use median paired candidate/reference time ratios and bootstrap 95% intervals; a ratio below one means less measured time. Repeated registration uses warmed registry capacity.

| Workload | Timing measure | CPU 6 ratio and 95% interval | CPU 7 ratio and 95% interval |
|---|---|---:|---:|
| Repeated element registration | Thread CPU | 0.4660 [0.4363, 0.4855] | 0.4341 [0.4266, 0.4396] |
| Repeated property registration | Thread CPU | 1.0266 [0.9668, 1.0744] | 1.0296 [0.9942, 1.0731] |
| Default interface registration | Wall time | 0.7087 [0.6779, 0.7391] | 0.6991 [0.6700, 0.7093] |
| Successful interface conversion | Wall time | 0.9951 [0.9599, 1.0349] | 0.9560 [0.8825, 0.9917] |
| Successful boxed weak conversion | Thread CPU | 0.9517 [0.9323, 0.9787] | 0.9749 [0.9359, 0.9972] |

One warmed replacement pass over 32 default interface declarations drops from 32 allocations / 768 allocated bytes to zero allocations / zero allocated bytes. This isolates replacement-adapter allocation from first-time registry capacity growth. The rejected boxed-conversion lane reports a different outcome:

| CPU pin | Paired median ratio | Bootstrap 95% interval |
|---|---:|---:|
| 6 | 1.1532 | 1.0108–1.3596 |
| 7 | 1.1979 | 1.0066–1.2694 |

These are the original cohort results, even though successful boxed conversion improves and the native lookup/clone checks pass. A process-order association motivated a separate longer-run diagnostic using the same unmodified binaries: five million operations per process, a second invocation of the exact reference executable as a control, and all six process orders. Its 31 measured blocks on each CPU retain these rejected-conversion results:

| CPU pin | Candidate/reference ratio and 95% interval | Same-image reference control/reference ratio and 95% interval |
|---|---:|---:|
| 6 | 1.3928 [1.3292, 1.4333] | 1.0153 [0.9854, 1.0373] |
| 7 | 1.0465 [0.9812, 1.1112] | 1.0022 [0.9782, 1.0132] |

The longer CPU 6 run retains a slowdown while its same-image control includes parity. CPU 7's longer-run interval includes parity; it does not erase either original cohort's flag. The process-order observation therefore does not explain away the failure, and the cohorts are not pooled. A bounded native inspection finds no additional call or reference-count operation in the compared rejection path, but branch and layout differences remain; it establishes no causal percentage. [Residual route audit](benchmarks/compiled-theme-stage4-rejected-boxed-linux-x86_64.json) The completed startup/binding comparisons and template companion do not resolve this conversion result.

#### Completed startup and binding comparisons

All seven binding lanes include parity in their 25-pair thread-CPU intervals on CPU 6. Inclusion of parity does not prove identical throughput for every workload.

| Binding workload | Paired ratio | Bootstrap 95% interval |
|---|---:|---:|
| Local installation | 0.9722 | 0.9336–1.0235 |
| Style installation | 1.0092 | 0.8949–1.0410 |
| Local notification | 0.9764 | 0.9144–1.0769 |
| Style notification | 0.9908 | 0.9276–1.0315 |
| Typed notification | 1.0328 | 0.9861–1.1812 |
| Typed write-back | 1.0522 | 0.9540–1.1482 |
| Typed data-context change | 0.9920 | 0.8981–1.0831 |

The complete application uses the clean startup protocol described below. Its CPU 6 process-readiness measurements retain each workload separately:

| Startup workload | Pairs | Paired ratio | Bootstrap 95% interval |
|---|---:|---:|---:|
| Embedded archive | 101 | 1.0063 | 0.9753–1.0377 |
| Scalar-edited v3 archive | 41 | 0.9929 | 0.9811–1.0099 |
| Structurally edited v3 archive | 41 | 0.9986 | 0.9568–1.0159 |
| Legacy v1 fallback | 21 | 0.9699 | 0.9421–0.9925 |

The separate 201-pair embedded repeat on CPU 7 reports process readiness at 0.9943 [0.9795, 1.0096] and internal theme creation plus typed Button lookup at 1.0042 [0.9874, 1.0195]. Both metrics describe the same 201 process pairs. The timing-sequence record retains the commands and start/finish order. These broader comparisons leave the rejected-boxed failure open.

#### Public API review

A source review from the retained checkpoint `6d5225e8` found no removed public entry points or tightened caller bounds in this continuation. The internal decoder helpers change their result type; the public decoder still returns `XamlResult<XamlDocument>`, and all 29 original diagnostic detail strings match the compact error mapping.

| Public surface | Preserved contract |
|---|---|
| Styled property registration | `TOwner: ObjectType`, `TValue: PropertyValue`, and the original typed result |
| Attached property registration | `TOwner: StaticType`, `THost: ObjectType`, `TValue: PropertyValue`; a non-object owner remains valid |
| Markup handle registration | `H: 'static`, with the original first-registration rule |
| Reference-model registration | `PartialEq + 'static`; no additional `Clone` requirement on the model |
| Explicit interface registration | `fn(Ref<T>) -> I`, with the expression evaluated at registration and the selected callback retained |
| Class and override declarations | Existing matcher forms and hidden `@cast` arms remain; handwritten implementations without an override mask retain conservative typed thunks |

The nullable descriptor and batched registration APIs are additive. Constructing a descriptor records typed function pointers; it does not execute a user's clone or conversion implementation while assembling the batch. The reviewed contracts are source-level Rust APIs, supported by the scoped tests above.


## Changes to the Rust implementation

### Shared setup with typed operations

| Area | Representation and specialization | Source |
|---|---|---|
| Inherited virtual methods | Built-in declarations state their actual overrides. An unchanged slot can point at the inherited implementation while the class retains its own type identity and virtual table. Handwritten implementations without a declared override mask keep the conservative route. | [Class runtime and declaration macros](../src/FerroUI.Base/type_system.rs), [dispatch tests](../src/FerroUI.Base/tests/virtual_dispatch_tests.rs) |
| Native `StaticResource` assignments | A document shares a resolved operation only when its property and verified setter plan match. Each factory keeps its key and source position. Sharing runs after pruning and other lowering; a single surviving use stays inline. | [Resource assignment lowering](../src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/resource_assignments.rs) |
| XamlX transforms and portable decoding | Transforms borrow semantic-node data where ownership is unnecessary. Decoder internals propagate a compact error enum and construct the public `XamlError` at the API boundary. | [AST visitors](../external/XamlX/src/XamlX/ast/common.rs), [semantic transforms](../external/XamlX/src/XamlX/transform/transformers), [portable decoder](../src/Markup/FerroUI.Markup.Xaml.Loader/binary_document) |
| Required compiler types | An ordered lookup loop fills fixed stack storage. The wrapper specializes for the array length; the lookup loop does not. Lookup stops at the first error, and partially resolved handles drop in reverse lookup order. | [Required-type lookup](../external/XamlX/src/XamlX/type_system/type_system.rs), [compiler type table](../src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/transformers/ferro_xaml_il_well_known_types.rs) |
| Nullable and reference models | Typed descriptors provide null, wrap, unwrap and projection functions. Common installers perform the registry work. The existing individual registration APIs remain available. | [Value registry](../src/FerroUI.Base/data/core/value_type.rs) |
| Plain-model binding access | `owner_of<TSource>` depends only on the source type; `value_from_untyped<TValue>` depends only on the value type. Conversion diagnostics are shared. These operations no longer specialize unnecessarily for both type parameters together. | [Typed binding and property metadata](../src/FerroUI.Base/data/core/typed_binding_expression.rs) |

Native resource sharing preserves the verified branch and property-getter evaluation order. Its helpers remain local to the document and reference only their own operations; it does not introduce a global dispatch table that roots unrelated properties. The portable decoder retains its validation conditions and public diagnostic content. Borrowing an AST node does not authorize holding a mutable `RefCell` borrow across a visitor callback.

### Batch registration is also compiler metadata

The runtime's `NullableRegistration::of::<T>()` descriptor has to supply the same nullability fact to the source scanner as `ValueTypes::register_nullable::<T>()`. Without that connection the generated metadata omitted nullable handles such as `Rc<dyn IServiceProvider>`, and the complete WinUI build failed even though the runtime registration code compiled.

The scanner now normalizes both spellings to the existing `nullable` model entry. Its AST traversal, declaration-token reader, registration-macro expansion and unread-registration accounting use the same spelling map. Named constants, inline const arrays, fully qualified descriptor paths, local type aliases and repeated macro arguments are covered. Unsupported or inferred types remain visible as unread entries, and the result survives metadata serialization. The model format and the downstream interpretation of a nullable registration are unchanged.

[Scanner implementation](../src/FerroUI.Build.Scan/scanner/source.rs) · [Descriptor fixture](../src/FerroUI.Build.Scan/tests/fixtures/nullable_registration/lib.rs) · [Model tests](../src/FerroUI.Build.Scan/scanner/tests.rs)

### Weak element references use static operations

`register_element_ref<T>` now supplies an `ElementReferenceOperations` descriptor to one non-generic installer. The descriptor contains the operations that depend on `T`: checking the dynamic class, constructing an `ElementRef<T>`, upgrading its weak reference, and boxing the exact destination type. Registry insertion and the relationships among handle, nullable handle, element reference and nullable element reference are shared.

The built-in conversions use static function pointers instead of allocating a captured `Rc<dyn Fn>` adapter for each route. Stage5 keeps the root-to-element and root-to-nullable-element routes as concrete borrowed static leaves, with the root downcast and class cast inside each leaf; the shared installer and separate metadata projection remain intact. Public conversion APIs still accept their existing user callbacks. The registry keeps conversion, assignability and borrowed conversion views separate when a later registration overrides one view. This matters for `T = FerroObject`, where several keys overlap.

An element reference remains weak. Converting a live reference preserves object identity. Expiration causes a conversion requiring a strong handle to fail, while a nullable destination receives its correctly typed null. A root handle is checked against the requested runtime subclass before an element reference is created. Replacing a conversion preserves the previously registered assignability and borrowed conversion views. Re-registering an element class restores its declared built-in conversion routes.

[Implementation](../src/FerroUI.Base/data/core/value_type.rs) · [Weak-reference regression tests](../src/FerroUI.Base/tests/element_ref_registration_tests.rs) · [Registration and conversion probe](../scripts/themes/bench/registration_hot_paths.rs)

### Property owners are descriptors, while values remain typed

`FerroProperty::new<T>` supplies `TypeId::of::<T>()` and the Rust type name to one non-generic `new_core`. Name validation, identity allocation, flags, notification setup and route initialization are common work. The original property ID, type identity, callback ownership and initialization order are retained.

The public `register_with<TOwner, TValue>` and `register_attached_with<TOwner, THost, TValue>` functions remain typed facades. Their internal installers specialize only for `TValue`; owner and host are `&'static TypeInfo` operands. Typed metadata construction, validation, coercion and the resulting `StyledProperty<TValue>` or `AttachedProperty<TValue>` remain concrete. The assign-binding flag, attached host and registry insertions are initialized in their original order.

This change does not replace the typed effective-value cache or add a new lookup to the source implementation of an ordinary typed read or write. Whole-program optimization can still change the resulting machine code, so notification, metadata lookup and write-back throughput remain separate measurements.

[Property construction and registration](../src/FerroUI.Base/ferro_property.rs) · [Registration tests](../src/FerroUI.Base/tests/property_registration_tests.rs) · [Metadata behavior tests](../src/FerroUI.Base/tests/ferro_object_tests_metadata.rs)

### Default and explicit interface projections

Default `ferro_class_info!` interface declarations use a static projection with the existing `Ref<T>: Into<I>` contract. An explicit `I => expression` declaration continues to call the public `register_interface<T, I>(fn(Ref<T>) -> I)` API. Its expression is evaluated once at registration, and the selected function remains installed until that registration is replaced.

The private callback representation distinguishes a static function from the existing owned adapter. Both are selected according to the object's most-derived registered class and cloned out of the registry before invoking conversion code. Replacing a base entry cannot shadow a more-derived one. Nullable wrapping and exact boxed result types use the existing value registry. Static and explicit registrations can replace the same class entry in either order.

The supplied interface probe separates repeated registration allocations from conversion throughput. Removing an adapter allocation is a setup improvement; the extra callback-representation branch also executes during conversion and must be measured there.

[Interface registry](../src/FerroUI.Base/data/core/value_type.rs) · [Declaration routing](../src/FerroUI.Base/type_system.rs) · [Precedence, replacement and expression-order tests](../src/FerroUI.Base/tests/class_registration_tests.rs) · [Interface probe](../scripts/themes/bench/interface_registration.rs)

`MarkupType::register_handle<H>` follows the same boundary: its public generic wrapper computes the handle `TypeId`, then one non-generic function performs locking and insertion. Poisoned-lock recovery and the rule that an already registered handle keeps its original markup type are preserved. [Handle registration](../src/FerroUI.Base/metadata/markup_type.rs)

### Notification paths require their own evidence

The inherited typed-binding change groups target attachment, ownership, logging and completion state in `BindingTarget`. Its publication helper is also on the notification path. The current candidate explicitly inlines `owner` and `publish` so the concrete caller remains visible to optimization. The stage4 and stage5 binding measurements cover the resulting full paths in separate cohorts; the individual annotations’ contributions remain unisolated.

The write-back suppression flag still uses a drop guard so it is restored after panic unwinding. Completion invokes the sink before rereading the target property for frame disposal, preserving changes made by reentrant callbacks. Source and target ownership remain weak. The debug hot-reload pause/resume guards and pending publication behavior are covered by the separate development run with both the feature and debug assertions enabled.

The boxed local/style observer lanes do not directly use `TypedBindingExpression`. A regression in those lanes cannot be assigned to this helper solely from a combined result. Changes to inherited virtual callees, old profile coverage and downstream inlining must be distinguished with matched source trials. Reusing one profile and retraining each revision answer different questions and must be reported separately.

### Validation boundaries

The scanner regression suite establishes metadata recognition, coverage accounting and serialization. Weak-reference and class-registration tests establish the conversion and callback contracts. Property and binding tests cover typed values, precedence, reentrancy and ownership. Compiler and archive checks cover resource inventories, generated callers, external archives and structural edits. None of these replaces a throughput measurement.

A complete release comparison records stripped bytes and allocated ELF sections, then tests startup through typed resource resolution, real template activation, external edits, boxed notification followed by a typed read, two-way write-back, registration/conversion CPU time and allocations. Each measured pair records the same compiler and link policy together with its own source, profile and dependency graph identities. Separate correctness companions record any different link settings explicitly. Headless resource lookup and template construction do not measure painted frames, GPU work or platform-specific desktop behavior. The retained release remains a preserved comparison baseline. Runtime acceptance is evaluated separately from evidence completeness and the numerical size targets.

## Keep the small hashing operation visible across compiler units

Stage2, the first freshly profiled candidate, still regressed in weak-reference conversion. A failed borrowed lookup is particularly useful here: it does not find a converter, so its slowdown cannot come from invoking a static function instead of an owned callback.

The source of the public lookup methods, their registry representation and the tested TypeIds matched the reference. The final machine code did not. Reference lookup inlined the TypeId hash; the candidate called an additional `hash_one` function and saved/reloaded registers around it. The reference lookup occupied 247 bytes; the candidate used 262 bytes plus an 11-byte hash helper. Both boxed and borrowed conversion reached this lookup, including the missing-key case.

The difference starts before final LTO. `IdHasher::write_u64` moved to a different compiler unit from the generic `hash_one`. Without an inline annotation, the latter retained stack storage and two calls during the earlier optimization. Final LTO simplified the small helper but left the outer call in the lookup.

The correction is deliberately local:

```rust
#[inline(always)]
fn write_u64(&mut self, i: u64) {
    self.0 = self.0.rotate_left(32) ^ i;
}
```

The hasher type, accumulator algorithm, equality checks, table representation and conversion callbacks are unchanged. The annotation exposes the tiny operation before the earlier inlining decision. In the isolated native object, both conversion lookups contain rotate/XOR directly and have no call instructions; the standalone pair-key `hash_one` symbol disappears. All six weak-conversion lanes improved or included parity in that experiment, while property setup regressed under its inherited profile.

That experiment is a diagnostic rather than the release decision. Even this annotation changes the Base compiler-unit partition: the notification functions move from `.12` to `.10`, invalidating their inherited local profile names. Stage3 therefore trained the hash-annotated source on the original workloads and repeated the complete runtime comparison. [Source, native instructions, unchanged TypeIds and isolated measurements](benchmarks/compiled-theme-conversion-codegen-linux-x86_64.json)

## Keep conversion ownership snapshots in the caller

Stage3, the hash-annotated and freshly profiled candidate, fixes the extra hash call but exposes another optimization boundary. Boxed live conversions take 6.0% and 8.2% longer on two CPUs, and a rejected boxed conversion takes about 9% longer on both. Borrowed live and expired conversions improve, while borrowed missing-key lookup and boxed expiration include parity in their intervals.

Stage3's final native `try_convert_registered` calls an out-of-line, 86-byte `ConvertFn::clone`. Both the matched reference and the faster isolated hashing trial inline that operation. The outlined route adds a call and a 24-byte stack round trip. Selected static conversion leaves are instruction- and relocation-identical to those in the isolated trial, and the common `apply` implementation differs by only one byte. This identifies the snapshot boundary as the next local repair.

The derived clone already has an ordinary inline hint. Its fresh optimized definition carries a low accepted entry count of 130; the isolated trial lacks that entry metadata. The repair supplies a manual `Clone` implementation with `#[inline(always)]` for the private `ConvertFn` enum. Every variant retains the derived operation: static functions and the nullable flag are copied; owned callbacks receive the same `Rc` clone; the borrowed conversion retains its own clone implementation. There is no new callback invocation, registry lookup or change to the descriptor layout.

The public `try_convert_registered` body stays unchanged. It clones the descriptor while reading the registry, ends that borrow, then invokes the owned snapshot. This preserves callbacks that reenter registration and callbacks whose registration is replaced during execution. Inlining is also permitted at other internal clone sites, so both complete-file size and startup require remeasurement. [Linked calls, clone branches, static leaves and exact artifact identities](benchmarks/compiled-theme-stage3-codegen-linux-x86_64.json)

The isolated manual-clone diagnostic removes the native clone call and its outlined body while preserving the folded hash lookup. It also moves root notification code from compiler unit `.10` to `.14` and changes the private implementation indices in conversion symbols. All 81 prior root-profile matches are lost under the inherited stage3 profile. Stage4 consequently trains a new profile for the manual-clone source before building its complete executable. [Clone diagnostic and its inherited-profile limitation](benchmarks/compiled-theme-clone-trial-codegen-linux-x86_64.json) · [Diagnostic native proof](benchmarks/compiled-theme-clone-trial-native-linux-x86_64.json)

## Why the old PGO input stopped describing the changed source

The unchanged `FerroObject::notify_property_changed` and root virtual handlers moved from LLVM code-generation module `ferroui_base.aa5a9f3f002932e2-cgu.03` in the reference to `.02` at `484189ae`. LLVM's internal-function profile keys include the module name. All 105 internal root-handler definitions in the rebuilt Base archive had an old profile record when matched by Rust symbol alone; none had an exact match for its current module-prefixed key.

The corresponding optimized bitcode lost the function-entry metadata and indirect-call promotion on the notification path. The reference guarded and inlined the common root handler, skipped its empty change handler, and inlined handler-list emptiness checks. The fixed-profile rebuild instead made an indirect core-handler call, a further indirect jump through the change slot, and out-of-line emptiness checks. The linked QA executable confirmed those calls. This is observable extra work even though the source of these methods did not change.

Retraining uses exactly the original 13 launches: three rounds each of startup, real templates, the existing property workload, and a scalar edit, followed by one structural edit. It does not add the held-out boxed notification, typed-binding, registration or conversion probes. The first freshly trained profile, before the hashing annotation, is identified by SHA-256 `9f0d44c37aeaccbcb7dd4ee9b3c8cab328b40be32fe6df9ff2606a5b34cfef66`; the old input was `736840a1a89cda4724cbc66d7e6e8bf525be704f769fc12027b6a9ca4ddbe709`.

For 15 selected functions, the raw edge-counter vectors and control-flow hashes are identical between the two training runs. The first fresh candidate's internal keys name module `.12`, matching that source partition. Its optimized Base library consumes those keys: `notify_property_changed` regains its 14,164-entry metadata and both root-target promotions. Its handler-list operations are inlined again. These counts are pre-final-LTO metadata after optimization, not raw call totals.

Inherited-slot sharing also changes the indirect-target distribution legitimately. The notification core slot records the same 361,703 total targets in both training runs, but 15 distinct target functions become four. Calls formerly directed to inherited control wrappers now reach their parent implementations directly. The boxed `i32` observer entry points retain zero training counters in both profiles.

This evidence establishes the profile-key failure and the recovered code generation. It does not assign a percentage of the measured performance change to that mechanism. The first fixed-profile trial, the refreshed deployment build, and their paired measurements remain separate results. Training at `opt-level=s` and applying at `z` also retains the existing possibility of control-flow hash mismatches; LLVM discards incompatible counts. [Selected profile records, optimized IR and provenance](benchmarks/compiled-theme-pgo-key-drift-linux-x86_64.json)

Stage3 has a separately trained profile, SHA-256 `14ab6d82afc601fca80bf6785dc2c5d6dc613c545677945d3f213642f63fda80`. Its 13 training launches use the same original workloads. All 15 selected internal profile names match the new `.10` compiler unit, and all 81 optimized root-vtable definitions retain the preceding fresh build's accepted entry counts. `notify_property_changed` again has its 14,164-entry metadata and both root-target promotions. The pair-key hashing IR contains no stack allocation or calls. Those recovered properties coexist with the boxed-clone regression measured in that cohort.

Stage4 uses the fresh `ec080cdf…7d5e329` profile identified above. The optimized Base audit verifies all 15 explicit local root keys in `.14`, accepted profile counts for all 81 root-vtable definitions, both notification target promotions, folded pair-key hashing and inlined outer and nested conversion clones. All 13 captured changed-source hashes match the source used for measurement. The final registration fixture's captured native object confirms both hash lookups and both clone levels: no pair-key hash helper or outlined conversion clone remains. Candidate and reference capture identities tie the exact objects and original link commands to their fixture manifests. The completed stage4 measurements still expose the rejected-boxed slowdown: these machine-code checks establish their specific repairs, not whole-path throughput. [Stage4 profile, optimized Base and native audit](benchmarks/compiled-theme-stage4-codegen-linux-x86_64.json)

## Timing and attribution boundaries

The retained compact-unwind executable is 28,087,224 bytes. A fresh rebuild of its source with the preserved input profile measures 28,089,456 bytes and serves as the matched runtime reference. These two artifacts have separate hashes. Size savings against the retained artifact do not imply that a benchmark used that exact file; each raw record identifies the executable or library graph it actually used.

The full-application readiness marker follows theme construction and the first typed Button lookup. Optional normal-density and compact-density font checks run after that marker. When a parent and child share one CPU, work after the flush can still delay the parent's receipt of the marker. Stage2's startup cohorts had those optional checks enabled in both arms. Preserve those cohorts with that scope; do not attribute their entire difference to startup code.

Final startup timing clears `FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE` and `FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE` for both reference and candidate. Separate compatibility launches still verify both values for all archive cases. The embedded repeat records theme-construction time, Button lookup time and their sum inside the child, alongside external process readiness. Internal timings exclude process loading and parent scheduling; the external measurement includes them. Changing this protocol and changing source at the same time cannot establish which one caused a cross-cohort difference.

Whole-image attribution counts each linked instruction range once, together with its attributable unwind metadata, even when safe identical-code folding gives the range several Rust names. Its symbol-bearing twin must match the shipping image's executable and allocated data sections. The older pre-compaction category totals describe the 28,324,712-byte historical image; they must not be relabeled as the retained 28,087,224-byte compact-unwind image.

## Reproduction

Use the native Linux x86-64 toolchain and locked full editable consumer described in the [whole-graph release workflow](compiled-theme-performance.md#whole-graph-profile-guided-release). Train a profile from the source revision being measured. The following command intentionally omits `--use-profile`:

```sh
python3 scripts/themes/bench/profile_winui.py --source "$PWD" \
  --consumer "$work/reference/consumer" --work-dir "$work" \
  --use-opt-level z --compact-unwind
```

An explicit `--use-profile` remains useful for a fixed-input diagnostic comparison. It does not establish that moved internal functions still receive their original profile counts. Record the source hashes, toolchain, profile hash, dependency identities and complete executable hash for either method.

The public registration probes can be built against preserved production library graphs, with the corresponding profile supplied for each revision. Each graph must contain the exact Base rlib and its dependencies; use the repeatable dependency-directory arguments for preserved host macro or native inputs. The comparison driver records compiler/linker identities and input hashes, validates the fixture assertions, and checks whether the guarded unwind compaction actually applied. The optional `--capture-native` flag preserves the original final-LTO object and exact compiler-driver arguments without changing linker flags; supply it on both build and measurement invocations. Its capture requires a fresh work directory, and measurement verifies the captured-object hashes.

```sh
python3 scripts/themes/bench/compare_registration.py \
  --reference-graph "$reference_rlibs" --reference-profile "$reference_profile" \
  --candidate-graph "$candidate_rlibs" --candidate-profile "$candidate_profile" \
  --work-dir "$work/registration" --out "$work/registration.json" \
  --cpu "$cpu" --pairs 31 --build-only
# After other compilation has finished, repeat with --measure-only.
```

Measurement alternates reference and candidate in fresh processes and rotates the workload order on one allowed CPU. The element/property probe reports thread CPU time and wall time per operation; the interface probe reports wall time per class registration and successful conversion. Two warmup rounds are excluded. Allocation accounting covers one warmed pass over 32 interface declarations, so it measures replacement-adapter cost rather than first-time registry capacity growth. The resulting JSON retains the samples and paired bootstrap intervals. These microbenchmarks supplement the complete application and public API checks.

The assembled report also retains the small measurement recipes, QA entry and dependent modules, registry harness, unwind probe and generated capture wrappers as a text appendix deduplicated by SHA-256. Each original path and hash remains recorded; temporary paths are not claimed to be portable. The retained QA materializer reproduces all 21 current fixture files at the recorded location. Nineteen also match the original historical as-built hashes; the two path-bearing wrappers have different hashes after fixture-path substitution. Current producer hashes establish the later runs' exact inputs independently. The separate development manifest records the same QA sources and its additional feature/debug configuration.
