//! Observations of actual floating, one-handed, split and resize controls.
use super::{ProbeResult, require};
use ferroboard_controls::ScreenKeyboardHost;
use ferroboard_core::{editor::ITextInputTarget, settings::OneHandedMode};
use ferroui_base::{WeakRef, input::InputMethod};
use ferroui_controls::{Border, Canvas, Control, Grid, TextBox, automation::AutomationProperties};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
#[derive(Default)]
pub(super) struct State {
    viewport: Cell<f64>,
    keyboard_height: Cell<f64>,
    float_y: Cell<f64>,
    frame: RefCell<Option<WeakRef<Border>>>,
}
pub(super) fn verify(
    host: &ScreenKeyboardHost,
    field: &TextBox,
    original: &RefCell<Option<Weak<dyn ITextInputTarget>>>,
    state: &State,
    n: usize,
) -> ProbeResult<&'static str> {
    let labels = [
        "PASS layout docked",
        "PASS floating",
        "PASS floating clamped",
        "PASS floating moved",
        "PASS dock restored",
        "PASS one right",
        "PASS one left",
        "PASS one moved right",
        "PASS one exited",
        "PASS split",
        "PASS split gap",
        "PASS split one handed",
        "PASS split restored",
        "PASS split disabled",
        "PASS resize opened",
        "PASS resized",
        "PASS resize reset",
        "PASS resize done",
        "PASS layout typed",
    ];
    let label = *labels.get(n).ok_or("unexpected layout observation")?;
    let keyboard = host.keyboard();
    let engine = keyboard.engine();
    let settings = keyboard.settings();
    let target = engine.target().ok_or("layout action detached target")?;
    let saved = original
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("original adapter released by layout action")?;
    require(
        Rc::ptr_eq(&saved, &target)
            && field.is_focused()
            && !InputMethod::get_is_input_method_enabled(field),
        "layout action lost native target/focus/IME ownership",
    )?;
    let text = if n == 18 {
        "ztm"
    } else if n >= 10 {
        "zt"
    } else {
        "z"
    };
    require(
        field.text().as_deref() == Some(text)
            && target.text() == text.into()
            && target.selection_start() == text.len() as i32
            && target.selection_length() == 0,
        "layout gesture edited field/caret unexpectedly",
    )?;
    require(
        keyboard
            .touch_processor()
            .pressed_keys()
            .into_iter()
            .next()
            .is_none(),
        "layout gesture left a pressed contact",
    )?;
    // Follow the unchanged source composition using public native control bases.
    // No private view export, reflection lookup or test-only library API is needed.
    let root = Control::from_boxed(&keyboard.content().ok_or("keyboard content missing")?)
        .ok_or("keyboard content is not a Control")?
        .cast::<Grid>()
        .ok_or("keyboard root Grid missing")?;
    let body = root
        .children()
        .get(1)
        .cast::<Grid>()
        .ok_or("body Grid missing")?;
    let border = body
        .children()
        .get(0)
        .cast::<Border>()
        .ok_or("body padding Border missing")?;
    let inner = border
        .child()
        .ok_or("body inner content missing")?
        .cast::<Grid>()
        .ok_or("body inner Grid missing")?;
    let main = inner
        .children()
        .get(0)
        .cast::<Grid>()
        .ok_or("keyboard main Grid missing")?;
    let viewport = host
        .content()
        .ok_or("host content missing")?
        .visual_parent()
        .ok_or("content viewport missing")?
        .bounds()
        .height;
    let floating = (1..=3).contains(&n);
    require(
        host.is_floating() == floating && settings.floating() == floating,
        "floating mode mismatch",
    )?;
    if n == 0 {
        state.viewport.set(viewport);
        state.keyboard_height.set(keyboard.bounds().height);
    }
    if floating {
        let grid = keyboard
            .visual_parent()
            .ok_or("floating parent missing")?
            .cast::<Grid>()
            .ok_or("floating grid missing")?;
        let frame = grid
            .visual_parent()
            .ok_or("floating frame parent missing")?
            .cast::<Border>()
            .ok_or("floating Border missing")?;
        let width = 320.0_f64.max(
            host.bounds()
                .width
                .min(host.bounds().width * settings.floating_width_ratio()),
        );
        require(
            keyboard.width() == width && frame.width() == width + 2.0,
            "source floating width/border formula mismatch",
        )?;
        require(
            viewport == host.bounds().height && viewport > state.viewport.get(),
            "floating did not restore content viewport",
        )?;
        if n == 1 {
            let handle = grid
                .children()
                .get(0)
                .cast::<Border>()
                .ok_or("floating drag handle missing")?;
            // Identifiers on existing controls, never a geometry or gesture setter.
            AutomationProperties::set_name(&handle, Some("Floating drag handle"));
            AutomationProperties::set_automation_id(&handle, Some("FloatingDragHandle"));
            *state.frame.borrow_mut() = Some(frame.downgrade());
        } else {
            require(
                state
                    .frame
                    .borrow()
                    .as_ref()
                    .and_then(WeakRef::upgrade)
                    .as_ref()
                    == Some(&frame),
                "floating frame identity changed during drag",
            )?;
        }
        if n == 2 {
            require(
                Canvas::get_left(&frame) == 0.0
                    && Canvas::get_top(&frame)
                        == (host.bounds().height - frame.bounds().height).max(0.0),
                "native drag did not reach exact source left/bottom clamps",
            )?;
            state.float_y.set(Canvas::get_top(&frame));
        } else if n == 3 {
            require(
                Canvas::get_top(&frame) < state.float_y.get(),
                "captured floating drag did not move upward",
            )?;
        }
    } else {
        require(
            keyboard.visual_parent().as_ref() == Some(&host.to_ref().upcast())
                && keyboard.width().is_nan(),
            "docked parent/auto width not restored",
        )?;
    }
    let one = match n {
        5 | 7 | 11 => OneHandedMode::Right,
        6 => OneHandedMode::Left,
        _ => OneHandedMode::Off,
    };
    require(
        settings.one_handed_mode() == one,
        "native one-handed action/direction mismatch",
    )?;
    if one != OneHandedMode::Off {
        require(
            main.width() == keyboard.bounds().width * settings.one_handed_width(),
            "one-handed source width formula mismatch",
        )?;
        require(
            main.bounds().x == 0.0 || one == OneHandedMode::Right,
            "left-handed layout is not left aligned",
        )?;
        require(
            main.bounds().x > 0.0 || one == OneHandedMode::Left,
            "right-handed layout is not right aligned",
        )?;
    }
    let split = (9..=12).contains(&n);
    require(settings.split_keyboard() == split, "split setting mismatch")?;
    let gap = if split && one == OneHandedMode::Off {
        keyboard.bounds().width * settings.split_gap_ratio()
    } else {
        0.0
    };
    let computed = engine.keyboard()?;
    let left = computed
        .find_key('t' as i32).unwrap()
        .ok_or("character t missing")?
        .touch_bounds();
    let right = computed
        .find_key('y' as i32).unwrap()
        .ok_or("character y missing")?
        .touch_bounds();
    let actual_gap = right.x - (left.x + left.width);
    // Subtracting independently accumulated key edges can round a few ulps.
    require(
        (actual_gap - gap).abs() < 1e-10,
        "source split/one-handed gap width mismatch",
    )?;
    if matches!(n, 9 | 10 | 12) {
        let computed = engine.keyboard()?;
        let left = computed
            .find_key('t' as i32).unwrap()
            .ok_or("split t missing")?
            .touch_bounds();
        let right = computed
            .find_key('y' as i32).unwrap()
            .ok_or("split y missing")?
            .touch_bounds();
        let gap_x = left.x + left.width + (right.x - left.x - left.width) / 4.0;
        let snapped = computed.hit_test(gap_x, left.y + left.height / 2.0).unwrap();
        require(
            right.x > left.x + left.width
                && snapped.as_ref().is_some_and(|key| key.code() == 't' as i32),
            "split gap did not snap to nearest row key as source specifies",
        )?;
    }
    if n == 14 {
        require(
            settings.height_scale() == 1.0,
            "unexpected initial resize scale",
        )?;
        state.keyboard_height.set(keyboard.bounds().height);
    } else if n == 15 {
        if !(settings.height_scale() > 1.0
            && keyboard.bounds().height > state.keyboard_height.get())
        {
            return Err(format!(
                "native resize did not increase scale/height: scale={}, height={}, baseline={}",
                settings.height_scale(),
                keyboard.bounds().height,
                state.keyboard_height.get()
            )
            .into());
        }
    } else if n >= 16 {
        require(
            settings.height_scale() == 1.0
                && keyboard.bounds().height == state.keyboard_height.get(),
            "native resize reset did not restore scale/height",
        )?;
    }
    Ok(label)
}
