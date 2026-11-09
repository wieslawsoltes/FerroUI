//! Opt-in native selection checks in the compiled sample, with volatile private state.
use ferroboard_controls::{SmartbarAction, input::TextBoxInputTarget};
use ferroboard_core::keys::KeyIcon;
use ferroboard_sample::{
    App,
    composition::SampleSession,
    sample_options::SampleOptions,
    sample_storage::{SETTINGS_KEY, SampleStorage, SampleStoragePlatform, StorageResult},
};
use ferroui_base::{
    FerroObjectImpl, Ref,
    controls::NameScope,
    input::{InputElement, text_input::TextInputMethodClientRequestedEventArgs},
};
use ferroui_controls::{
    AppBuilder, ApplicationImpl, ApplicationImplExt, NewApplication, TextBox,
    application_lifetimes::ClassicDesktopStyleApplicationLifetime,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

struct VolatileStorage {
    settings: RefCell<HashMap<String, String>>,
    documents: RefCell<HashMap<PathBuf, String>>,
}
impl VolatileStorage {
    fn new() -> Self {
        Self {
            settings: RefCell::new(HashMap::from([(
                SETTINGS_KEY.to_owned(),
                r#"{"learnWords":false,"incognitoMode":true,"showSuggestions":false,"emojiSuggestions":false,"glideTyping":false,"emojiHistoryEnabled":false,"clipboardHistoryEnabled":false,"clipboardSuggestion":false}"#.to_owned(),
            )])),
            documents: RefCell::new(HashMap::new()),
        }
    }
}
impl SampleStoragePlatform for VolatileStorage {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        Ok(self.settings.borrow().get(key).cloned())
    }
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        self.settings
            .borrow_mut()
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        Ok(None)
    }
    fn read_document(&self, name: &Path) -> StorageResult<Option<String>> {
        Ok(self.documents.borrow().get(name).cloned())
    }
    fn write_document(&self, name: &Path, content: &str) -> StorageResult<()> {
        self.documents
            .borrow_mut()
            .insert(name.to_owned(), content.to_owned());
        Ok(())
    }
    fn log(&self, message: &str) {
        eprintln!("selection probe: {message}");
    }
}

#[repr(C)]
struct ProbeApp {
    base: App,
}
ferroui_base::ferro_class!(ProbeApp: App);
ferroui_base::ferro_impl_classes!(ProbeApp: FerroObjectImpl);
impl NewApplication for ProbeApp {
    fn new_application() -> Ref<Self> {
        let app = ferroui_base::instantiate(Self {
            base: App::construct(),
        });
        app.set_name(Some("FerroBoard Selection Check".into()));
        app
    }
}
impl ApplicationImpl for ProbeApp {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
    }
    fn on_framework_initialization_completed(_this: &Self) {}
}

#[derive(Default)]
struct Observation {
    armed: Cell<bool>,
    replace: Cell<bool>,
    first: Cell<Option<(i32, i32)>>,
    completed: Cell<u8>,
    failed: Cell<bool>,
}
fn prepare(session: &SampleSession, field: &Ref<TextBox>, observed: &Observation) -> bool {
    observed.armed.set(false);
    observed.first.set(None);
    field.set_current_value(TextBox::text_property(), Some("a😀bcdef".to_owned()));
    field.set_caret_index(3);
    field.focus();
    session
        .keyboard
        .attach_to(Some(field.clone().upcast()))
        .expect("native probe target");
    let request = TextInputMethodClientRequestedEventArgs::new();
    request.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    field.raise_event(&request);
    request
        .client()
        .is_some_and(|client| client.text_view_visual().is_some())
}
fn report(
    session: &SampleSession,
    field: &TextBox,
    check: &str,
    passed: bool,
    first: Option<(i32, i32)>,
) -> bool {
    let target = session.keyboard.target_element() == Some(field.to_ref().upcast());
    let focus = field.is_focused();
    let status = if passed && target && focus {
        "PASS"
    } else {
        "FAILED"
    };
    session.window.set_title(Some(format!("FerroBoard Selection Check | {status} {check} first={first:?} focus={focus} target={target}")));
    passed && target && focus
}

fn main() -> Result<(), Box<dyn Error>> {
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    AppBuilder::configure::<ProbeApp>()
        .use_platform_detect()
        .setup_with_lifetime(lifetime.clone());
    let options = Arc::new(SampleOptions::default());
    options.parse(Some(&"--settings=false".into()));
    let session =
        SampleSession::with_options(SampleStorage::new(Rc::new(VolatileStorage::new())), options)?;
    session.window.set_title(Some(
        "FerroBoard Selection Check | Choose a check in Quick actions".into(),
    ));
    let field = NameScope::get_name_scope(&session.window)
        .and_then(|scope| scope.find("MessageBox"))
        .and_then(|value| value.cast::<TextBox>())
        .ok_or("compiled MessageBox missing")?;
    let other = NameScope::get_name_scope(&session.window)
        .and_then(|scope| scope.find("NameBox"))
        .and_then(|value| value.cast::<TextBox>())
        .ok_or("compiled NameBox missing")?;
    let observer = TextBoxInputTarget::new(field.clone(), None)?;
    let observed = Rc::new(Observation::default());
    let state = observed.clone();
    let weak = field.downgrade();
    let token = observer.changed().add(Rc::new(move |target| {
        if !state.armed.replace(false) {
            return;
        }
        state
            .first
            .set(Some((target.selection_start(), target.selection_length())));
        if state.replace.get() {
            weak.upgrade()
                .expect("live probe field")
                .set_selected_text(Some("!"));
        }
    }));
    let actions = session
        .keyboard
        .smartbar_actions()
        .ok_or("native actions unavailable")?;
    for (mask, id, name, replace, collapse) in [
        (1, "range", "Check complete range", false, false),
        (2, "replace", "Check reentrant replacement", true, false),
        (4, "collapse", "Check unchanged caret collapse", false, true),
    ] {
        let weak = Rc::downgrade(&session);
        let field = field.clone();
        let observed = observed.clone();
        actions
            .lock()
            .map_err(|_| "native actions lock poisoned")?
            .push(Some(SmartbarAction::new(
                Some(id.into()),
                KeyIcon::None,
                Some(name.into()),
                Some(Rc::new(move |engine| {
                    let session = weak.upgrade().expect("live probe session");
                    if !prepare(&session, &field, &observed) {
                        report(&session, &field, name, false, None);
                        observed.failed.set(true);
                        return Ok(());
                    }
                    observed.replace.set(replace);
                    observed.armed.set(!collapse);
                    let primary = engine.target().expect("native probe primary target");
                    primary.select(1, 5);
                    let passed = if collapse {
                        primary.select(6, 0);
                        primary.selection_start() == 6
                            && primary.selection_length() == 0
                            && field.caret_index() == 6
                    } else if replace {
                        observed.first.get() == Some((1, 5))
                            && field.text().as_deref() == Some("a!ef")
                    } else {
                        observed.first.get() == Some((1, 5))
                            && primary.selection_start() == 1
                            && primary.selection_length() == 5
                            && field.caret_index() == 6
                    };
                    if report(&session, &field, name, passed, observed.first.get()) {
                        observed.completed.set(observed.completed.get() | mask);
                    } else {
                        observed.failed.set(true);
                    }
                    Ok(())
                })),
                None,
            )));
    }
    let weak = Rc::downgrade(&session);
    let inactive_field = field.clone();
    let inactive_observed = observed.clone();
    actions
        .lock()
        .map_err(|_| "native actions lock poisoned")?
        .push(Some(SmartbarAction::new(
            Some("inactive".into()),
            KeyIcon::None,
            Some("Check inactive replacement".into()),
            Some(Rc::new(move |_| {
                let session = weak.upgrade().expect("live probe session");
                let prepared = prepare(&session, &inactive_field, &inactive_observed);
                let writer = TextBoxInputTarget::new(inactive_field.clone(), None)
                    .expect("inactive native probe target");
                let switched = other.focus();
                let request = TextInputMethodClientRequestedEventArgs::new();
                request.set_routed_event(Some(
                    InputElement::text_input_method_client_requested_event(),
                ));
                inactive_field.raise_event(&request);
                let inactive = request
                    .client()
                    .is_some_and(|client| client.text_view_visual().is_none());
                inactive_observed.first.set(None);
                inactive_observed.replace.set(true);
                inactive_observed.armed.set(true);
                writer.select(1, 5);
                writer.detach();
                let passed = prepared
                    && switched
                    && inactive
                    && !inactive_field.is_focused()
                    && inactive_observed.first.get() == Some((1, 5))
                    && inactive_field.text().as_deref() == Some("a!ef");
                if report(
                    &session,
                    &other,
                    "inactive replacement",
                    passed,
                    inactive_observed.first.get(),
                ) {
                    inactive_observed
                        .completed
                        .set(inactive_observed.completed.get() | 8);
                } else {
                    inactive_observed.failed.set(true);
                }
                Ok(())
            })),
            None,
        )));
    session.window.show();
    let exit = lifetime.start(&[]);
    observer.changed().remove(token);
    observer.detach();
    session.release();
    lifetime.dispose();
    if exit != 0 {
        return Err(format!("native selection probe exited with {exit}").into());
    }
    if observed.failed.get() || observed.completed.get() != 15 {
        return Err(format!(
            "native selection checks failed or incomplete: completed={}, failed={}",
            observed.completed.get(),
            observed.failed.get()
        )
        .into());
    }
    Ok(())
}
