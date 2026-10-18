//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SynchronizeTest.cs` of Rx.NET.
//! Every test of the upstream file checks the lock around the notifications of a source that
//! notifies from other threads; the port is single-threaded and `synchronize()` is the source
//! itself, so none of them applies.

// Not ported: Synchronize_ArgumentChecking - argument validation
// Not ported: Synchronize_Range - real threads (NewThreadScheduler, a lock held by the test, Thread.Sleep)
// Not ported: Synchronize_Throw - real threads (NewThreadScheduler, a lock held by the test, Thread.Sleep)
// Not ported: Synchronize_BadObservable - real threads (a source that notifies from two threads at once)
