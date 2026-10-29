//! Private stdin protocol for the development supervisor. No socket or external
//! watcher loads code. The parent owns the isolated build, full input collection,
//! compiler pin, artifact provenance and exclusive access to this pipe.
use ferroui_hot_patch::{AdmittedPatch, FunctionRegistration, PatchSupervisor, SourceSnapshot};
use ferroui_markup_xaml::FromXamlObject;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{BufRead, Write},
    sync::mpsc::{self, Receiver},
};

pub struct Development {
    commands: Receiver<Value>,
    supervisor: PatchSupervisor,
    pending: Option<(u64, AdmittedPatch, BTreeMap<String, String>)>,
    request: u64,
    restart_pause: Option<ferroui_hot_reload::RestartHandoff>,
}
impl Development {
    /// The parent must record the exact inputs of the running executable and
    /// supply only artifacts built from an admitted snapshot with the pinned tools.
    /// Patchable code in this sample runs exclusively on the UI thread.
    ///
    /// # Safety
    /// Only a trusted supervisor satisfying PatchSupervisor::apply's compiler and
    /// artifact guarantees may own stdin. Source admission is not ABI validation.
    pub unsafe fn connect() -> Option<Self> {
        let path = std::env::var_os("FERROUI_PATCH_BASELINE")?;
        let sources: SourceSnapshot =
            serde_json::from_slice(&std::fs::read(path).expect("patch baseline"))
                .expect("serialized build inputs");
        assert_eq!(
            sources.get("program.rs").map(String::as_str),
            Some(include_str!("program.rs")),
            "supervisor baseline does not match the compiled executable source"
        );
        let supervisor = PatchSupervisor::with_registrations(
            sources,
            [
                FunctionRegistration::callback("program.rs", "counter_click"),
                FunctionRegistration::view_builder("program.rs", "build_view", ["main".into()]),
            ],
        )
        .expect("valid executable function registrations");
        let (send, commands) = mpsc::sync_channel(8);
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines() {
                let Ok(line) = line else {
                    break;
                };
                let message = serde_json::from_str(&line)
                    .unwrap_or_else(|error| json!({"op": "invalid", "error": error.to_string()}));
                if send.send(message).is_err() {
                    break;
                }
            }
            // A disconnected compiler must not leave markup indefinitely frozen.
            let _ = send.send(json!({"op": "cancel"}));
        });
        reply(json!({"status": "ready", "aslr": ferroui_hot_patch::aslr_reference()}));
        Some(Self {
            commands,
            supervisor,
            pending: None,
            request: 0,
            restart_pause: None,
        })
    }
    pub fn building(&self) -> bool {
        self.pending.is_some() || self.restart_pause.is_some()
    }
    /// Called from the same dispatcher turn that pumps markup, between UI events.
    pub fn pump(
        &mut self,
        host: &ferroui_controls::ContentControl,
        window: &ferroui_controls::Window,
        session: &mut ferroui_hot_reload::HotReloadSession,
    ) {
        // Bound per-turn work so a busy supervisor cannot starve input/layout.
        for _ in 0..8 {
            let Ok(message) = self.commands.try_recv() else {
                break;
            };
            let result = self.handle(message, host, window, session);
            reply(result.unwrap_or_else(|error| json!({"status": "rejected", "error": error})));
        }
    }
    fn handle(
        &mut self,
        message: Value,
        host: &ferroui_controls::ContentControl,
        window: &ferroui_controls::Window,
        session: &mut ferroui_hot_reload::HotReloadSession,
    ) -> Result<Value, String> {
        let operation = message["op"].as_str();
        if self.restart_pause.is_some() && !matches!(operation, Some("resume" | "cancel" | "state"))
        {
            return Err("Restart handoff has frozen application interaction".into());
        }
        match operation {
            Some("admit") => {
                self.pending = None;
                self.request += 1;
                let sources = serde_json::from_value(message["sources"].clone())
                    .map_err(|e| e.to_string())?;
                let markup = serde_json::from_value(
                    message.get("markup").cloned().unwrap_or_else(|| json!({})),
                )
                .map_err(|error| error.to_string())?;
                let patch = self.supervisor.admit(sources)?;
                let response = json!({"status": "admitted", "request": self.request, "functions": patch.functions,
                    "views": patch.views_to_reapply()});
                self.pending = Some((self.request, patch, markup));
                Ok(response)
            }
            Some("cancel") => {
                self.pending = None;
                self.restart_pause.take();
                Ok(json!({"status": "cancelled"}))
            }
            Some("apply") => {
                let request = message["request"]
                    .as_u64()
                    .ok_or("Missing request identity")?;
                if self.pending.as_ref().map(|(id, _, _)| *id) != Some(request) {
                    return Err("No matching admitted build".into());
                }
                let table =
                    serde_json::from_value(message["table"].clone()).map_err(|e| e.to_string())?;
                let current = serde_json::from_value(message["sources"].clone())
                    .map_err(|e| e.to_string())?;
                let (_, patch, markup) = self.pending.take().unwrap();
                let rebuild = !patch.views_to_reapply().is_empty();
                if patch.views_to_reapply().iter().any(|view| view != "main") {
                    return Err("No adapter for the requested view".into());
                }
                session.submit_texts(markup);
                if rebuild {
                    let sources = session.sources();
                    let uri = sources
                        .keys()
                        .find(|uri| uri.ends_with("/View.xaml"))
                        .ok_or("Missing main view")?
                        .clone();
                    // Preserve any submitted draft while requesting an explicit
                    // builder reapplication even if its authored text is unchanged.
                    session
                        .reapply_document(&uri)
                        .map_err(|error| format!("{error:?}"))?;
                    let mut outcome = None;
                    // SAFETY: the same exclusive compiler/artifact/UI-thread
                    // contract as below. The builder is pure; session rollback
                    // restores the graph before apply_with restores old code.
                    let result = unsafe {
                        self.supervisor.apply_with(patch, &current, table, || {
                            let applied = session
                                .apply_pending()
                                .ok_or("No builder revision to apply")?;
                            let accepted =
                                applied.status == ferroui_hot_reload::ReloadStatus::Applied;
                            let error = applied.diagnostic.clone();
                            outcome = Some(applied);
                            if accepted {
                                Ok(())
                            } else {
                                Err(error
                                    .unwrap_or_else(|| "View builder transaction failed".into()))
                            }
                        })
                    };
                    if outcome.as_ref().is_some_and(|outcome| {
                        outcome.status == ferroui_hot_reload::ReloadStatus::RestartRequired
                    }) {
                        return Ok(json!({"status": "restart_required",
                            "error": result.err()}));
                    }
                    let revision = result?;
                    if let Some(outcome) = &outcome {
                        ferroui_hot_reload::record_frame_timing(window, outcome);
                    }
                    return Ok(json!({"status": "applied", "revision": revision,
                        "markup_revision": session.applied_revision()}));
                }
                // SAFETY: connect requires a trusted owning supervisor which builds only
                // admitted inputs. This pump runs between UI callbacks; the stdin worker
                // never invokes patchable code. Unmatched/superseded inputs fail closed.
                let mut revision = None;
                let install = || {
                    revision = Some(
                        unsafe { self.supervisor.apply(patch, &current, table) }
                            .map_err(ferroui_hot_reload::ReloadError::Failed)?,
                    );
                    Ok(())
                };
                if session.has_pending_changes() {
                    let outcome = session
                        .apply_pending_with(install)
                        .ok_or("Markup draft requires a new revision before patch application")?;
                    if outcome.status == ferroui_hot_reload::ReloadStatus::RestartRequired {
                        return Ok(
                            json!({"status": "restart_required", "error": outcome.diagnostic}),
                        );
                    }
                    if outcome.status != ferroui_hot_reload::ReloadStatus::Applied {
                        return Err(outcome
                            .diagnostic
                            .unwrap_or_else(|| "Markup transaction failed".into()));
                    }
                    ferroui_hot_reload::record_frame_timing(window, &outcome);
                } else {
                    install().map_err(|error| format!("{error:?}"))?;
                }
                Ok(json!({"status": "applied", "revision": revision,
                    "markup_revision": session.applied_revision()}))
            }
            Some("markup") => {
                if self.pending.is_some() {
                    return Err("A patch build already owns the development revision".into());
                }
                let markup: BTreeMap<String, String> =
                    serde_json::from_value(message["markup"].clone())
                        .map_err(|error| error.to_string())?;
                session.submit_texts(markup);
                if !session.has_pending_changes() {
                    return Ok(json!({"status": "applied",
                        "markup_revision": session.applied_revision(),
                        "preparation_us": 0, "application_us": 0}));
                }
                let outcome = session.apply_pending().ok_or("No new markup revision")?;
                ferroui_hot_reload::record_frame_timing(window, &outcome);
                Ok(json!({"status": match outcome.status {
                    ferroui_hot_reload::ReloadStatus::Applied => "applied",
                    ferroui_hot_reload::ReloadStatus::RestartRequired => "restart_required",
                    _ => "rejected",
                }, "markup_revision": outcome.revision, "error": outcome.diagnostic,
                    "preparation_us": outcome.preparation.as_micros(),
                    "application_us": outcome.application.as_micros()}))
            }
            Some("click") => {
                host.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
                    ferroui_controls::Button::click_event(),
                ));
                Ok(
                    json!({"status": "clicked", "counter": crate::COUNT.with(|count| count.get()),
                    "host": host as *const _ as usize,
                    "content": host.content().and_then(|content|
                        ferroui_base::Ref::<ferroui_controls::StackPanel>::from_xaml_object(&content))
                        .map(|panel| &*panel as *const ferroui_controls::StackPanel as usize),
                    "spacing": host.content().and_then(|content|
                        ferroui_base::Ref::<ferroui_controls::StackPanel>::from_xaml_object(&content)).map(|panel| panel.spacing()),
                    "markup_revision": session.applied_revision(),
                    "type": ferroui_controls::ContentControl::TYPE as *const _ as usize}),
                )
            }
            Some("prepare_restart") => {
                if self.pending.is_some() {
                    return Err("A patch build owns the current revision".into());
                }
                self.restart_pause = Some(
                    crate::begin_restart_handoff(window).map_err(|error| format!("{error:?}"))?,
                );
                Ok(
                    json!({"status": "state", "state": crate::capture_restart_state(window), "interaction_enabled": window.is_enabled()}),
                )
            }
            Some("place_window") => {
                let placement: ferroui_hot_patch::restart::WindowPlacement =
                    serde_json::from_value(message["placement"].clone())
                        .map_err(|error| error.to_string())?;
                if placement.key != "main"
                    || placement
                        .size
                        .iter()
                        .any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return Err("Invalid main-window placement".into());
                }
                window.set_window_state(ferroui_controls::WindowState::Normal);
                window.set_width(placement.size[0]);
                window.set_height(placement.size[1]);
                window.set_position(ferroui_base::PixelPoint::new(
                    placement.position[0],
                    placement.position[1],
                ));
                if placement.maximized {
                    window.set_window_state(ferroui_controls::WindowState::Maximized);
                }
                Ok(json!({"status": "placed"}))
            }
            Some("resume") => {
                self.restart_pause.take();
                Ok(json!({"status": "resumed"}))
            }
            Some("state") => Ok(
                json!({"status": "state", "state": crate::capture_restart_state(window), "interaction_enabled": window.is_enabled()}),
            ),
            _ => Err("Unknown development command".into()),
        }
    }
}
fn reply(value: Value) {
    println!("FERROUI_DEV={value}");
    let _ = std::io::stdout().flush();
}
