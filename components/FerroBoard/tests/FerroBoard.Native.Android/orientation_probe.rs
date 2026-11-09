//! Real Android orientation changes; observations use native/public geometry only.
use super::{ProbeResult, State, require};
use ferroboard_core::settings::OneHandedMode;
use ferroui_base::{Point, input::InputMethod, layout::HorizontalAlignment};
use ferroui_controls::{Border, Control, Grid, ScrollViewer, presenters::ScrollContentPresenter};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
#[derive(Default)]
pub(super) struct Probe {
    phase: Cell<usize>,
    portrait_width: Cell<f64>,
    landscape_width: Cell<f64>,
    full_height: Cell<f64>,
    compact_scale: Cell<f64>,
    narrow_width: Cell<f64>,
}
impl Probe {
    pub(super) fn complete(&self) -> bool {
        self.phase.get() >= 17
    }
}
pub(super) fn verify(state: &State) -> ProbeResult<&'static str> {
    let p = &state.orientation;
    let n = p.phase.get();
    let labels = [
        "PASS orientation initial",
        "PASS compact resize",
        "PASS compact height",
        "PASS compact done",
        "PASS landscape",
        "PASS landscape typed",
        "PASS landscape split",
        "PASS portrait split",
        "PASS portrait restored",
        "PASS portrait typed",
        "PASS portrait one handed",
        "PASS landscape one handed",
        "PASS landscape one moved",
        "PASS portrait one handed again",
        "PASS orientation exited",
        "PASS orientation reset",
        "PASS orientation typed",
    ];
    let label = *labels.get(n).ok_or("extra orientation observation")?;
    let keyboard = state.host.keyboard();
    let engine = keyboard.engine();
    let settings = keyboard.settings();
    let field = &state.snapshot.field;
    let text = match n {
        0..=2 => "ztmnpx",
        3..=4 => "ztmnpxy",
        5..=8 => "ztmnpxyl",
        9..=15 => "ztmnpxylz",
        _ => "ztmnpxylzc",
    };
    let target = engine
        .target()
        .ok_or("orientation detached editing target")?;
    let original = state
        .snapshot
        .adapter
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("orientation released original adapter")?;
    require(
        Rc::ptr_eq(&original, &target)
            && field.is_focused()
            && !InputMethod::get_is_input_method_enabled(field),
        "orientation changed native target/focus/IME ownership",
    )?;
    require(
        field.text().as_deref() == Some(text)
            && target.text() == text.into()
            && target.selection_start() == text.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "orientation text/UTF16 caret mismatch",
    )?;
    require(
        keyboard
            .touch_processor()
            .pressed_keys()
            .into_iter()
            .next()
            .is_none(),
        "orientation left a held typing contact",
    )?;
    require(
        !state.host.is_floating() && state.generation == 3,
        "orientation replaced hosted fixture",
    )?;
    let landscape = matches!(n, 4..=6 | 11..=12);
    require(
        (state.host.bounds().width > state.host.bounds().height) == landscape,
        "native host orientation aspect mismatch",
    )?;
    let root = Control::from_boxed(&keyboard.content().ok_or("keyboard content missing")?)
        .ok_or("keyboard root missing")?
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
        .ok_or("body padding missing")?;
    let inner = border
        .child()
        .ok_or("body content missing")?
        .cast::<Grid>()
        .ok_or("body inner Grid missing")?;
    let main = inner
        .children()
        .get(0)
        .cast::<Grid>()
        .ok_or("main Grid missing")?;
    let overlay = root
        .children()
        .get(2)
        .cast::<Grid>()
        .ok_or("resize overlay missing")?;
    require(
        overlay.is_visible() == matches!(n, 1 | 2 | 15),
        "orientation resize overlay visibility mismatch",
    )?;
    if n == 0 {
        p.portrait_width.set(keyboard.bounds().width);
        p.full_height.set(keyboard.bounds().height);
        require(
            settings.height_scale() == 1.0,
            "orientation initial scale mismatch",
        )?;
    } else if n == 2 {
        require(
            (0.5..1.0).contains(&settings.height_scale())
                && keyboard.bounds().height < p.full_height.get(),
            "real compact drag did not reduce keyboard",
        )?;
        p.compact_scale.set(settings.height_scale());
    }
    if (3..=14).contains(&n) {
        require(
            settings.height_scale() == p.compact_scale.get(),
            "rotation changed compact setting",
        )?;
    }
    if n >= 15 {
        require(
            settings.height_scale() == 1.0 && keyboard.bounds().height == p.full_height.get(),
            "reset did not restore portrait height",
        )?;
    }
    if n == 4 {
        require(
            keyboard.bounds().width > p.portrait_width.get(),
            "landscape did not widen keyboard",
        )?;
        p.landscape_width.set(keyboard.bounds().width);
    }
    if matches!(n, 7..=10 | 13..=16) {
        require(
            keyboard.bounds().width == p.portrait_width.get(),
            "portrait width not restored",
        )?;
    }
    let one = match n {
        10 | 11 => OneHandedMode::Left,
        12 | 13 => OneHandedMode::Right,
        _ => OneHandedMode::Off,
    };
    require(
        settings.one_handed_mode() == one && settings.split_keyboard() == matches!(n, 6 | 7),
        "orientation mode settings changed",
    )?;
    if one != OneHandedMode::Off {
        require(
            main.width() == keyboard.bounds().width * settings.one_handed_width()
                && main.horizontal_alignment()
                    == if one == OneHandedMode::Left {
                        HorizontalAlignment::Left
                    } else {
                        HorizontalAlignment::Right
                    },
            "one-handed width/alignment did not track orientation",
        )?;
        if n == 10 {
            p.narrow_width.set(main.width());
        }
        if n == 11 {
            require(
                main.width() > p.narrow_width.get(),
                "landscape kept stale portrait one-handed width",
            )?;
        }
        if n == 13 {
            require(
                main.width() == p.narrow_width.get(),
                "portrait one-handed width did not recover",
            )?;
        }
    } else {
        require(
            main.width().is_nan(),
            "full-width mode retained explicit narrow width",
        )?;
    }
    let computed = engine.keyboard()?;
    require(
        computed.width() > 0.0 && computed.width() <= keyboard.bounds().width,
        "computed keyboard width invalid after orientation",
    )?;
    let left = computed
        .find_key('t' as i32).unwrap()
        .ok_or("orientation t key missing")?
        .touch_bounds();
    let right = computed
        .find_key('y' as i32).unwrap()
        .ok_or("orientation y key missing")?
        .touch_bounds();
    let expected_gap = if matches!(n, 6 | 7) {
        keyboard.bounds().width * settings.split_gap_ratio()
    } else {
        0.0
    };
    require(
        (right.x - left.x - left.width - expected_gap).abs() < 1e-10,
        "rotation retained stale split geometry",
    )?;
    let viewer = state
        .host
        .content()
        .ok_or("orientation scroll viewer missing")?
        .cast::<ScrollViewer>()
        .ok_or("orientation content is not ScrollViewer")?;
    let presenter = viewer
        .get_visual_descendants()
        .find_map(|v| v.downcast::<ScrollContentPresenter>().ok())
        .ok_or("orientation presenter missing")?;
    let top = field
        .translate_point(Point::default(), &presenter)
        .ok_or("orientation field transform missing")?;
    let bottom = field
        .translate_point(Point::new(0.0, field.bounds().height), &presenter)
        .ok_or("orientation field bottom transform missing")?;
    require(
        viewer.viewport().height > 0.0
            && top.y >= -1.0
            && bottom.y <= viewer.viewport().height + 1.0,
        "orientation hid editing field outside viewport",
    )?;
    p.phase.set(n + 1);
    Ok(label)
}
