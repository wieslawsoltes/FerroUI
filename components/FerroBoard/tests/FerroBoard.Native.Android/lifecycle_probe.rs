//! Native host/activity fixture. All production controls come from dependency XAML.
use ferroboard_controls::{
    OnScreenKeyboard, ScreenKeyboardHost, primitives::TouchButton, themes::KeyboardThemes,
};
use ferroboard_core::{editor::ITextInputTarget, engine::KeyboardEngine};
use ferroui_base::{
    Ref, Thickness, WeakRef, input::InputMethod, layout::Orientation, media::SolidColorBrush,
};
use ferroui_controls::{
    Control, Grid, GridLength, RowDefinition, StackPanel, TextBlock, TextBox, TopLevel,
    automation::AutomationProperties,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
#[path = "cancel_probe.rs"]
mod cancel_probe;
#[path = "layout_probe.rs"]
mod layout_probe;
#[path = "orientation_probe.rs"]
mod orientation_probe;
#[path = "scroll_probe.rs"]
mod scroll_probe;
#[path = "transform_probe.rs"]
mod transform_probe;
type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;
thread_local! {
    static LAST: RefCell<Option<Rc<Snapshot>>> = const { RefCell::new(None) };
    static GENERATION: Cell<usize> = const { Cell::new(0) };
}
struct Snapshot {
    host: WeakRef<ScreenKeyboardHost>,
    keyboard: WeakRef<OnScreenKeyboard>,
    engine: Weak<KeyboardEngine>,
    adapter: RefCell<Option<Weak<dyn ITextInputTarget>>>,
    field: Ref<TextBox>,
}
struct State {
    host: Ref<ScreenKeyboardHost>,
    root: WeakRef<Grid>,
    status: WeakRef<TextBlock>,
    snapshot: Rc<Snapshot>,
    previous: Option<Rc<Snapshot>>,
    phase: Cell<usize>,
    loads: Cell<usize>,
    unloads: Cell<usize>,
    generation: usize,
    layout: layout_probe::State,
    scroll: RefCell<Option<Rc<scroll_probe::Probe>>>,
    cancel: Rc<cancel_probe::Probe>,
    orientation: orientation_probe::Probe,
    transform: transform_probe::Probe,
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
    .expect("dependency compiled host");
    let host = value
        .downcast_ref::<Ref<ScreenKeyboardHost>>()
        .unwrap()
        .clone();
    let fields = host.content().unwrap().cast::<StackPanel>().unwrap();
    let field = fields.children().get(5).cast::<TextBox>().unwrap();
    field.set_text(Some(""));
    let keyboard = host.keyboard();
    let settings = keyboard.settings();
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    settings.set_clipboard_history_enabled(false);
    let actions = keyboard.smartbar_actions().expect("native quick actions");
    actions
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .retain(|action| {
            action.as_ref().is_some_and(|action| {
                action.id.as_ref().is_some_and(|id| {
                    ["floating", "onehanded", "split", "resize"]
                        .iter()
                        .any(|name| *id == (*name).into())
                })
            })
        });
    let root = Grid::new();
    // The test toolbar must sit below Android's edge-to-edge status bar.
    root.set_margin(Thickness::new(12.0, 32.0, 12.0, 0.0));
    for height in [GridLength::AUTO, GridLength::AUTO, GridLength::STAR] {
        root.row_definitions()
            .add(RowDefinition::with_height(height));
    }
    let toolbar = StackPanel::new();
    toolbar.set_orientation(Orientation::Horizontal);
    let status = TextBlock::new();
    AutomationProperties::set_automation_id(&status, Some("LifecycleStatus"));
    let generation = GENERATION.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    status.set_text(Some(&format!("READY {generation}")));
    Grid::set_row(&status, 1);
    Grid::set_row(&host, 2);
    root.children().add(toolbar.clone());
    root.children().add(status.clone());
    root.children().add(host.clone());
    let snapshot = Rc::new(Snapshot {
        host: host.downgrade(),
        keyboard: keyboard.downgrade(),
        engine: Rc::downgrade(&keyboard.engine()),
        adapter: RefCell::new(None),
        field,
    });
    let previous = LAST.with(|last| last.replace(Some(snapshot.clone())));
    let state = Rc::new(State {
        host: host.clone(),
        root: root.downgrade(),
        status: status.downgrade(),
        snapshot,
        previous,
        phase: Cell::new(0),
        loads: Cell::new(0),
        unloads: Cell::new(0),
        generation,
        layout: layout_probe::State::default(),
        scroll: RefCell::new(None),
        cancel: Rc::new(cancel_probe::Probe::default()),
        orientation: orientation_probe::Probe::default(),
        transform: transform_probe::Probe::default(),
    });
    let weak = Rc::downgrade(&state);
    host.loaded(move |_, _| {
        if let Some(state) = weak.upgrade() {
            state.loads.set(state.loads.get() + 1);
            state.snapshot.field.focus();
        }
    });
    let weak = Rc::downgrade(&state);
    host.unloaded(move |_, _| {
        if let Some(state) = weak.upgrade() {
            state.unloads.set(state.unloads.get() + 1);
        }
    });
    let names = if generation >= 3 {
        ["Scroll", "First", "Last", "Check"]
    } else {
        ["Remove", "Reattach", "Release", "Check"]
    };
    for name in names {
        let button = TouchButton::new();
        button.set_width(95.0);
        button.set_height(40.0);
        button.set_text(
            name,
            16.0,
            Some(SolidColorBrush::with_color(KeyboardThemes::floris_day().key_foreground()).into()),
            None,
        );
        AutomationProperties::set_name(&button, Some(name));
        let state = state.clone();
        button.click(move |_| {
            let result = match name {
                "Scroll" => scroll_probe::install(&state),
                "First" if state.orientation.complete() => transform_probe::scale(&state, true),
                "Last" if state.orientation.complete() => transform_probe::scale(&state, false),
                "First" if state.cancel.active() => cancel_probe::steal(&state),
                "First" => scroll_probe::focus(&state, true),
                "Last" if state.cancel.active() => cancel_probe::drop_capture(&state),
                "Last" => scroll_probe::focus(&state, false),
                "Remove" => {
                    state
                        .root
                        .upgrade()
                        .expect("live root")
                        .children()
                        .remove(&state.host);
                    Ok("REMOVED")
                }
                "Reattach" => {
                    state
                        .root
                        .upgrade()
                        .expect("live root")
                        .children()
                        .add(state.host.clone());
                    Ok("REATTACHED")
                }
                "Release" => state
                    .host
                    .release_native_subscriptions()
                    .map(|_| "RELEASED"),
                _ => verify(&state),
            };
            if let Some(status) = state.status.upgrade() {
                match result {
                    Ok(label) => status.set_text(Some(label)),
                    Err(error) => status.set_text(Some(&format!("FAIL {error}"))),
                }
            }
        });
        toolbar.children().add(button);
    }
    root.upcast()
}
fn require(condition: bool, message: &'static str) -> ProbeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn verify(state: &State) -> ProbeResult<&'static str> {
    let n = state.phase.get();
    if state.generation >= 3 && n >= 21 {
        if state.cancel.complete() {
            if state.orientation.complete() {
                return transform_probe::verify(state);
            }
            return orientation_probe::verify(state);
        }
        if scroll_probe::complete(state) {
            return cancel_probe::verify(state);
        }
        return scroll_probe::verify(state);
    }
    if state.generation >= 3 && n >= 2 {
        let label = layout_probe::verify(
            &state.host,
            &state.snapshot.field,
            &state.snapshot.adapter,
            &state.layout,
            n - 2,
        )?;
        state.phase.set(n + 1);
        return Ok(label);
    }

    let labels = [
        "PASS initial",
        "PASS typed",
        "PASS removed",
        "PASS reattached",
        "PASS retyped",
        "PASS resumed",
        "PASS resume typed",
        "PASS released",
    ];
    let label = *labels.get(n).ok_or("unexpected Check")?;
    let keyboard = state.host.keyboard();
    let engine = keyboard.engine();
    let field = &state.snapshot.field;
    let text = if state.generation > 1 {
        if n == 0 { "" } else { "z" }
    } else {
        match n {
            0 => "",
            1..=3 => "a",
            4..=5 => "ab",
            _ => "abc",
        }
    };
    let actual = field.text().unwrap_or_default();
    if actual != text {
        return Err(format!(
            "native text changed across lifecycle: generation={}, phase={n}, expected={text:?}, actual={actual:?}",
            state.generation
        )
        .into());
    }
    if matches!(n, 2 | 7) {
        require(
            engine.target().is_none() && keyboard.target_element().is_none(),
            "host did not detach target",
        )?;
        require(
            InputMethod::get_is_input_method_enabled(field),
            "native IME guard was not restored",
        )?;
        require(
            state
                .snapshot
                .adapter
                .borrow()
                .as_ref()
                .is_some_and(|weak| weak.upgrade().is_none()),
            "retired adapter retained",
        )?;
    } else {
        require(
            field.is_focused() && !InputMethod::get_is_input_method_enabled(field),
            "native focus/IME ownership lost",
        )?;
        let target = engine.target().ok_or("native target missing")?;
        require(
            target.text() == text.into()
                && target.selection_start() == text.encode_utf16().count() as i32
                && target.selection_length() == 0,
            "native target text/UTF16 caret mismatch",
        )?;
        if n == 0 || n == 3 {
            *state.snapshot.adapter.borrow_mut() = Some(Rc::downgrade(&target));
        }
        let saved = state
            .snapshot
            .adapter
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or("missing saved adapter")?;
        require(
            Rc::ptr_eq(&saved, &target),
            "activity transition replaced adapter",
        )?;
    }
    require(
        keyboard
            .touch_processor()
            .pressed_keys()
            .into_iter()
            .next()
            .is_none(),
        "pressed contact survived lifecycle",
    )?;
    if n == 2 {
        require(
            TopLevel::get_top_level(Some(&state.host)).is_none() && state.unloads.get() == 1,
            "host did not leave native tree",
        )?;
    }
    if (3..=7).contains(&n) {
        require(
            state.loads.get() == 2 && state.unloads.get() == 1,
            "host load/unload count mismatch",
        )?;
    }
    if let Some(old) = state.previous.as_ref() {
        require(
            old.host.upgrade().is_none()
                && old.keyboard.upgrade().is_none()
                && old.engine.upgrade().is_none(),
            "destroyed activity retained old native keyboard graph",
        )?;
        require(
            old.adapter
                .borrow()
                .as_ref()
                .is_some_and(|target| target.upgrade().is_none())
                && InputMethod::get_is_input_method_enabled(&old.field),
            "destroyed activity retained adapter/IME guard",
        )?;
    }
    state.phase.set(n + 1);
    Ok(label)
}
