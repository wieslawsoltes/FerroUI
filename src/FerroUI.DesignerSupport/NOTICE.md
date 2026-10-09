# NOTICE

## Source code

The loader of the previewed window, the previewer entry point, the window
and the windowing platform of the previewer, its stubs and its transports are
ported from the `Avalonia.DesignerSupport` project of Avalonia (MIT; see the
`NOTICE.md` at the root of the repository).

The HTML transport (`remote/html_transport/`: the HTTP and WebSocket server,
the connection that serves the previewer page, and the web application under
`webapp/`, whose built files are in `webapp/build/`) is ported from
`Remote/HtmlTransport/` of the same project. The web application is rewritten
without the libraries the original is built on (React, MobX): nothing of
them is part of this crate.
