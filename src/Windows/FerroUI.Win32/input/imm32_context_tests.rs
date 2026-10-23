//! Recorder regressions for native HWND/HIMC ownership. These exercise the
//! production transport on any host; actual Windows IME behavior is a separate
//! platform validation requirement.

use super::tests::{Call, Client, FakeSystem};
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

const FIRST: isize = 0x40;
const SECOND: isize = 0x41;
const JAPANESE: isize = 0x0411;

struct ContextWindow(isize);

impl Imm32Parent for ContextWindow {
    fn hwnd(&self) -> isize {
        self.0
    }
    fn desktop_scaling(&self) -> f64 {
        1.0
    }
    fn set_ignore_wm_char(&self, _value: bool) {}
    fn raw_text_input(&self, _timestamp: u64, _text: &str) -> bool {
        true
    }
    fn raw_key_press(&self, _key: Key, _physical_key: PhysicalKey) {}
}

fn parent(hwnd: isize) -> Rc<dyn Imm32Parent> {
    Rc::new(ContextWindow(hwnd))
}

fn active(system: &Rc<FakeSystem>, parent: &Rc<dyn Imm32Parent>) -> Rc<Imm32InputMethod> {
    let input = Imm32InputMethod::new(system.clone());
    input.set_language_and_window(Rc::downgrade(parent), parent.hwnd(), JAPANESE);
    input.set_client(Some(Client::new()));
    system.run_posted();
    input
}

fn balanced(system: &FakeSystem) {
    assert!(system.borrows.borrow().values().all(|count| *count == 0));
}

#[test]
fn imm32_context_repeated_enable_balances_every_borrow() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    system.take_calls();
    for _ in 0..8 {
        input.enable_imm();
    }
    assert_eq!(
        (0..8)
            .flat_map(|_| [Call::GetContext(FIRST), Call::ReleaseContext(FIRST, 0x1C)])
            .collect::<Vec<_>>(),
        system.take_calls()
    );
    balanced(&system);
}

#[test]
fn imm32_context_creation_failure_creates_no_association_or_caret() {
    let system = FakeSystem::new();
    system.context.set(0);
    system.next_created_context.set(0);
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    assert_eq!(
        vec![Call::GetContext(FIRST), Call::CreateContext],
        system.take_calls()
    );
    assert_eq!(0, input.current_himc.get());
    drop(input);
    assert!(system.take_calls().is_empty());
    balanced(&system);
}

#[test]
fn imm32_context_created_handle_is_reused_then_destroyed_once() {
    let system = FakeSystem::new();
    system.context.set(0);
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    for _ in 0..3 {
        assert!(input.disable_imm());
        assert_eq!(Some(&0), system.associations.borrow().get(&FIRST));
        input.enable_imm();
        assert_eq!(Some(&0x2C), system.associations.borrow().get(&FIRST));
    }
    input.release_window(FIRST, &Rc::downgrade(&owner));
    input.release_window(FIRST, &Rc::downgrade(&owner));
    drop(input);
    let calls = system.take_calls();
    assert_eq!(
        1,
        calls
            .iter()
            .filter(|call| matches!(call, Call::CreateContext))
            .count()
    );
    assert_eq!(
        1,
        calls
            .iter()
            .filter(|call| matches!(call, Call::DestroyContext(0x2C)))
            .count()
    );
    assert_eq!(Some(&0), system.associations.borrow().get(&FIRST));
    assert!(
        calls
            .iter()
            .all(|call| !matches!(call, Call::ReleaseContext(_, 0) | Call::DestroyContext(0)))
    );
    balanced(&system);
}

#[test]
fn imm32_context_disabled_borrowed_association_is_restored_on_release() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    assert!(input.disable_imm());
    assert_eq!(Some(&0), system.associations.borrow().get(&FIRST));
    input.release_window(FIRST, &Rc::downgrade(&owner));
    drop(input);
    assert_eq!(Some(&0x1C), system.associations.borrow().get(&FIRST));
    assert!(
        system
            .take_calls()
            .iter()
            .all(|call| !matches!(call, Call::CreateContext | Call::DestroyContext(_)))
    );
    balanced(&system);
}

#[test]
fn imm32_context_same_language_migration_keeps_physical_ownership_separate() {
    let system = FakeSystem::new();
    system.context.set(0);
    let first = parent(FIRST);
    let second = parent(SECOND);
    let input = active(&system, &first);
    input.set_language_and_window(Rc::downgrade(&second), SECOND, JAPANESE);
    assert_eq!(Some(&0x2C), system.associations.borrow().get(&FIRST));
    assert_eq!(Some(&0x2D), system.associations.borrow().get(&SECOND));
    system.take_calls();

    input.release_window(FIRST, &Rc::downgrade(&first));
    assert_eq!(SECOND, input.hwnd());
    assert!(input.is_active());
    assert_eq!(0x2D, input.current_himc.get());
    assert_eq!(
        vec![
            Call::GetContext(FIRST),
            Call::ReleaseContext(FIRST, 0x2C),
            Call::AssociateContext(FIRST, 0),
            Call::DestroyContext(0x2C)
        ],
        system.take_calls()
    );
    drop(input);
    assert_eq!(Some(&0), system.associations.borrow().get(&SECOND));
    assert_eq!(
        1,
        system
            .take_calls()
            .iter()
            .filter(|call| matches!(call, Call::DestroyContext(0x2D)))
            .count()
    );
    balanced(&system);
}

#[test]
fn imm32_context_shared_default_is_never_owned_by_the_transport() {
    let system = FakeSystem::new();
    let first = parent(FIRST);
    let second = parent(SECOND);
    let input = active(&system, &first);
    input.set_language_and_window(Rc::downgrade(&second), SECOND, JAPANESE);
    input.release_window(FIRST, &Rc::downgrade(&first));
    assert!(input.disable_imm());
    drop(input);
    assert_eq!(Some(&0x1C), system.associations.borrow().get(&SECOND));
    assert!(
        system
            .take_calls()
            .iter()
            .all(|call| !matches!(call, Call::CreateContext | Call::DestroyContext(_)))
    );
    balanced(&system);
}

#[test]
fn imm32_context_stale_parent_cannot_release_a_reused_hwnd() {
    let system = FakeSystem::new();
    system.context.set(0);
    let first = parent(FIRST);
    let replacement = parent(FIRST);
    let input = active(&system, &first);
    input.release_window(FIRST, &Rc::downgrade(&first));
    input.set_language_and_window(Rc::downgrade(&replacement), FIRST, JAPANESE);
    input.set_client(Some(Client::new()));
    system.run_posted();
    system.take_calls();

    input.release_window(FIRST, &Rc::downgrade(&first));
    assert_eq!(FIRST, input.hwnd());
    assert!(input.is_active());
    assert_eq!(Some(&0x2D), system.associations.borrow().get(&FIRST));
    assert!(system.take_calls().is_empty());
    balanced(&system);
}

#[test]
fn imm32_context_reentrant_completion_cannot_disable_the_new_window() {
    let system = FakeSystem::new();
    system.context.set(0);
    let first = parent(FIRST);
    let second = parent(SECOND);
    let input = active(&system, &first);
    let weak_input = Rc::downgrade(&input);
    let second_parent = Rc::downgrade(&second);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::NotifyIme(_, _, _, _)),
        Box::new(move || {
            weak_input
                .upgrade()
                .unwrap()
                .set_language_and_window(second_parent, SECOND, JAPANESE);
        }),
    ));
    assert!(!input.disable_imm());
    assert_eq!(SECOND, input.hwnd());
    assert!(input.is_active());
    assert_eq!(Some(&0x2D), system.associations.borrow().get(&SECOND));
    assert!(
        system
            .take_calls()
            .iter()
            .all(|call| !matches!(call, Call::AssociateContext(SECOND, 0)))
    );
    balanced(&system);
}

#[test]
fn imm32_context_window_release_during_acquire_or_release_keeps_owned_handle_alive() {
    let hooks: [fn(&Call) -> bool; 2] = [
        |call| matches!(call, Call::GetContext(_)),
        |call| matches!(call, Call::ReleaseContext(_, _)),
    ];
    for hook in hooks {
        let system = FakeSystem::new();
        system.context.set(0);
        let owner = parent(FIRST);
        let input = active(&system, &owner);
        input.handle_composition_start();
        system.take_calls();
        let weak_input = Rc::downgrade(&input);
        let weak_owner = Rc::downgrade(&owner);
        *system.next_hook.borrow_mut() = Some((
            hook,
            Box::new(move || {
                weak_input
                    .upgrade()
                    .unwrap()
                    .release_window(FIRST, &weak_owner);
            }),
        ));
        assert_eq!(None, input.get_composition_string(GCS::GCS_COMPSTR));
        assert_eq!(0, input.hwnd());
        let calls = system.take_calls();
        let destroy = calls
            .iter()
            .position(|call| matches!(call, Call::DestroyContext(0x2C)))
            .unwrap();
        let release = calls
            .iter()
            .rposition(|call| matches!(call, Call::ReleaseContext(FIRST, 0x2C)))
            .unwrap();
        assert!(release < destroy);
        assert_eq!(Some(&0), system.associations.borrow().get(&FIRST));
        balanced(&system);
    }
}

#[test]
fn imm32_context_window_release_during_creation_or_association_cleans_owned_handle() {
    let hooks: [fn(&Call) -> bool; 2] = [
        |call| matches!(call, Call::CreateContext),
        |call| matches!(call, Call::AssociateContext(_, 0x2C)),
    ];
    for hook in hooks {
        let system = FakeSystem::new();
        system.context.set(0);
        let owner = parent(FIRST);
        let input = Imm32InputMethod::new(system.clone());
        input.set_language_and_window(Rc::downgrade(&owner), FIRST, JAPANESE);
        let weak_input = Rc::downgrade(&input);
        let weak_owner = Rc::downgrade(&owner);
        *system.next_hook.borrow_mut() = Some((
            hook,
            Box::new(move || {
                weak_input
                    .upgrade()
                    .unwrap()
                    .release_window(FIRST, &weak_owner);
            }),
        ));
        input.enable_imm();
        assert_eq!(0, input.hwnd());
        let calls = system.take_calls();
        assert_eq!(
            1,
            calls
                .iter()
                .filter(|call| matches!(call, Call::DestroyContext(0x2C)))
                .count()
        );
        assert!(
            calls
                .iter()
                .all(|call| !matches!(call, Call::CreateCaret(_, _, _)))
        );
        balanced(&system);
    }
}

#[test]
fn imm32_context_unwinding_native_notification_releases_the_original_pair() {
    let system = FakeSystem::new();
    system.context.set(0);
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::NotifyIme(_, _, _, _)),
        Box::new(|| panic!("simulated native callback unwind")),
    ));
    assert!(catch_unwind(AssertUnwindSafe(|| input.disable_imm())).is_err());
    balanced(&system);
    system.take_calls();
    drop(input);
    assert_eq!(
        1,
        system
            .take_calls()
            .iter()
            .filter(|call| matches!(call, Call::DestroyContext(0x2C)))
            .count()
    );
    balanced(&system);
}

#[test]
fn imm32_context_cleanup_preserves_an_external_replacement_association() {
    let system = FakeSystem::new();
    system.context.set(0);
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    system.associations.borrow_mut().insert(FIRST, 0x7C);
    input.release_window(FIRST, &Rc::downgrade(&owner));
    assert_eq!(Some(&0x7C), system.associations.borrow().get(&FIRST));
    let calls = system.take_calls();
    assert_eq!(
        1,
        calls
            .iter()
            .filter(|call| matches!(call, Call::DestroyContext(0x2C)))
            .count()
    );
    assert!(
        calls
            .iter()
            .all(|call| !matches!(call, Call::DestroyContext(0x7C)))
    );
    balanced(&system);
}

#[test]
fn imm32_context_drop_restores_suspended_borrowed_context_even_with_owned_history() {
    let system = FakeSystem::new();
    system.context.set(0);
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    system.associations.borrow_mut().insert(FIRST, 0x7C);
    assert!(input.disable_imm());
    assert_eq!(Some(&0), system.associations.borrow().get(&FIRST));
    drop(input);
    assert_eq!(Some(&0x7C), system.associations.borrow().get(&FIRST));
    let calls = system.take_calls();
    assert_eq!(
        1,
        calls
            .iter()
            .filter(|call| matches!(call, Call::DestroyContext(0x2C)))
            .count()
    );
    assert!(
        calls
            .iter()
            .all(|call| !matches!(call, Call::DestroyContext(0x7C)))
    );
    balanced(&system);
}

#[test]
fn imm32_context_reentrant_caret_creation_does_not_resurrect_destroyed_caret() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = Imm32InputMethod::new(system.clone());
    input.set_language_and_window(Rc::downgrade(&owner), FIRST, JAPANESE);
    let weak_input = Rc::downgrade(&input);
    let weak_owner = Rc::downgrade(&owner);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::CreateCaret(_, _, _)),
        Box::new(move || {
            weak_input
                .upgrade()
                .unwrap()
                .release_window(FIRST, &weak_owner)
        }),
    ));
    input.enable_imm();
    input.caret_manager.try_move(&*system, 1, 2);
    drop(input);
    let calls = system.take_calls();
    assert_eq!(
        1,
        calls
            .iter()
            .filter(|call| matches!(call, Call::DestroyCaret))
            .count()
    );
    assert!(
        calls
            .iter()
            .all(|call| !matches!(call, Call::SetCaretPos(_, _)))
    );
    balanced(&system);
}

#[test]
fn imm32_context_reentrant_caret_move_drops_stale_candidate_geometry() {
    let system = FakeSystem::new();
    system.context.set(0);
    let first = parent(FIRST);
    let second = parent(SECOND);
    let input = active(&system, &first);
    system.active_window.set(FIRST);
    let weak_input = Rc::downgrade(&input);
    let second_parent = Rc::downgrade(&second);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::SetCaretPos(_, _)),
        Box::new(move || {
            weak_input
                .upgrade()
                .unwrap()
                .set_language_and_window(second_parent, SECOND, JAPANESE)
        }),
    ));
    input.set_cursor_rect(Rect::new(10.0, 20.0, 1.0, 16.0));
    system.run_posted();
    assert_eq!(SECOND, input.hwnd());
    assert!(
        system
            .take_calls()
            .iter()
            .all(|call| !matches!(call, Call::SetCandidateWindow(_, _)))
    );
    balanced(&system);
}

#[test]
fn imm32_context_native_read_release_cannot_overwrite_a_new_composition() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    input.handle_composition_start();
    *system.composition_string.borrow_mut() = Some("old native preedit".to_owned());
    let weak_input = Rc::downgrade(&input);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::ReleaseContext(_, _)),
        Box::new(move || weak_input.upgrade().unwrap().handle_composition_start()),
    ));
    input.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);
    assert!(input.is_composing());
    assert_eq!(None, input.composition());
    balanced(&system);
}
