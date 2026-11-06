//! A long native document. Focus and host layout perform all scrolling.
use super::{ProbeResult, State, require};
use ferroboard_core::editor::ITextInputTarget;
use ferroui_base::{
    Point, Ref, WeakRef,
    input::InputMethod,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    Border, Control, ScrollViewer, StackPanel, TextBox, automation::AutomationProperties,
    presenters::ScrollContentPresenter,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
pub(super) struct Probe {
    viewer: Ref<ScrollViewer>,
    first: Ref<TextBox>,
    phase: Cell<usize>,
    target: RefCell<Option<Weak<dyn ITextInputTarget>>>,
    viewport: Cell<f64>,
    offset: Cell<f64>,
}
pub(super) fn install(state: &Rc<State>) -> ProbeResult<&'static str> {
    require(
        state.phase.get() == 21 && state.scroll.borrow().is_none(),
        "scroll setup at wrong phase",
    )?;
    let original = state.host.content().ok_or("original XAML fields missing")?;
    state.host.set_content(None);
    let first = TextBox::new();
    first.set_text(Some(""));
    AutomationProperties::set_automation_id(&first, Some("FirstScrollInput"));
    AutomationProperties::set_name(&first, Some("First scroll input"));
    let spacer = Border::new();
    spacer.set_height(1000.0);
    let document = StackPanel::new();
    document.children().add(first.clone());
    document.children().add(spacer);
    document.children().add(original);
    let viewer = ScrollViewer::new();
    // Isolate host reveal from the viewer's separate focus-change convenience.
    viewer.set_bring_into_view_on_focus_change(false);
    viewer.set_content(Some(Control::boxed(document)));
    AutomationProperties::set_automation_id(&viewer, Some("FocusedFieldScroll"));
    let weak = Rc::downgrade(state);
    viewer.loaded(move |_, _| {
        if let Some(state) = weak.upgrade() {
            queue_focus(&state, state.snapshot.field.downgrade(), "SCROLL READY");
        }
    });
    *state.scroll.borrow_mut() = Some(Rc::new(Probe {
        viewer: viewer.clone(),
        first,
        phase: Cell::new(0),
        target: RefCell::new(None),
        viewport: Cell::new(0.0),
        offset: Cell::new(0.0),
    }));
    state.host.set_content(Some(viewer.upcast()));
    Ok("SCROLL INSTALLING")
}
fn queue_focus(state: &Rc<State>, field: WeakRef<TextBox>, ready: &'static str) {
    let weak = Rc::downgrade(state);
    Dispatcher::current_dispatcher().post_local(
        move || {
            if let (Some(state), Some(field)) = (weak.upgrade(), field.upgrade()) {
                field.focus();
                let weak = Rc::downgrade(&state);
                // Host focus handling enqueues its reveal at background priority. Publish
                // readiness after that request, without setting any scroll/layout values.
                Dispatcher::current_dispatcher().post_local(
                    move || {
                        if let Some(state) = weak.upgrade()
                            && let Some(status) = state.status.upgrade()
                        {
                            status.set_text(Some(ready));
                        }
                    },
                    DispatcherPriority::BACKGROUND,
                );
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}
pub(super) fn focus(state: &Rc<State>, first: bool) -> ProbeResult<&'static str> {
    let probe = state
        .scroll
        .borrow()
        .clone()
        .ok_or("scroll fixture not installed")?;
    let field = if first {
        &probe.first
    } else {
        &state.snapshot.field
    };
    queue_focus(
        state,
        field.downgrade(),
        if first { "FOCUS FIRST" } else { "FOCUS LAST" },
    );
    Ok("FOCUS INSTALLING")
}
pub(super) fn verify(state: &State) -> ProbeResult<&'static str> {
    let labels = [
        "PASS scroll initial",
        "PASS scroll typed",
        "PASS scroll first",
        "PASS scroll first typed",
        "PASS scroll last",
        "PASS scroll resize opened",
        "PASS scroll resized",
        "PASS scroll reset",
        "PASS scroll final",
    ];
    let probe = state
        .scroll
        .borrow()
        .clone()
        .ok_or("scroll fixture not installed")?;
    let n = probe.phase.get();
    let label = *labels.get(n).ok_or("extra scroll observation")?;
    let first = matches!(n, 2 | 3);
    let active = if first {
        &probe.first
    } else {
        &state.snapshot.field
    };
    let inactive = if first {
        &state.snapshot.field
    } else {
        &probe.first
    };
    let last_text = match n {
        0 => "ztm",
        8 => "ztmnp",
        _ => "ztmn",
    };
    let first_text = if n >= 3 { "k" } else { "" };
    require(
        state.snapshot.field.text().as_deref() == Some(last_text)
            && probe.first.text().as_deref() == Some(first_text),
        "focus/scroll edited wrong native field",
    )?;
    require(
        active.is_focused()
            && !InputMethod::get_is_input_method_enabled(active)
            && InputMethod::get_is_input_method_enabled(inactive),
        "scroll focus/IME ownership mismatch",
    )?;
    let keyboard = state.host.keyboard();
    let target = keyboard
        .engine()
        .target()
        .ok_or("scroll native target missing")?;
    let text = if first { first_text } else { last_text };
    require(
        target.text() == text.into()
            && target.selection_start() == text.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "scroll target text/UTF16 selection mismatch",
    )?;
    if matches!(n, 0 | 2 | 4) {
        if n != 0 {
            require(
                probe
                    .target
                    .borrow()
                    .as_ref()
                    .is_some_and(|old| old.upgrade().is_none()),
                "focus replacement retained old adapter",
            )?;
        }
        *probe.target.borrow_mut() = Some(Rc::downgrade(&target));
        *state.snapshot.adapter.borrow_mut() = Some(Rc::downgrade(&target));
    }
    let saved = probe
        .target
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("active scroll adapter lost")?;
    require(
        Rc::ptr_eq(&saved, &target),
        "scroll/resize replaced adapter without focus change",
    )?;
    let presenter = probe
        .viewer
        .get_visual_descendants()
        .find_map(|v| v.downcast::<ScrollContentPresenter>().ok())
        .ok_or("native scroll presenter missing")?;
    let top = active
        .translate_point(Point::default(), &presenter)
        .ok_or("active field transform missing")?;
    let bottom = active
        .translate_point(Point::new(0.0, active.bounds().height), &presenter)
        .ok_or("active field bottom transform missing")?;
    require(
        top.y >= -1.0 && bottom.y <= probe.viewer.viewport().height + 1.0,
        "host left focused field outside native viewport",
    )?;
    require(
        if first {
            probe.viewer.offset().y == 0.0
        } else {
            probe.viewer.offset().y > 0.0
        },
        "host did not reveal distant focused field",
    )?;
    if n == 5 {
        probe.viewport.set(probe.viewer.viewport().height);
        probe.offset.set(probe.viewer.offset().y);
        require(
            keyboard.settings().height_scale() == 1.0,
            "scroll resize started with wrong scale",
        )?;
    } else if n == 6 {
        require(
            keyboard.settings().height_scale() > 1.0
                && probe.viewer.viewport().height < probe.viewport.get()
                && probe.viewer.offset().y > probe.offset.get(),
            "growing keyboard did not shrink viewport and re-reveal focus",
        )?;
    } else if n >= 7 {
        require(
            keyboard.settings().height_scale() == 1.0
                && probe.viewer.viewport().height == probe.viewport.get(),
            "reset did not restore scroll viewport",
        )?;
    }
    probe.phase.set(n + 1);
    Ok(label)
}

pub(super) fn complete(state: &State) -> bool {
    state
        .scroll
        .borrow()
        .as_ref()
        .is_some_and(|probe| probe.phase.get() >= 9)
}
