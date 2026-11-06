//! Native glide observations through public controls, engine state and touch trails.
use ferroboard_controls::{OnScreenKeyboard, ScreenKeyboardHost, primitives::TouchButton};
use ferroboard_core::{editor::ITextInputTarget, text::SuggestionKind};
use ferroui_android::log::{self, LogPriority};
use ferroui_base::{
    Ref, Thickness, WeakRef,
    input::{IPointer, InputElement, InputMethod},
    interactivity::RoutingStrategies,
};
use ferroui_controls::{
    Control, Grid, GridLength, RowDefinition, StackPanel, TextBlock, TextBox,
    automation::AutomationProperties,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
#[path = "glide_casing_probe.rs"]
mod casing;
#[path = "glide_layout_probe.rs"]
mod layout;
type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;
struct State {
    layout: layout::State,
    phase: Cell<usize>,
    host: WeakRef<ScreenKeyboardHost>,
    geometry_word: RefCell<Option<String>>,
    logical_size: Cell<(f64, f64)>,
    keyboard: WeakRef<OnScreenKeyboard>,
    field: WeakRef<TextBox>,
    adapter: RefCell<Option<Weak<dyn ITextInputTarget>>>,
    contact: RefCell<Option<Weak<dyn IPointer>>>,
    available: RefCell<Vec<(u64, u64)>>,
    press: Cell<Option<(u64, u64)>>,
    replacement: RefCell<Option<String>>,
    previews: Cell<usize>,
}
fn emit(line: &str) {
    log::write(LogPriority::Info, "FerroBoardGlide", line);
}
fn require(ok: bool, message: &'static str) -> ProbeResult<()> {
    if ok { Ok(()) } else { Err(message.into()) }
}
pub fn view() -> Ref<Control> {
    let value = ferroboard_native_consumer::compiled_xaml::try_load(
        None,
        &format!(
            "{}PrivacyHosted.xaml",
            ferroboard_native_consumer::compiled_xaml::ROOT_URI
        ),
    )
    .unwrap()
    .unwrap();
    let host = value
        .downcast_ref::<Ref<ScreenKeyboardHost>>()
        .unwrap()
        .clone();
    let fields = host.content().unwrap().cast::<StackPanel>().unwrap();
    let field = fields.children().get(5).cast::<TextBox>().unwrap();
    field.set_text(Some(""));
    let keyboard = host.keyboard();
    let engine = keyboard.engine();
    let settings = keyboard.settings();
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_glide_typing(true);
    settings.set_glide_preview(true);
    settings.set_show_suggestions(true);
    settings.set_suggestion_count(3);
    settings.set_learn_words(false);
    settings.set_clipboard_history_enabled(false);
    let root = Grid::new();
    root.set_margin(Thickness::new(12.0, 32.0, 12.0, 0.0));
    for height in [GridLength::AUTO, GridLength::AUTO, GridLength::STAR] {
        root.row_definitions()
            .add(RowDefinition::with_height(height));
    }
    let toolbar = StackPanel::new();
    toolbar.set_orientation(ferroui_base::layout::Orientation::Horizontal);
    let check = TouchButton::new();
    check.set_width(95.0);
    check.set_height(40.0);
    AutomationProperties::set_name(&check, Some("Check"));
    let status = TextBlock::new();
    status.set_text(Some("READY"));
    AutomationProperties::set_automation_id(&status, Some("GlideStatus"));
    Grid::set_row(&status, 1);
    Grid::set_row(&host, 2);
    toolbar.children().add(check.clone());
    root.children().add(toolbar.clone());
    root.children().add(status.clone());
    root.children().add(host.clone());
    let state = Rc::new(State {
        layout: layout::State::default(),
        phase: Cell::new(0),
        host: host.downgrade(),
        geometry_word: RefCell::new(None),
        logical_size: Cell::new((0.0, 0.0)),
        keyboard: keyboard.downgrade(),
        field: field.downgrade(),
        adapter: RefCell::new(None),
        contact: RefCell::new(None),
        available: RefCell::new(Vec::new()),
        press: Cell::new(None),
        replacement: RefCell::new(None),
        previews: Cell::new(0),
    });
    for (label, scaled) in [("Zoom", true), ("Restore", false)] {
        let button = TouchButton::new();
        button.set_width(95.0);
        button.set_height(40.0);
        AutomationProperties::set_name(&button, Some(label));
        let weak = Rc::downgrade(&state);
        let status = status.downgrade();
        button.click(move |_| {
            if let Some(state) = weak.upgrade()
                && let Some(status) = status.upgrade()
            {
                let result = casing::transform(&state, scaled);
                status.set_text(Some(&match result {
                    Ok(label) => label.to_owned(),
                    Err(e) => format!("FAIL {e}"),
                }));
            }
        });
        toolbar.children().add(button);
    }
    let row = TouchButton::new();
    row.set_width(95.0);
    row.set_height(40.0);
    AutomationProperties::set_name(&row, Some("Row"));
    let weak = Rc::downgrade(&state);
    let row_status = status.downgrade();
    row.click(move |_| {
        if let Some(state) = weak.upgrade()
            && let Some(status) = row_status.upgrade()
        {
            status.set_text(Some(&match layout::toggle(&state) {
                Ok(label) => label.to_owned(),
                Err(e) => format!("FAIL {e}"),
            }));
        }
    });
    toolbar.children().add(row);
    let weak = Rc::downgrade(&state);
    host.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, e| {
            if let Some(state) = weak.upgrade() {
                *state.contact.borrow_mut() = Some(Rc::downgrade(e.pointer()));
                state.previews.set(0);
                state.available.borrow_mut().clear();
                state.press.set(e.pointer().captured().map(|target| {
                    let p = e.get_position(Some(&target));
                    (p.x.to_bits(), p.y.to_bits())
                }));
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let weak = Rc::downgrade(&state);
    host.add_handler_with(
        InputElement::pointer_moved_event(),
        move |_, e| {
            if let Some(state) = weak.upgrade()
                && let Some(target) = e.pointer().captured()
            {
                state.available.borrow_mut().extend(
                    e.get_intermediate_points(Some(&target))
                        .into_iter()
                        .map(|p| (p.position.x.to_bits(), p.position.y.to_bits())),
                );
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let weak = Rc::downgrade(&state);
    engine.suggestions_changed().add(Rc::new(move |engine| {
        if engine.is_gliding()
            && let Some(state) = weak.upgrade()
        {
            state.previews.set(state.previews.get() + 1);
        }
    }));
    let weak_status = status.downgrade();
    check.click(move |_| {
        let result = verify(&state);
        if let Some(status) = weak_status.upgrade() {
            status.set_text(Some(&match result {
                Ok(label) => label.to_owned(),
                Err(e) => format!("FAIL {e}"),
            }));
        }
    });
    let field = field.downgrade();
    host.loaded(move |_, _| {
        if let Some(field) = field.upgrade() {
            field.focus();
        }
    });
    root.upcast()
}
fn verify(state: &State) -> ProbeResult<&'static str> {
    let n = state.phase.get();
    if n >= 25 {
        return layout::verify(state);
    }
    if n >= 10 {
        return casing::verify(state);
    }
    let labels = [
        "PASS initial",
        "PASS pressed",
        "PASS batch preview",
        "PASS batch committed",
        "PASS replacement",
        "PASS unbatched preview",
        "PASS unbatched committed",
        "PASS cancel preview",
        "PASS canceled",
        "PASS recovered",
    ];
    let label = *labels.get(n).ok_or("extra glide observation")?;
    let keyboard = state.keyboard.upgrade().ok_or("native keyboard retired")?;
    let engine = keyboard.engine();
    let field = state.field.upgrade().ok_or("native field retired")?;
    let target = engine.target().ok_or("glide detached native target")?;
    if n == 0 {
        *state.adapter.borrow_mut() = Some(Rc::downgrade(&target));
    }
    let saved = state
        .adapter
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("native adapter retired")?;
    require(
        Rc::ptr_eq(&saved, &target)
            && field.is_focused()
            && !InputMethod::get_is_input_method_enabled(&field),
        "glide lost adapter/focus/IME ownership",
    )?;
    let replacement = state.replacement.borrow().clone().unwrap_or_default();
    let text = match n {
        0..=2 => String::new(),
        3 => "hello".to_owned(),
        4..=5 => replacement.clone(),
        6..=8 => format!("{replacement} hello"),
        _ => format!("{replacement} hellox"),
    };
    if n >= 4 {
        require(
            !replacement.is_empty() && replacement != "hello",
            "missing distinct glide alternative",
        )?;
    }
    if field.text().as_deref() != Some(text.as_str()) || target.text() != text.as_str().into() {
        return Err(format!(
            "glide text at{n}: expected={text:?}, actual={:?}",
            field.text()
        )
        .into());
    }
    require(
        target.selection_start() == text.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "glide UTF16 caret mismatch",
    )?;
    let gliding = matches!(n, 2 | 5 | 7);
    require(
        engine.can_glide() && engine.is_gliding() == gliding,
        "native glide phase/readiness mismatch",
    )?;
    let pressed: Vec<_> = keyboard
        .touch_processor()
        .pressed_keys()
        .into_iter()
        .collect();
    require(
        if n == 1 {
            pressed.len() == 1
        } else {
            pressed.is_empty()
        },
        "glide typing pressed state mismatch",
    )?;
    let captured = state
        .contact
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .is_some_and(|p| p.captured().is_some());
    require(
        captured == matches!(n, 1 | 2 | 5 | 7),
        "native glide capture ownership mismatch",
    )?;
    require(
        engine
            .language_models()
            .get_user_dictionary(&"en".into())
            .words()
            .is_empty(),
        "disabled learning recorded glide words",
    )?;
    if n == 0 {
        emit(&format!("BEGIN\t{}", std::process::id()));
        for key in engine.keyboard()?.character_keys() {
            let output = key.data().as_string(false)?.ok_or("null native key text")?;
            if let [c] = output.units()
                && (u16::from(b'a')..=u16::from(b'z')).contains(c)
            {
                let r = key.visible_bounds();
                emit(&format!(
                    "KEY\t{c}\t{:016x}\t{:016x}\t{:016x}\t{:016x}",
                    r.x.to_bits(),
                    r.y.to_bits(),
                    r.width.to_bits(),
                    r.height.to_bits()
                ));
            }
        }
    }
    if gliding {
        record_trace(
            state,
            &keyboard,
            match n {
                2 => "batch",
                5 => "unbatched",
                _ => "cancel",
            },
            n != 5,
        )?;
    }
    if matches!(n, 3 | 6) {
        let mut ranked = Vec::new();
        let suggestions = engine.suggestions().snapshot()?;
        if n == 3 {
            let alternative = suggestions
                .get(1)
                .and_then(Option::as_ref)
                .ok_or("missing glide alternative")?
                .text()
                .ok_or("null glide alternative")?;
            let alternative = String::from_utf16(alternative.units())?;
            let buttons: Vec<_> = keyboard
                .get_visual_descendants()
                .filter_map(|v| v.downcast::<TouchButton>().ok())
                .filter(|button| {
                    button.get_visual_descendants().any(|v| {
                        v.downcast::<TextBlock>()
                            .ok()
                            .is_some_and(|t| t.text().as_deref() == Some(alternative.as_str()))
                    })
                })
                .collect();
            require(
                buttons.len() == 1,
                "ranked glide alternative has no unique native button",
            )?;
            AutomationProperties::set_automation_id(&buttons[0], Some("GlideAlternate"));
            *state.replacement.borrow_mut() = Some(alternative);
        }
        for suggestion in suggestions {
            let suggestion = suggestion.ok_or("null glide suggestion")?;
            require(
                suggestion.kind() == SuggestionKind::Glide,
                "committed glide lost ranked suggestions",
            )?;
            ranked.push(
                suggestion
                    .text()
                    .ok_or("null glide word")?
                    .units()
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        require(ranked.len() == 3, "native glide candidate limit changed")?;
        emit(&format!(
            "RANK\t{}\t{}",
            if n == 3 { "batch" } else { "unbatched" },
            ranked.join("|")
        ));
    }
    state.phase.set(n + 1);
    Ok(label)
}

fn record_trace(
    state: &State,
    keyboard: &OnScreenKeyboard,
    case: &str,
    batched: bool,
) -> ProbeResult<()> {
    let engine = keyboard.engine();
    require(
        state.previews.get() > 0,
        "native glide never published a preview",
    )?;
    let suggestions = engine.suggestions().snapshot()?;
    require(
        !suggestions.is_empty()
            && suggestions.iter().all(|s| {
                s.as_ref()
                    .is_some_and(|s| s.kind() == SuggestionKind::Glide)
            }),
        "native glide preview suggestions missing",
    )?;
    let touch = keyboard.touch_processor();
    let trail = touch.glide_trail();
    let trace: Vec<_> = trail
        .borrow()
        .iter()
        .map(|p| (p.x.to_bits(), p.y.to_bits()))
        .collect();
    require(trace.len() > 10, "native history lost glide samples")?;
    for (x, y) in &trace {
        emit(&format!("POINT\t{case}\t{x:016x}\t{y:016x}"));
    }
    let available = state.available.borrow();
    for (x, y) in available.iter() {
        emit(&format!("RAW\t{case}\t{x:016x}\t{y:016x}"));
    }
    require(
        trace.first().copied() == state.press.get() && available.ends_with(&trace[1..]),
        "glide did not preserve available native samples after activation",
    )?;
    if batched {
        require(available.len() >= 32, "native batched history was lost")?;
    }
    Ok(())
}
