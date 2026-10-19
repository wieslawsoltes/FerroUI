//! `ChainLinkReaderTests` (`ReactiveUI.Binding.Tests/ObservableForProperty/ChainLinkReaderTests.cs`).
//!
//! `ChainLinkReader` reads one member of an expression chain through a getter compiled by
//! reflection. It is not ported: each step of a property path reads its own value
//! (`PathStep::get`), so none of its tests has a subject.

// Not ported: CreateGetter_ForPropertyLink_ReturnsAFetcher - reflection (ChainLinkReader compiles a
// getter from a member expression; a path step carries its getter).
// Not ported: CreateGetter_ForArrayIndexLink_ReturnsNull - expression trees (an array index
// expression; a property path has no indexers).
// Not ported: ReadValue_WithNoParent_ReturnsNull - reflection (ChainLinkReader is not ported).
// Not ported: ReadValue_WithFetcher_ReadsThroughIt - reflection (ChainLinkReader is not ported).
// Not ported: ReadValue_WithoutFetcher_SurfacesTheUnreadableLink - expression trees (an array index
// expression read without a getter).
