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
on the existing PR branch. The reference revision is
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
the release producer. The
[toolchain bootstrap](../scripts/themes/bench/bootstrap_toolchain.py) installs
checksummed official components into an explicitly selected benchmark work
directory. It records compiler and LLVM identities without changing a system
toolchain.

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
