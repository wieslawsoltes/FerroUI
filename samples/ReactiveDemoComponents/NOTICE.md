# NOTICE

This crate is derived from the sample `ReactiveDemo` of ReactiveUI.Samples
(<https://github.com/reactiveui/ReactiveUI.Samples>, `wpf/getting-started/ReactiveDemo`, MIT,
Copyright (c) .NET Foundation and Contributors), the getting-started sample of the ReactiveUI
documentation, through its port `samples/ReactiveDemo`.

- The application is the one of the reference: a search box whose text is throttled for 800 ms,
  trimmed and searched, a list of results that show the details of each package (here, of each
  crate), and a link to the page of a package. It is written again with components that run
  again instead of a view model and views; the sources are not ports of the reference sources
  line by line.
- The search service, the metadata of a crate and the in-memory catalogue are those of
  `samples/ReactiveDemo` (`crate_search.rs`), which the crate depends on.
- The page of a crate, the router and the error boundary are not in the reference: the reference
  opens the page of a package in the browser of the system.
