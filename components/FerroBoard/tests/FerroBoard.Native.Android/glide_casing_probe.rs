//! Native Shift/caps and live ancestor-transform observations for glide input.
use super::{ProbeResult, State, emit, record_trace, require};
use ferroboard_core::{keys::ShiftState, text::SuggestionKind};
use ferroui_base::{input::InputMethod, media::ScaleTransform};
use std::rc::{Rc, Weak};

pub(super) fn transform(state: &State, scaled: bool) -> ProbeResult<&'static str> {
    require(
        state.phase.get() == if scaled { 20 } else { 23 },
        "unexpected glide transform action",
    )?;
    let host = state.host.upgrade().ok_or("glide host retired")?;
    host.set_render_transform(if scaled {
        Some(ScaleTransform::with_scale(0.75, 0.75).into())
    } else {
        None
    });
    Ok(if scaled { "ZOOMED" } else { "RESTORED" })
}

pub(super) fn verify(state: &State) -> ProbeResult<&'static str> {
    let n = state.phase.get();
    let labels = [
        "PASS manual ready",
        "PASS manual preview",
        "PASS manual committed",
        "PASS caps held",
        "PASS caps ready",
        "PASS caps preview",
        "PASS caps committed",
        "PASS caps typed",
        "PASS caps exited",
        "PASS geometry before",
        "PASS geometry changed",
        "PASS geometry preview",
        "PASS geometry committed",
        "PASS geometry restored",
        "PASS case final",
    ];
    let label = *labels.get(n - 10).ok_or("extra cased glide observation")?;
    let keyboard = state.keyboard.upgrade().ok_or("cased keyboard retired")?;
    let engine = keyboard.engine();
    let host = state.host.upgrade().ok_or("cased host retired")?;
    let field = state.field.upgrade().ok_or("cased field retired")?;
    let target = engine.target().ok_or("cased glide detached target")?;
    let saved = state
        .adapter
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("cased adapter retired")?;
    require(
        Rc::ptr_eq(&saved, &target)
            && field.is_focused()
            && !InputMethod::get_is_input_method_enabled(&field),
        "cased glide lost adapter/focus/IME ownership",
    )?;
    if n == 22 {
        let best = engine
            .suggestions()
            .get(0)?
            .ok_or("changed-geometry glide has no best candidate")?
            .text()
            .ok_or("null changed-geometry word")?;
        *state.geometry_word.borrow_mut() = Some(String::from_utf16(best.units())?);
    }
    let replacement = state
        .replacement
        .borrow()
        .clone()
        .ok_or("missing initial glide alternative")?;
    let base = format!("{replacement} hellox");
    let manual = format!("{base} Hello");
    let caps = format!("{manual} HELLO");
    let capx = format!("{caps}X");
    let geometry = if n >= 22 {
        Some(
            state
                .geometry_word
                .borrow()
                .clone()
                .ok_or("missing changed-geometry word")?,
        )
    } else {
        None
    };
    let text = match n {
        10..=11 => base,
        12..=15 => manual,
        16 => caps,
        17..=21 => capx,
        22..=23 => format!("{capx} {}", geometry.as_deref().unwrap()),
        _ => format!("{capx} {}x", geometry.as_deref().unwrap()),
    };
    if field.text().as_deref() != Some(text.as_str()) || target.text() != text.as_str().into() {
        return Err(format!(
            "cased glide text at{n}: expected={text:?}, actual={:?}",
            field.text()
        )
        .into());
    }
    require(
        target.selection_start() == text.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "cased glide UTF16 caret mismatch",
    )?;
    let shift = match n {
        10 | 11 => ShiftState::ShiftedManual,
        13..=17 => ShiftState::CapsLock,
        _ => ShiftState::Unshifted,
    };
    require(
        engine.shift_state() == shift,
        "native cased glide shift transition mismatch",
    )?;
    let gliding = matches!(n, 11 | 15 | 19..=21);
    require(
        engine.can_glide() && engine.is_gliding() == gliding,
        "cased glide phase mismatch",
    )?;
    let pressed = keyboard
        .touch_processor()
        .pressed_keys()
        .into_iter()
        .count();
    require(
        pressed == usize::from(n == 13),
        "cased glide pressed-state mismatch",
    )?;
    let captured = state
        .contact
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .is_some_and(|p| p.captured().is_some());
    require(
        captured == (gliding || n == 13),
        "cased glide native capture mismatch",
    )?;
    require(
        engine
            .language_models()
            .get_user_dictionary(&"en".into())
            .words()
            .is_empty(),
        "cased glide learned with learning disabled",
    )?;
    if n == 10 {
        state
            .logical_size
            .set((keyboard.bounds().width, keyboard.bounds().height));
    }
    require(
        (keyboard.bounds().width, keyboard.bounds().height) == state.logical_size.get(),
        "ancestor scale changed logical keyboard dimensions",
    )?;
    if (20..=22).contains(&n) {
        let t = host
            .render_transform()
            .ok_or("ancestor scale missing during glide")?
            .value();
        require(
            t.m11 == 0.75 && t.m22 == 0.75,
            "unexpected live glide ancestor matrix",
        )?;
    } else {
        require(
            host.render_transform().is_none(),
            "ancestor scale not restored",
        )?;
    }
    if matches!(n, 19 | 20) {
        require(
            state.previews.get() > 0,
            "pre-transform glide has no preview",
        )?;
    }
    if matches!(n, 11 | 15 | 21) {
        let case = match n {
            11 => "shift",
            15 => "caps",
            _ => "geometry",
        };
        emit(&format!("MODE\t{case}\t{}", shift.0));
        record_trace(state, &keyboard, case, n != 21)?;
    }
    if matches!(n, 12 | 16 | 22) {
        let case = match n {
            12 => "shift",
            16 => "caps",
            _ => "geometry",
        };
        let mut ranked = Vec::new();
        for suggestion in engine.suggestions().snapshot()? {
            let suggestion = suggestion.ok_or("null cased glide suggestion")?;
            require(
                suggestion.kind() == SuggestionKind::Glide,
                "cased glide lost ranked candidates",
            )?;
            ranked.push(
                suggestion
                    .text()
                    .ok_or("null cased word")?
                    .units()
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        require(ranked.len() == 3, "cased glide candidate count mismatch")?;
        emit(&format!("RANK\t{case}\t{}", ranked.join("|")));
        emit(&format!("AFTER\t{case}\t{}", shift.0));
        let word = match n {
            12 => "Hello",
            16 => "HELLO",
            _ => geometry.as_deref().unwrap(),
        };
        emit(&format!(
            "COMMIT\t{case}\t{}",
            word.encode_utf16()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    state.phase.set(n + 1);
    Ok(label)
}
