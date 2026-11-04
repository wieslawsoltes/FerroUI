# Shared runtime code for the editable WinUI size budget

The next release target is **10,000,000 bytes for the complete stripped Linux
x86-64 executable**, including the editable WinUI runtime and all theme
documents. The last validated shipping result remains **27,406,416 bytes** at
the stage5 checkpoint documented in
[the architecture report](compiled-theme-rust-architecture.md). The source
changes described here require a fresh profile and full executable measurement;
fixture savings and the compressed distribution size do not establish progress
against that budget.

## Source checkpoint

The runtime and focused regression tests are committed through
[`6401e654`](https://github.com/wieslawsoltes/FerroUI/commit/6401e654c148a473f33358b79f5cf886d5f9d86b),
with test-only corrections through
[`d2d2f83d`](https://github.com/wieslawsoltes/FerroUI/commit/d2d2f83d606f547b4ebda2323a145b14ca8379ad).
The production comparison freezes that revision on the existing PR branch.
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
