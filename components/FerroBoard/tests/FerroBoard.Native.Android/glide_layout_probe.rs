//! Replace the actual computed keyboard while native key/glide capture stays held.
use super::{ProbeResult, State as ProbeState, emit, record_trace, require};
use ferroboard_core::{
    keys::ShiftState,
    layouts::{ComputedKey, ComputedKeyboard},
    text::SuggestionKind,
};
use ferroui_base::input::InputMethod;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

#[derive(Default)]
pub(super) struct State {
    old_keyboard: RefCell<Option<Rc<ComputedKeyboard>>>,
    held_key: RefCell<Option<Rc<ComputedKey>>>,
    base: RefCell<String>,
    word: RefCell<Option<String>>,
    popup_word: RefCell<Option<String>>,
    update_after: Cell<usize>,
    samples_before: Cell<usize>,
}

pub(super) fn toggle(state: &ProbeState) -> ProbeResult<&'static str> {
    let enabled = match state.phase.get() {
        26 => true,
        29 => false,
        _ => return Err("unexpected live number-row action".into()),
    };
    let keyboard = state.keyboard.upgrade().ok_or("layout keyboard retired")?;
    require(
        keyboard.settings().number_row() != enabled,
        "number-row action did not replace a layout",
    )?;
    keyboard.settings().set_number_row(enabled);
    Ok(if enabled { "ROW ON" } else { "ROW OFF" })
}

fn layout(keyboard: &ComputedKeyboard, name: &str) -> ProbeResult<()> {
    for key in keyboard.character_keys() {
        if let Some(text) = key.data().as_string(false)?
            && text.len() == 1
            && (97..=122).contains(&text.units()[0])
        {
            let r = key.visible_bounds();
            emit(&format!(
                "LAYOUT\tlogical\t{name}\t{}\t{:016x}\t{:016x}\t{:016x}\t{:016x}",
                text.units()[0],
                r.x.to_bits(),
                r.y.to_bits(),
                r.width.to_bits(),
                r.height.to_bits()
            ));
        }
    }
    Ok(())
}

pub(super) fn verify(state: &ProbeState) -> ProbeResult<&'static str> {
    let n = state.phase.get();
    let labels = [
        "PASS layout pressed",
        "PASS popup layout replaced",
        "PASS popup layout released",
        "PASS layout glide before",
        "PASS layout glide changed",
        "PASS layout glide preview",
        "PASS layout glide committed",
        "PASS layout recovery",
    ];
    let label = *labels.get(n - 25).ok_or("extra layout observation")?;
    let keyboard = state.keyboard.upgrade().ok_or("layout keyboard retired")?;
    let engine = keyboard.engine();
    let field = state.field.upgrade().ok_or("layout field retired")?;
    let target = engine.target().ok_or("layout target detached")?;
    let saved = state
        .adapter
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("layout adapter retired")?;
    require(
        Rc::ptr_eq(&target, &saved)
            && field.is_focused()
            && !InputMethod::get_is_input_method_enabled(&field),
        "layout replacement lost adapter/focus/IME ownership",
    )?;
    let computed = engine.keyboard()?;
    let touch = keyboard.touch_processor();
    let pressed = touch
        .pressed_keys()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let gliding = matches!(n, 28..=30);
    require(
        engine.is_gliding() == gliding && engine.can_glide(),
        "live layout glide state mismatch",
    )?;
    let captured = state
        .contact
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .is_some_and(|p| p.captured().is_some());
    require(
        captured == (gliding || n <= 26),
        "live layout native capture mismatch",
    )?;
    if pressed.len() != usize::from(n == 25) {
        return Err(format!(
            "live layout pressed-state mismatch at{n}: pressed={}, popup={}",
            pressed.len(),
            touch.popup().is_some()
        )
        .into());
    }
    require(
        engine.shift_state() == ShiftState::Unshifted,
        "layout replacement changed shift",
    )?;
    require(
        keyboard.settings().number_row() == matches!(n, 26..=28),
        "wrong live number-row setting",
    )?;
    if n == 25 {
        *state.layout.base.borrow_mut() =
            field.text().ok_or("missing layout base text")?.to_string();
        *state.layout.old_keyboard.borrow_mut() = Some(computed.clone());
        *state.layout.held_key.borrow_mut() = Some(pressed[0].clone());
    }
    if n == 26 {
        require(
            !Rc::ptr_eq(
                state.layout.old_keyboard.borrow().as_ref().unwrap(),
                &computed,
            ),
            "number row reused original keyboard object",
        )?;
        let popup = touch.popup().ok_or("long-press layout lost popup")?;
        require(
            Rc::ptr_eq(
                state.layout.held_key.borrow().as_ref().unwrap(),
                &popup.key(),
            ),
            "layout replacement substituted popup source key",
        )?;
        let selected = popup
            .keys()
            .get(popup.selected_index())?
            .as_string(false)?
            .ok_or("null live popup word")?;
        *state.layout.popup_word.borrow_mut() = Some(String::from_utf16(selected.units())?);
        let choices = popup
            .keys()
            .iter()
            .map(|key| -> ProbeResult<String> {
                let word = key?.as_string(false)?.ok_or("null live popup choice")?;
                Ok(word
                    .units()
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(","))
            })
            .collect::<ProbeResult<Vec<_>>>()?;
        emit(&format!(
            "POPUP\t{}\t{}",
            popup.selected_index(),
            choices.join("|")
        ));
    }
    if n >= 27 {
        require(touch.popup().is_none(), "released layout retained popup")?;
    }
    if n == 28 {
        *state.layout.old_keyboard.borrow_mut() = Some(computed.clone());
        state
            .layout
            .update_after
            .set(touch.glide_trail().borrow().len());
        state
            .layout
            .samples_before
            .set(engine.glide_classifier().point_count());
        emit("MODE\tlogical\t0");
        layout(&computed, "before")?;
    }
    if n == 29 {
        require(
            !Rc::ptr_eq(
                state.layout.old_keyboard.borrow().as_ref().unwrap(),
                &computed,
            ),
            "glide number-row change reused keyboard",
        )?;
        require(
            touch.glide_trail().borrow().len() == state.layout.update_after.get(),
            "layout action unexpectedly changed native trail",
        )?;
        emit(&format!(
            "UPDATE\tlogical\t{}",
            state.layout.update_after.get()
        ));
        let samples = engine.glide_classifier().point_count();
        require(
            samples == state.layout.samples_before.get(),
            "layout replacement cleared classifier samples",
        )?;
        emit(&format!(
            "SAMPLES\tlogical\t{}\t{samples}",
            state.layout.samples_before.get()
        ));
        layout(&computed, "after")?;
    }
    if n == 30 {
        record_trace(state, &keyboard, "logical", false)?;
        let best = engine
            .suggestions()
            .get(0)?
            .ok_or("live layout has no best candidate")?
            .text()
            .ok_or("null live layout candidate")?;
        *state.layout.word.borrow_mut() = Some(String::from_utf16(best.units())?);
    }
    let base = state.layout.base.borrow().clone();
    let prefix = format!(
        "{base}{}",
        state.layout.popup_word.borrow().as_deref().unwrap_or("")
    );
    let expected = match n {
        25..=26 => base,
        27..=30 => prefix,
        31 => format!(
            "{prefix} {}",
            state
                .layout
                .word
                .borrow()
                .as_ref()
                .ok_or("missing live layout word")?
        ),
        _ => format!(
            "{prefix} {}x",
            state
                .layout
                .word
                .borrow()
                .as_ref()
                .ok_or("missing live layout word")?
        ),
    };
    require(
        field.text().as_deref() == Some(expected.as_str())
            && target.text() == expected.as_str().into(),
        "live layout typing/release text mismatch",
    )?;
    require(
        target.selection_start() == expected.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "live layout UTF16 caret mismatch",
    )?;
    if n == 27 {
        emit(&format!(
            "POPUPCOMMIT\t{}",
            state
                .layout
                .popup_word
                .borrow()
                .as_ref()
                .unwrap()
                .encode_utf16()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    require(
        engine
            .language_models()
            .get_user_dictionary(&"en".into())
            .words()
            .is_empty(),
        "live layout learned despite disabled learning",
    )?;
    if n == 31 {
        let mut ranked = Vec::new();
        for suggestion in engine.suggestions().snapshot()? {
            let suggestion = suggestion.ok_or("null live layout suggestion")?;
            require(
                suggestion.kind() == SuggestionKind::Glide,
                "live layout lost glide candidates",
            )?;
            ranked.push(
                suggestion
                    .text()
                    .ok_or("null live layout word")?
                    .units()
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        require(ranked.len() == 3, "live layout candidate count mismatch")?;
        emit(&format!("RANK\tlogical\t{}", ranked.join("|")));
        emit(&format!("AFTER\tlogical\t{}", engine.shift_state().0));
        emit(&format!(
            "COMMIT\tlogical\t{}",
            state
                .layout
                .word
                .borrow()
                .as_ref()
                .unwrap()
                .encode_utf16()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if n == 32 {
        emit(&format!("END\t{}", std::process::id()));
    }
    state.phase.set(n + 1);
    Ok(label)
}
