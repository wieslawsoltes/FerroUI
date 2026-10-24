# Transition ownership and native cancellation audit

This record covers WR-26, the transition lifecycle correction reviewed during
the 2026-10-20 continuation of PR #269. The acceptance source remains
[`microsoft-ui-xaml@7e4b0c2096be402f51cc8f7d203ebf67cff183d0`](https://github.com/microsoft/microsoft-ui-xaml/tree/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater).
The correction combines the source's transition notifications and factory
ordering with FerroUI's animation values, cancellation tokens and recyclable
native controls.

## Source contracts and the cross-feature failure

| Pinned WinUI source | Contract retained in FerroUI |
|---|---|
| [`TransitionManager::ClearElement` and `OnTransitionProviderTransitionCompleted`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/TransitionManager.cpp) | An accepted removal owns its element until completion. Completion returns it to the factory and invalidates arrange. A provider can be shared by multiple repeaters. |
| [`ViewManager::ClearElementToAnimator` and `ClearElementToElementFactory`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/ViewManager.cpp) | Removal ownership is assigned after queueing. ElementClearing observes the original element before the factory receives it. |
| [`ItemsRepeater::OnItemTemplateChanged`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/ItemsRepeater.cpp) | The old layout releases its references and returns its controls while the old ItemTemplateShim remains installed. The new factory is installed afterward. |
| [`ItemCollectionTransitionProvider::QueueTransition`, `OnRendering` and `StartNewKeepAliveTimer`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/ItemCollectionTransitionProvider.cpp) | Queued transitions that do not start receive completion on the rendering frame. Started transitions retain the full five-second batch lifetime required by weak progress handles. |
| [`ItemCollectionTransitionProgress::Complete`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/ItemCollectionTransitionProgress.cpp) | Complete forwards a notification on every call while the transition and its provider remain available. Public completion is not made idempotent. |
| [`LinedFlowLayoutItemCollectionTransitionProvider`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/LinedFlowLayoutItemCollectionTransitionProvider.cpp) | Add, remove and move choreography is implemented through the native rendering clock; completing or replacing a native animation releases its transient values. |

Template replacement can clear an element that a previous removal still owns.
Returning that element to the old factory without stopping its queued or active
transition lets an old animation mutate a reused control. Checking only the
control's current parent and Animator ownership is insufficient: the same
control can be reused and removed again before an old progress handle reports
another completion. That old completion must not return the new removal's
element to a factory.

Changing ItemTransitionProvider presents a related problem. The pinned
`OnTransitionProviderChanged` unsubscribes from the old provider without first
returning its pending removals. A queued or running old removal can then finish
without any handler returning its Animator-owned element to the factory.

The pinned implementation does not provide a per-element provider cancellation
hook, a removal-generation comparison or a drain before provider replacement.
WR-26 explicitly adds those native lifecycle protections. They are recorded as
a correction required by the combined template, recycling and animation
behavior, rather than attributed to an existing WinRT API.

## Native implementation

`TransitionManager` records the exact `Rc<ItemCollectionTransition>` identity
that moved each element to Animator ownership. Its completion handler removes
that identity before calling back into the repeater. Notifications for another
host, a previous removal or an already completed removal still reach public
subscribers, but cannot recycle the current element incarnation.

Before `ViewManager::clear_element_to_element_factory` raises ElementClearing,
it calls `TransitionManager::on_element_clearing`. The manager first forgets the
pending removal identity and takes its tracked transitions out of the map. It
then cancels work for that element through the current provider and any older
providers still associated with those transitions. No transition-map borrow is
held while provider or application callbacks execute.

Provider replacement cancels the old provider's tracked work before changing
subscriptions. It returns pending removals once through the repeater's current
factory and preserves realized controls that were only adding or moving. The
cleanup drains work queued by its completion and factory callbacks while the
old provider remains installed. A version token allows a reentrant provider
change to supersede the original request; the outer request cannot overwrite
the newer provider after its callbacks return. Canceling one repeater's work
does not stop another repeater's elements in a shared provider.

Provider cancellation marks both queued transitions and transitions currently
being dispatched. `StartTransitions` filters canceled entries before invoking
the subclass. The dispatch scope also covers direct public StartTransitions
calls and nested provider callbacks; it does not extend those direct calls'
transition lifetime after the call returns. The lined-flow provider checks
cancellation again before each element starts, and its animation manager checks
again after completing a replaced animation. Those checks prevent a completion
callback from clearing a control and then allowing a later entry in the same
batch to animate it.

The optional `ItemCollectionTransitionProviderImpl::cancel_transitions_core`
override is a FerroUI lifecycle extension with a no-op base implementation.
Providers that own native animation resources implement it to stop work for
the supplied element. The built-in lined-flow provider removes only that
element from `AnimationManager`, cancels its `Animation` through the existing
`CancellationTokenSource`, and disposes its Animation-priority transform,
transform origin and opacity values. Disposing those values reveals the
application's current bindings and styles before the old factory can reuse the
control. The same hook releases the old provider's values when its provider
property changes, even for controls that remain realized. Other elements using
the same provider continue animating.

The base provider can suppress pending framework dispatches; it cannot dispose
arbitrary resources owned by a custom subclass that does not implement the
cancellation override. Custom providers remain responsible for stopping their
own effects when the override is called, including any work they defer beyond
StartTransitions. This responsibility is separate from the framework's exact
removal identity check, which applies to all providers.

Canceling an unstarted queued transition preserves its frame-deferred
completion. Canceling a started built-in animation releases its values
synchronously and reports its ordinary completion. Both paths leave the public
Progress::Complete behavior unchanged. Started queued transitions retain the
source's five-second keepalive even after cancellation; retaining an old public
progress handle never grants ownership of a later reuse of the same control.

## Behavioral regressions

The following regressions use FerroUI's controls, native animation clock,
dispatcher, factories and transition provider. They are in
[`services_tests.rs`](../../../src/FerroUI.Controls.Repeater/services_tests.rs).

| Regression | Observable guarantee |
|---|---|
| `removal_completion_invalidates_arrange_and_does_not_recycle_a_reused_element_twice` | Repeating an old Complete while the same control is Animator-owned by a newer removal still raises public notifications, while only the newer removal returns the control to its factory. |
| `template_replacement_cancels_queued_transitions_without_affecting_shared_hosts` | Queued Add, Move and Remove transitions for controls from the old template are never passed to the provider's start override. Their unstarted completions remain frame-deferred, and another repeater sharing the provider completes normally. |
| `clearing_transition_provider_recycles_pending_removals_before_their_render_frame` | Removing a provider returns pending removals immediately, preserves the remaining realized controls, suppresses their old queued work and leaves the shared provider's other host intact. |
| `replacing_transition_provider_stops_old_animations_and_preserves_new_removal_identity` | Replacing an active provider restores native values and returns its removal exactly once. A late old completion cannot recycle a subsequent removal under the new provider, while the old shared provider continues animating its other host. |
| `a_provider_change_from_cancellation_completion_supersedes_the_outer_change` | A provider installed by a cancellation-completion callback remains the active provider after the outer change returns. |
| `cancellation_preserves_started_progress_keepalive_and_repeated_public_completion` | Cancellation preserves the full five-second weak-handle lifetime and repeatable public completion notifications. |
| `template_replacement_stops_native_animations_before_the_old_factory_reuses_controls` | Active native remove/move animations restore current opacity bindings and the original transform/origin before ElementClearing and the old factory's recycle callback. The exact controls can be reused in another repeater while the shared provider finishes unrelated animations. Later clock ticks do not overwrite the reused controls' new values. |
| `reentrant_cancellation_prevents_later_direct_batch_transitions_from_starting` | Completion while replacing an animation can cancel both the replacement and a later element in a direct StartTransitions batch, without leaving transient property values installed. |

The factory regression starts its recycle count after the first StackLayout
measure because that layout legitimately recycles a FORCE_CREATE sizing probe.
It also records a new baseline after the destination host's initial measure.
The exact-once assertions therefore cover the template transition and later
animation ticks without counting either layout's initial size estimator as a
duplicate factory return.

Run the service suite through the actual crate:

```sh
cargo test -p ferroui-controls --lib repeater::services_tests
```

Execution results, revisions and remaining platform gates belong in
[VALIDATION.md](VALIDATION.md). This audit maps the implementation and tests;
it does not claim that Windows composition or cross-platform visual equivalence
has been measured.

## Scrolling operations

The compatibility scroll host captures the destination after native layout has
arranged the target. Its temporary made anchor is then released so intermediate
animation viewports can realize the correct items. Releasing that anchor must
not allow the requested control to be recycled before the animation arrives.

`BringIntoViewPins` owns one shared, idempotently released lease per operation.
It pins the target's realized container and each containing Repeater container,
retaining only weak owner/control references and their existing virtualization
records. Cloning pending request state shares the lease. Completion,
superseding requests, external scrolling, viewer/content replacement, target
subtree or host detachment, and host disposal release it. Removing the last
contribution invalidates the
original owner's measure so the existing pinned pool can be pruned.

Each virtualization record has a pin generation. Terminal transitions to the
element factory or animator reset pins and advance that generation; stable-key
reset ownership retains it. A stale operation cannot unpin a newly reused item.
The same identity check guards animation progress, completion and made-anchor
release, so an old weak control handle cannot redirect scrolling to a different
item after recycling. Releasing a lease while its item is temporarily in the
stable-key reset pool remains valid.

A layout can use `SUPPRESS_AUTO_RECYCLE` while it owns a row, then recycle that
row into the pinned pool. Once that transfer occurs, the layout no longer
tracks the row. Remove, Replace and Reset notifications therefore clear
affected pinned-pool rows independently of the old suppression flag. Stable-key
reset instead transfers them into the key pool, preserving pin count and
generation for keys that survive; arrange clears leftovers. This correction to
the pinned source is recorded as WR-29. It prevents a removed row from retaining
an obsolete index or blocking reuse of the exact same native control.

The aligned-scroll scenario checks target identity at the first, intermediate
and final frames, then exact final geometry, offset, realization range and zero
remaining operation pins. Additional scenarios exercise independent caller
pins, request replacement, retained detached hosts, nested Repeater pin chains,
and the same native control removed and reused while an older request is active.
The stale-request regression checks both an intermediate progress callback and
later completion. Collection tests cover remove/replace/unkeyed reset and
stable-key restoration with an independently pinned removed-key leftover.
These tests use the native animation clock; they do not wait for wall time.
