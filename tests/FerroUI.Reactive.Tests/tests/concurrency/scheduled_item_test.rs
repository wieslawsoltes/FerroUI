//! Ported from `Tests.System.Reactive/Tests/Concurrency/ScheduledItemTest.cs` of Rx.NET.
//! Nothing is ported: every upstream test checks null arguments or the ordering and equality
//! operators of subclasses of `ScheduledItem<TAbsolute>`, which the port's `ScheduledItem<T>` (a
//! sealed struct without comparison operators) does not have.

// Not ported: ArgumentChecking - argument validation
// Not ported: Inequalities - needs the comparison operators of ScheduledItem (<, <=, >, >=, CompareTo, comparison with null) and subclassing (InvokeCore), which the port does not have
// Not ported: Equalities - needs the equality operators of ScheduledItem (==, !=, Equals, GetHashCode) and subclassing (InvokeCore), which the port does not have
