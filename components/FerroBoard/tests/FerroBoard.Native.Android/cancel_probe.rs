//! Native capture observations; no library gesture state is changed by the fixture.
use super::{ProbeResult, State, require};
use ferroui_base::{
    Ref, WeakRef,
    input::{IPointer, InputElement, InputMethod},
    interactivity::RoutingStrategies,
};
use ferroui_controls::{
    Border, Canvas, Control, Grid, StackPanel, automation::AutomationProperties,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
#[derive(Clone)]
struct Contact {
    pointer: Weak<dyn IPointer>,
    target: WeakRef<InputElement>,
    captured_on_press: bool,
    id: i32,
}
#[derive(Default)]
pub(super) struct Probe {
    phase: Cell<usize>,
    contacts: RefCell<Vec<Contact>>,
    frame: RefCell<Option<WeakRef<Border>>>,
    foreign: RefCell<Option<WeakRef<InputElement>>>,
    scale: Cell<f64>,
    y: Cell<f64>,
}
impl Probe {
    pub(super) fn require_capture(&self, expected: bool) -> ProbeResult<()> {
        if expected {
            require(held(&self.latest()?), "native resize capture lost")
        } else {
            require(self.all_released(), "native resize capture retained")
        }
    }
    pub(super) fn complete(&self) -> bool {
        self.phase.get() >= 18
    }
    pub(super) fn active(&self) -> bool {
        self.phase.get() > 0
    }
    fn latest(&self) -> ProbeResult<Contact> {
        self.contacts
            .borrow()
            .last()
            .cloned()
            .ok_or_else(|| "native press was not observed".into())
    }
    fn all_released(&self) -> bool {
        self.contacts.borrow().iter().all(released)
    }
    fn y(&self) -> ProbeResult<f64> {
        let frame = self
            .frame
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .ok_or("floating frame released")?;
        Ok(Canvas::get_top(&frame))
    }
}
fn released(contact: &Contact) -> bool {
    contact
        .pointer
        .upgrade()
        .is_none_or(|p| p.captured().is_none())
}
fn held(contact: &Contact) -> bool {
    contact.captured_on_press
        && contact.target.upgrade().is_some_and(|target| {
            contact
                .pointer
                .upgrade()
                .is_some_and(|pointer| pointer.captured().as_ref() == Some(&target))
        })
}
fn observe(probe: &Rc<Probe>, target: Ref<InputElement>) {
    let weak = Rc::downgrade(probe);
    let expected = target.downgrade();
    target.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, event| {
            if let Some(probe) = weak.upgrade() {
                let pointer = event.pointer();
                probe.contacts.borrow_mut().push(Contact {
                    captured_on_press: pointer.captured() == expected.upgrade(),
                    target: expected.clone(),
                    id: pointer.id(),
                    pointer: Rc::downgrade(pointer),
                });
            }
        },
        RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
        true,
    );
}
pub(super) fn steal(state: &State) -> ProbeResult<&'static str> {
    let contact = state.cancel.latest()?;
    require(held(&contact), "capture was not held before native theft")?;
    let pointer = contact
        .pointer
        .upgrade()
        .ok_or("held native pointer released")?;
    let foreign = state
        .status
        .upgrade()
        .ok_or("native capture receiver missing")?
        .upcast::<InputElement>();
    pointer.capture(Some(&foreign));
    require(
        pointer.captured().as_ref() == Some(&foreign),
        "native capture transfer failed",
    )?;
    *state.cancel.foreign.borrow_mut() = Some(foreign.downgrade());
    Ok("CAPTURE STOLEN")
}
fn foreign_held(probe: &Probe) -> ProbeResult<()> {
    let contact = probe.latest()?;
    let pointer = contact
        .pointer
        .upgrade()
        .ok_or("foreign-owned pointer released")?;
    let foreign = probe
        .foreign
        .borrow()
        .as_ref()
        .and_then(WeakRef::upgrade)
        .ok_or("foreign receiver released")?;
    require(
        pointer.captured().as_ref() == Some(&foreign),
        "library released another control's capture",
    )
}
pub(super) fn drop_capture(state: &State) -> ProbeResult<&'static str> {
    foreign_held(&state.cancel)?;
    state
        .cancel
        .latest()?
        .pointer
        .upgrade()
        .ok_or("foreign pointer released")?
        .capture(None);
    require(
        released(&state.cancel.latest()?),
        "fixture-owned capture was not released",
    )?;
    Ok("CAPTURE DROPPED")
}
pub(super) fn verify(state: &State) -> ProbeResult<&'static str> {
    let probe = &state.cancel;
    let n = probe.phase.get();
    let labels = [
        "PASS cancel resize ready",
        "PASS resize held",
        "PASS resize canceled",
        "PASS resize recovers",
        "PASS resize capture stolen",
        "PASS resize ignores lost capture",
        "PASS resize lost recovery",
        "PASS cancel floating ready",
        "PASS floating held",
        "PASS floating canceled",
        "PASS floating recovers",
        "PASS floating capture stolen",
        "PASS floating ignores lost capture",
        "PASS floating lost recovery",
        "PASS overlap latest",
        "PASS overlap earlier release",
        "PASS overlap canceled",
        "PASS cancel typed",
    ];
    let label = *labels.get(n).ok_or("extra canceled-drag observation")?;
    let keyboard = state.host.keyboard();
    let target = keyboard
        .engine()
        .target()
        .ok_or("drag detached native editing target")?;
    let saved = state
        .snapshot
        .adapter
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("original editing adapter released")?;
    let text = if n == 17 { "ztmnpx" } else { "ztmnp" };
    require(
        Rc::ptr_eq(&saved, &target)
            && state.snapshot.field.is_focused()
            && !InputMethod::get_is_input_method_enabled(&state.snapshot.field),
        "canceled drag lost native target/focus/IME ownership",
    )?;
    require(
        state.snapshot.field.text().as_deref() == Some(text)
            && target.text() == text.into()
            && target.selection_start() == text.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "canceled/overlapping drag edited native text/caret",
    )?;
    require(
        keyboard
            .touch_processor()
            .pressed_keys()
            .into_iter()
            .next()
            .is_none(),
        "drag entered typing pressed state",
    )?;
    require(
        state.host.is_floating() == (7..=16).contains(&n),
        "cancellation floating mode mismatch",
    )?;
    if n == 0 {
        let root = Control::from_boxed(&keyboard.content().ok_or("keyboard content missing")?)
            .ok_or("native keyboard root missing")?
            .cast::<Grid>()
            .ok_or("keyboard root Grid missing")?;
        let overlay = root
            .children()
            .get(2)
            .cast::<Grid>()
            .ok_or("resize overlay missing")?;
        require(
            overlay.is_visible() && keyboard.settings().height_scale() == 1.0,
            "cancel resize initial state mismatch",
        )?;
        observe(probe, overlay.upcast());
        let toolbar = state
            .root
            .upgrade()
            .ok_or("fixture root missing")?
            .children()
            .get(0)
            .cast::<StackPanel>()
            .ok_or("fixture toolbar missing")?;
        for (index, label) in [(1, "Steal"), (2, "Drop")] {
            let button = toolbar
                .children()
                .get(index)
                .cast::<ferroboard_controls::primitives::TouchButton>()
                .ok_or("fixture capture button missing")?;
            AutomationProperties::set_name(&button, Some(label));
        }
    }
    if matches!(n, 1 | 8) {
        require(
            held(&probe.latest()?),
            "native control did not own held capture",
        )?;
    }
    if matches!(n, 2 | 3 | 6 | 9 | 10 | 13 | 16 | 17) {
        require(
            probe.all_released(),
            "canceled/released native contact retained capture",
        )?;
    }
    if matches!(n, 4 | 5 | 11 | 12) {
        foreign_held(probe)?;
    }
    let scale = keyboard.settings().height_scale();
    match n {
        1 => {
            require(scale > 1.0, "held resize did not grow")?;
            probe.scale.set(scale);
        }
        2 => require(
            scale == probe.scale.get(),
            "cancel changed committed resize scale",
        )?,
        3 => require(
            scale > probe.scale.get(),
            "fresh resize did not recover after cancel",
        )?,
        4 => {
            require(scale > 1.0, "resize did not grow before capture theft")?;
            probe.scale.set(scale);
        }
        5 => require(
            scale == probe.scale.get(),
            "lost capture still resized keyboard",
        )?,
        6.. => require(
            scale == 1.0,
            "reset height not retained through canceled floating drag",
        )?,
        _ => {}
    }
    if n == 7 {
        let grid = keyboard
            .visual_parent()
            .ok_or("floating parent missing")?
            .cast::<Grid>()
            .ok_or("floating Grid missing")?;
        let frame = grid
            .visual_parent()
            .ok_or("floating frame missing")?
            .cast::<Border>()
            .ok_or("floating Border missing")?;
        probe.y.set(Canvas::get_top(&frame));
        *probe.frame.borrow_mut() = Some(frame.downgrade());
        observe(
            probe,
            grid.children()
                .get(0)
                .cast::<Border>()
                .ok_or("floating drag handle missing")?
                .upcast(),
        );
    }
    match n {
        8 => {
            require(
                probe.y()? < probe.y.get(),
                "held floating drag did not move",
            )?;
            probe.y.set(probe.y()?);
        }
        9 => require(
            probe.y()? == probe.y.get(),
            "cancel changed floating position",
        )?,
        10 => require(
            probe.y()? > probe.y.get(),
            "floating drag did not recover after cancel",
        )?,
        11 => probe.y.set(probe.y()?),
        12 | 13 => require(
            probe.y()? == probe.y.get(),
            "lost capture still moved floating keyboard",
        )?,
        14 => {
            let contacts = probe.contacts.borrow();
            require(contacts.len() >= 2, "missing native overlapping contacts")?;
            let pair = &contacts[contacts.len() - 2..];
            require(
                pair[0].id != pair[1].id && pair.iter().all(held),
                "both native overlapping pointers must retain capture",
            )?;
            require(
                probe.y()? < probe.y.get(),
                "latest native contact did not control drag",
            )?;
            probe.y.set(probe.y()?);
        }
        15 => {
            let contacts = probe.contacts.borrow();
            let pair = &contacts[contacts.len() - 2..];
            require(
                released(&pair[0]) && held(&pair[1]),
                "earlier release disturbed surviving capture",
            )?;
            require(
                probe.y()? == probe.y.get(),
                "source earlier-release drag-stop behavior changed",
            )?;
        }
        16 => require(probe.y()? == probe.y.get(), "cancel moved overlapping drag")?,
        _ => {}
    }
    probe.phase.set(n + 1);
    Ok(label)
}
