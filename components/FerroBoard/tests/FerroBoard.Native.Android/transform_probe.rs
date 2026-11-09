//! Real ancestor transforms and captured native resize; no library state is set.
use super::{ProbeResult, State, require};
use ferroboard_core::settings::OneHandedMode;
use ferroui_base::{input::InputMethod, media::ScaleTransform};
use ferroui_controls::{Control, Grid, StackPanel, automation::AutomationProperties};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

#[derive(Default)]
pub(super) struct Probe {
    phase: Cell<usize>,
    width: Cell<f64>,
    height: Cell<f64>,
}

pub(super) fn scale(state: &State, scaled: bool) -> ProbeResult<&'static str> {
    require(
        state.transform.phase.get() == if scaled { 1 } else { 5 },
        "unexpected native transform action",
    )?;
    state.host.set_render_transform(if scaled {
        Some(ScaleTransform::with_scale(0.75, 0.75).into())
    } else {
        None
    });
    Ok(if scaled {
        "TRANSFORM SCALED"
    } else {
        "TRANSFORM RESTORED"
    })
}

pub(super) fn verify(state: &State) -> ProbeResult<&'static str> {
    let p = &state.transform;
    let n = p.phase.get();
    let labels = [
        "PASS transform initial",
        "PASS transform scaled",
        "PASS transform typed",
        "PASS transform resize",
        "PASS transform held",
        "PASS transform changed held",
        "PASS transform moved",
        "PASS transform released",
        "PASS transform reset",
        "PASS transform fresh drag",
        "PASS transform final reset",
        "PASS transform final typed",
    ];
    let label = *labels.get(n).ok_or("extra transform observation")?;
    let keyboard = state.host.keyboard();
    let engine = keyboard.engine();
    let target = engine.target().ok_or("transform detached target")?;
    let saved = state
        .snapshot
        .adapter
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("transform released editing adapter")?;
    require(
        Rc::ptr_eq(&saved, &target)
            && state.snapshot.field.is_focused()
            && !InputMethod::get_is_input_method_enabled(&state.snapshot.field),
        "transform lost native adapter/focus/IME ownership",
    )?;
    let text = match n {
        0..=1 => "ztmnpxylzc",
        2..=10 => "ztmnpxylzcb",
        _ => "ztmnpxylzcbn",
    };
    require(
        state.snapshot.field.text().as_deref() == Some(text)
            && target.text() == text.into()
            && target.selection_start() == text.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "transform edited unexpected native text/caret",
    )?;
    require(
        keyboard
            .touch_processor()
            .pressed_keys()
            .into_iter()
            .next()
            .is_none(),
        "resize transform entered typing contact state",
    )?;
    require(
        !state.host.is_floating()
            && keyboard.settings().one_handed_mode() == OneHandedMode::Off
            && !keyboard.settings().split_keyboard(),
        "transform changed keyboard mode",
    )?;
    let scaled = matches!(n, 1..=4);
    if scaled {
        let transform = state
            .host
            .render_transform()
            .ok_or("native host transform missing")?;
        require(
            transform.value().m11 == 0.75 && transform.value().m22 == 0.75,
            "native scale transform matrix mismatch",
        )?;
    } else {
        require(
            state.host.render_transform().is_none(),
            "native transform was not removed",
        )?;
    }
    if n == 0 {
        p.width.set(keyboard.bounds().width);
        p.height.set(keyboard.bounds().height);
        let toolbar = state
            .root
            .upgrade()
            .ok_or("transform fixture root missing")?
            .children()
            .get(0)
            .cast::<StackPanel>()
            .ok_or("transform toolbar missing")?;
        for (index, label) in [(1, "Zoom"), (2, "Restore")] {
            let button = toolbar
                .children()
                .get(index)
                .cast::<ferroboard_controls::primitives::TouchButton>()
                .ok_or("transform toolbar button missing")?;
            AutomationProperties::set_name(&button, Some(label));
        }
    }
    require(
        keyboard.bounds().width == p.width.get() && engine.keyboard()?.width() <= p.width.get(),
        "render transform changed logical keyboard width",
    )?;
    let settings = keyboard.settings();
    let expected = if matches!(n, 6 | 7 | 9) {
        // The selected emulator reports density 320: 96 physical pixels = 48 DIPs.
        // A press at 0.75 scale retains that coordinate frame after restoration.
        let delta = if n == 9 { 48.0 } else { 64.0 };
        ((1.0 + delta / (4.0 * settings.key_height())) * 100.0).round_ties_even() / 100.0
    } else {
        1.0
    };
    require(
        settings.height_scale() == expected,
        "captured resize used stale/live ancestor coordinate frame",
    )?;
    if expected == 1.0 {
        require(
            keyboard.bounds().height == p.height.get(),
            "render transform changed logical height",
        )?;
    } else {
        require(
            keyboard.bounds().height > p.height.get(),
            "native resize did not grow keyboard",
        )?;
    }
    let root = Control::from_boxed(
        &keyboard
            .content()
            .ok_or("transform keyboard content missing")?,
    )
    .ok_or("transform keyboard root missing")?
    .cast::<Grid>()
    .ok_or("transform keyboard Grid missing")?;
    let overlay = root
        .children()
        .get(2)
        .cast::<Grid>()
        .ok_or("transform resize overlay missing")?;
    require(
        overlay.is_visible() == (3..=10).contains(&n),
        "transform resize overlay visibility mismatch",
    )?;
    state.cancel.require_capture((4..=6).contains(&n))?;
    p.phase.set(n + 1);
    Ok(label)
}
