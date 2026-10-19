//! `ViewThreadObservableTests` (`ReactiveUI.Binding.Tests/Observables/ViewThreadObservableTests.cs`).
//!
//! `ViewThreadObservable` belongs to the binding wave and is not ported yet, so none of these tests is.

// Not ported: Subscribe_WithNoObserver_Throws - argument validation

// Not ported: OnNext_OnTheOwningThread_DeliversInline - needs ViewThreadObservable (binding wave)

// Not ported: OnNext_FromAnotherThread_WaitsForTheInvoker - needs ViewThreadObservable (binding wave)

// Not ported: OnNext_ABurstFromAnotherThread_WritesOnlyTheLatestValue - needs ViewThreadObservable (binding wave)

// Not ported: OnNext_OnASequencer_WaitsForTheSequencer - needs ViewThreadObservable (binding wave)

// Not ported: OnNext_ABurstOnASequencer_WritesOnlyTheLatestValue - needs ViewThreadObservable (binding wave)

// Not ported: OnNext_OnTheOwningThreadWhileAWriteWaits_ReplacesTheWaitingValue - needs ViewThreadObservable (binding wave)

// Not ported: OnCompleted_AfterAWaitingValue_WritesTheValueFirst - needs ViewThreadObservable (binding wave)

// Not ported: OnNext_RaisedWhileDraining_IsDeliveredInTheSameDrain - needs ViewThreadObservable (binding wave)

// Not ported: OnError_OnTheOwningThread_DeliversInline - needs ViewThreadObservable (binding wave)

// Not ported: OnError_FromAnotherThread_IsDeliveredByTheDrain - needs ViewThreadObservable (binding wave)

// Not ported: OnCompleted_OnTheOwningThread_DeliversInline - needs ViewThreadObservable (binding wave)

// Not ported: OnCompleted_FromAnotherThread_IsDeliveredByTheDrain - needs ViewThreadObservable (binding wave)

// Not ported: OnNext_FromAnotherThreadWhenTheHostSetsAMainThread_DrainsThere - needs ViewThreadObservable (binding wave)

// Not ported: Dispose_BeforeTheDrain_DropsWhatWasQueued - needs ViewThreadObservable (binding wave)

// Not ported: Dispose_Twice_DisposesTheSourceSubscriptionOnce - needs ViewThreadObservable (binding wave)

// Not ported: OnNext_AfterDispose_IsIgnored - needs ViewThreadObservable (binding wave)
