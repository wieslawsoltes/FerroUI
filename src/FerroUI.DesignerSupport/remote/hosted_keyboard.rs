use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::{Control, Window};
use ferroui_inspect::{KeyboardFocus, KeyboardInput, keyboard_input};
use ferroui_markup_xaml::{
    XamlLoadError, source_overlay::SourceSnapshot, xaml_error_scope::with_xaml_errors,
};
use ferroui_remote_protocol::designer::{
    DESIGN_KEYBOARD_QUERY, DESIGN_KEYBOARD_VERSION, DesignKeyboardFocus, DesignKeyboardRequest,
    DesignKeyboardResult,
};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
};
use url::Url;

ferroui_base::ferro_thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static ACCEPTED: RefCell<Option<(i64, Option<SourceSnapshot>)>> = const { RefCell::new(None) };
}

pub(super) fn configure(uri: Option<&Url>, source_snapshots: bool) -> Option<bool> {
    let enabled = source_snapshots
        && uri.is_some_and(|uri| {
            uri.query_pairs().any(|(name, version)| {
                name == DESIGN_KEYBOARD_QUERY && version == DESIGN_KEYBOARD_VERSION
            })
        });
    ENABLED.set(enabled);
    ACCEPTED.with(|target| *target.borrow_mut() = None);
    enabled.then_some(true)
}

pub(super) fn accept(revision: Option<i64>, sources: Option<SourceSnapshot>) {
    ACCEPTED.with(|target| {
        *target.borrow_mut() = revision
            .filter(|_| ENABLED.get())
            .map(|revision| (revision, sources));
    });
}

pub(super) fn deliver(
    request: &DesignKeyboardRequest,
    window: Option<Ref<Window>>,
) -> DesignKeyboardResult {
    let mut reply = DesignKeyboardResult {
        revision: request.revision,
        request: request.request,
        ..Default::default()
    };
    match input(request, window) {
        Ok(focus) => {
            reply.focus = focus.map(|focus| DesignKeyboardFocus {
                name: focus.name,
                role: focus.role,
            })
        }
        Err(error) => reply.error = Some(error),
    }
    reply
}

fn input(
    request: &DesignKeyboardRequest,
    window: Option<Ref<Window>>,
) -> Result<Option<KeyboardFocus>, String> {
    let sources = ACCEPTED
        .with(|target| {
            target.borrow().as_ref().and_then(|(revision, sources)| {
                (*revision == request.revision).then(|| sources.clone())
            })
        })
        .ok_or("The hosted keyboard scope changed. Wait for the current preview.")?;
    let scope = window
        .filter(|window| window.platform_impl().is_some())
        .ok_or("The application preview window retired")?
        .upcast::<Control>();
    let action = match (&request.key, &request.text) {
        (None, None) => KeyboardInput::Focus,
        (Some(key), None) => KeyboardInput::Key { key: key.clone() },
        (None, Some(text)) => KeyboardInput::Text { text: text.clone() },
        _ => return Err("Supply a key gesture or text".into()),
    };
    catch_unwind(AssertUnwindSafe(|| {
        let deliver = || {
            Dispatcher::ui_thread().capture_unhandled_exceptions(|| {
                with_xaml_errors(|| {
                    keyboard_input(&scope, action).map_err(XamlLoadError::with_message)
                })
                .map_err(|error| error.to_string())
            })
        };
        match sources {
            Some(sources) => sources.run(deliver),
            None => deliver(),
        }
    }))
    .map_err(|_| "The application could not process the preview keyboard input")??
}
