//! Stateful downstream composers use public traits and native ownership on the engine thread.
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{KeyboardEngine, LanguageModels},
    layouts::{ComponentName, KeyboardResources, Subtype, SubtypeValues},
    settings::KeyboardSettings,
    text::{IComposer, Utf16Text},
};
use std::{
    cell::{Cell, RefCell},
    error::Error,
    rc::Rc,
    sync::Arc,
};

type Calls = Rc<RefCell<Vec<(Utf16Text, Utf16Text)>>>;

struct LocalComposer {
    replacement: Rc<Cell<u16>>,
    calls: Calls,
}
impl IComposer for LocalComposer {
    fn id(&self) -> Utf16Text {
        "local".into()
    }
    fn label(&self) -> Utf16Text {
        "Local composer".into()
    }
    fn to_read(&self) -> i32 {
        2
    }
    fn get_actions(&self, before: &Utf16Text, insert: &Utf16Text) -> (i32, Utf16Text) {
        self.calls
            .borrow_mut()
            .push((before.clone(), insert.clone()));
        if before.units() == [97, 98] && insert.units() == [33] {
            (2, Utf16Text::from_units(vec![self.replacement.get()]))
        } else {
            (0, insert.clone())
        }
    }
}

// The public registry deliberately accepts Arc<dyn IComposer> without Send/Sync:
// a custom composer may use thread-local state even though built-ins are shareable.
#[allow(clippy::arc_with_non_send_sync)]
fn local(replacement: Rc<Cell<u16>>, calls: Calls) -> Arc<dyn IComposer> {
    Arc::new(LocalComposer { replacement, calls })
}

#[test]
fn independent_stateful_composer_keeps_utf16_context_live_state_and_registry_identity()
-> Result<(), Box<dyn Error>> {
    let calls = Calls::default();
    let replacement = Rc::new(Cell::new(0x2713));
    let composer = local(replacement.clone(), calls.clone());
    let retired = Arc::downgrade(&composer);
    let resources = Rc::new(KeyboardResources::create_default()?);
    resources.add_composer(composer.clone(), Some("consumer".into()));
    let name = ComponentName::new("consumer", "local");
    assert!(Arc::ptr_eq(&composer, &resources.get_composer(&name)));

    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_auto_space_punctuation(false);
    settings.set_show_suggestions(false);
    settings.set_emoji_suggestions(false);
    settings.set_glide_typing(false);
    settings.set_learn_words(false);
    let engine = KeyboardEngine::new_explicit(settings, resources.clone(), LanguageModels::new())?;
    let target = Rc::new(TextBufferTarget::new("😀".into(), None));
    engine.attach(target.clone())?;
    engine.set_subtypes([Arc::new(Subtype::from_values(SubtypeValues {
        composer: name,
        ..(*Subtype::shared_default()).values()
    }))])?;
    assert!(Arc::ptr_eq(&composer, &engine.composer()));
    engine.type_text(&"ab!".into())?;
    assert_eq!(target.text(), Utf16Text::from("😀✓"));
    assert_eq!(
        (target.selection_start(), target.selection_length()),
        (3, 0)
    );
    assert_eq!(
        *calls.borrow(),
        [
            ("😀".into(), "a".into()),
            (Utf16Text::from_units(vec![0xde00, 97]), "b".into()),
            ("ab".into(), "!".into()),
        ]
    );

    replacement.set(0x2605);
    target.set_text(Some("😀ab".into()));
    target.move_caret_externally(4);
    engine.type_text(&"!".into())?;
    assert_eq!(target.text(), Utf16Text::from("😀★"));

    let updated = local(Rc::new(Cell::new(0x2606)), calls.clone());
    resources.add_composer(updated.clone(), Some("consumer".into()));
    assert!(Arc::ptr_eq(&updated, &engine.composer()));
    drop(composer);
    assert!(retired.upgrade().is_none());
    target.set_text(Some("😀ab".into()));
    target.move_caret_externally(4);
    engine.type_text(&"!".into())?;
    assert_eq!(target.text(), Utf16Text::from("😀☆"));
    target.move_caret_externally(0);
    engine.type_text(&"!".into())?;
    assert_eq!(target.text(), Utf16Text::from("!😀☆"));
    assert_eq!(
        calls.borrow().last(),
        Some(&(Utf16Text::default(), "!".into()))
    );

    let released = Arc::downgrade(&updated);
    drop(updated);
    engine.dispose()?;
    drop(engine);
    drop(resources);
    assert!(released.upgrade().is_none());
    Ok(())
}

struct DropComposer {
    id: Utf16Text,
    on_drop: Option<Box<dyn FnOnce()>>,
}
impl IComposer for DropComposer {
    fn id(&self) -> Utf16Text {
        self.id.clone()
    }
    fn label(&self) -> Utf16Text {
        self.id.clone()
    }
    fn to_read(&self) -> i32 {
        0
    }
    fn get_actions(&self, _: &Utf16Text, insert: &Utf16Text) -> (i32, Utf16Text) {
        (0, insert.clone())
    }
}
impl Drop for DropComposer {
    fn drop(&mut self) {
        if let Some(on_drop) = self.on_drop.take() {
            on_drop();
        }
    }
}
// The supported public composer contract includes owner-thread state.
#[allow(clippy::arc_with_non_send_sync)]
fn drop_composer(id: &str, on_drop: Option<Box<dyn FnOnce()>>) -> Arc<dyn IComposer> {
    Arc::new(DropComposer {
        id: id.into(),
        on_drop,
    })
}

#[test]
fn retired_composer_can_read_and_mutate_resources_before_change_notifications()
-> Result<(), Box<dyn Error>> {
    let resources = Rc::new(KeyboardResources::create_default()?);
    let events = Rc::new(RefCell::new(Vec::new()));
    let replacement = drop_composer("retiring", None);
    let expected = replacement.clone();
    let owner = Rc::downgrade(&resources);
    let log = events.clone();
    let retired = drop_composer(
        "retiring",
        Some(Box::new(move || {
            log.borrow_mut().push("drop entered");
            let resources = owner.upgrade().expect("live resource owner");
            let name = ComponentName::new("consumer", "retiring");
            assert!(Arc::ptr_eq(&resources.get_composer(&name), &expected));
            resources.add_composer(drop_composer("nested", None), Some("consumer".into()));
            log.borrow_mut().push("drop finished");
        })),
    );
    let weak = Arc::downgrade(&retired);
    resources.add_composer(retired, Some("consumer".into()));
    let log = events.clone();
    resources.changed().add(Rc::new(move |owner| {
        assert_eq!(
            owner
                .get_composer(&ComponentName::new("consumer", "nested"))
                .id(),
            Utf16Text::from("nested")
        );
        log.borrow_mut().push("changed");
    }));

    resources.add_composer(replacement.clone(), Some("consumer".into()));
    assert!(weak.upgrade().is_none());
    assert!(Arc::ptr_eq(
        &resources.get_composer(&ComponentName::new("consumer", "retiring")),
        &replacement
    ));
    assert_eq!(
        *events.borrow(),
        ["drop entered", "changed", "drop finished", "changed"]
    );
    Ok(())
}

#[test]
fn extension_replacement_releases_custom_composer_after_publishing_the_definition()
-> Result<(), Box<dyn Error>> {
    let resources = Rc::new(KeyboardResources::create_empty());
    let owner = Rc::downgrade(&resources);
    let drops = Rc::new(Cell::new(0));
    let count = drops.clone();
    resources.add_composer(
        drop_composer(
            "retiring",
            Some(Box::new(move || {
                let owner = owner.upgrade().unwrap();
                let composer = owner.get_composer(&ComponentName::new("consumer", "retiring"));
                assert_eq!(composer.label(), Utf16Text::from("Replacement"));
                count.set(count.get() + 1);
            })),
        ),
        Some("consumer".into()),
    );
    let manifest = ferroboard_core::layouts::LayoutJson::read_extension_manifest(
        r#"{"meta":{"id":"consumer"},"composers":[{"$":"with-rules","id":"retiring","label":"Replacement"}]}"#,
    )?;
    resources.add_extension(Rc::new(manifest), Rc::new(|_| Ok(None)))?;
    assert_eq!(drops.get(), 1);
    Ok(())
}
