use super::cad_view_model::*;
use serde_json::{Value, json};
use std::{cell::RefCell, collections::HashSet, rc::Rc};
struct Hit {
    score: f64,
    trace: Rc<RefCell<Vec<String>>>,
}
impl CadSearchHit for Hit {
    fn score(&self) -> f64 {
        self.score
    }
    fn execute(&self) {
        self.trace.borrow_mut().push("execute:result".into());
    }
}
#[derive(Default)]
struct Session {
    workspace_text: RefCell<Option<String>>,
    trace: Rc<RefCell<Vec<String>>>,
    items: HashSet<String>,
    score: Option<f64>,
    tabs: RefCell<Vec<Value>>,
}

#[test]
fn source_tab_selection_sequences_keep_file_and_layout_independent() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/cad-live.json")).unwrap();
    let mut vm = CadViewModel::new(
        Rc::new(Session::default()),
        Rc::new(CadMemoryLog::default()),
    );
    let mut taps = 0;
    for case in fixture["tabs"].as_array().unwrap() {
        match case["kind"].as_str().unwrap() {
            "created" => {
                vm = CadViewModel::new(
                    Rc::new(Session::default()),
                    Rc::new(CadMemoryLog::default()),
                )
            }
            "tap" => {
                vm.page_action().execute(Some(&CadPageAction::TabSelected(
                    CadDocumentTab::ALL[case["index"].as_u64().unwrap() as usize],
                )));
                taps += 1;
            }
            "replaced-child" => vm
                .page_action()
                .execute(Some(&CadPageAction::TabSelected(CadDocumentTab::FloorPlan))),
            "updated" | "external-toggle" => (),
            _ => panic!("unmapped source tab case {case}"),
        }
        assert_eq!(
            vm.selected_file_tab(),
            CadDocumentTab::ALL[case["state"]["selectedFile"].as_u64().unwrap() as usize]
        );
        assert_eq!(
            vm.selected_layout_tab(),
            CadDocumentTab::ALL[case["state"]["selectedLayout"].as_u64().unwrap() as usize]
        );
    }
    assert_eq!(taps, 48);
}
impl CadRibbonSession for Session {
    fn invoke_id(&self, id: &str) -> bool {
        if self.items.contains(id) {
            self.trace.borrow_mut().push(format!("invoke:{id}"));
            true
        } else {
            false
        }
    }
    fn search(&self, text: &str) -> Option<Box<dyn CadSearchHit>> {
        self.trace.borrow_mut().push(format!("search:{text}"));
        self.score.map(|score| {
            Box::new(Hit {
                score,
                trace: self.trace.clone(),
            }) as Box<dyn CadSearchHit>
        })
    }
    fn apply(&self, effect: CadEffect) {
        match effect {
            CadEffect::TabSelection(choice) => {
                self.trace.borrow_mut().push(format!("tab:{choice:?}"))
            }
            CadEffect::Scene(action) => self.trace.borrow_mut().push(format!("scene:{action:?}")),
            CadEffect::WorkspaceText(text) => {
                self.workspace_text.replace(text);
            }
            CadEffect::SetLightTheme(light) => {
                self.trace.borrow_mut().push(format!("theme:{light}"))
            }
            CadEffect::Contexts(ids) => self
                .trace
                .borrow_mut()
                .push(format!("contexts:{}", ids.join(","))),
            CadEffect::PlaceTextEditor => self.trace.borrow_mut().push("placeText".into()),
            CadEffect::ZoomExtents => self.trace.borrow_mut().push("zoomExtents".into()),
            CadEffect::SelectTab(id) => self.trace.borrow_mut().push(format!("select:{id}")),
            CadEffect::WorkspaceTabs(ids) => {
                for tab in self.tabs.borrow_mut().iter_mut() {
                    if tab["contextual"] == false {
                        tab["visible"] =
                            json!(ids.iter().any(|id| Some(id.as_str()) == tab["id"].as_str()));
                    }
                }
            }
        }
    }
}
#[test]
fn source_application_commands_history_context_workspace_and_keyboard_cases() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/cad-page.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let kind = case["kind"].as_str().unwrap();
        let mut session = Session::default();
        if kind == "input" {
            session.items.extend(["line".into(), "stretch".into()]);
        }
        if kind == "run" {
            session.items.insert("line".into());
            session.score = case["score"].as_f64();
        }
        if kind == "workspace" {
            *session.tabs.borrow_mut() = [
                "home",
                "solid",
                "surface",
                "mesh",
                "visualize",
                "parametric",
                "insert",
                "annotate",
                "view",
                "manage",
                "output",
                "collaborate",
                "context",
            ]
            .into_iter()
            .map(|id| json!({"id":id,"contextual":id=="context","visible":true}))
            .collect();
        }
        let session = Rc::new(session);
        let log = Rc::new(CadMemoryLog::default());
        let vm = CadViewModel::new(session.clone(), log.clone());
        let mut handled = false;
        match kind {
            "input" => {
                vm.set_input(case["text"].as_str().map(str::to_owned));
                handled = vm.handle_key(match case["key"].as_str().unwrap() {
                    "Enter" => CadKey::Enter,
                    "Space" => CadKey::Space,
                    "Escape" => CadKey::Escape,
                    _ => CadKey::Other,
                });
            }
            "run" => vm.run_command(case["text"].as_str().unwrap()),
            "invoked" => vm.on_item_invoked(&CadInvocation {
                item_id: case["id"].as_str().map(str::to_owned),
                command_id: Some("copy".into()),
                choice: case["choice"].as_str().map(str::to_owned),
                split: case["split"].as_bool().unwrap(),
                label: Some("new\nStraße ᾀ".into()),
            }),
            "workspace" => vm.apply_workspace(case["workspace"].as_str().map(str::to_owned)),
            "history" => {
                for i in 0..case["count"].as_u64().unwrap() {
                    vm.echo(
                        &if i == 2 {
                            "Command:".into()
                        } else {
                            format!("line-{i}")
                        },
                        i % 2 == 0,
                    );
                }
            }
            "close" => {
                vm.show_text_editor();
                vm.close_contextual(case["name"].as_str().unwrap());
            }
            other => panic!("Unexpected reference case {other}"),
        }
        let actual = json!({"history":vm.history(),"visible":vm.history_text(),"input":vm.input(),"handled":handled,"workspace":vm.workspace(),"tabs":*session.tabs.borrow(),"textVisible":vm.text_visible(),"trace":*session.trace.borrow(),"log":*log.0.borrow()});
        let mut expected = case["state"].clone();
        if cfg!(windows) {
            expected["visible"] =
                json!(expected["visible"].as_str().unwrap().replace('\n', "\r\n"));
        }
        assert_eq!(actual, expected, "{case}");
    }
}
#[test]
fn reactive_actions_keep_weak_view_model_ownership() {
    let vm = CadViewModel::new(
        Rc::new(Session::default()),
        Rc::new(CadMemoryLog::default()),
    );
    let weak = Rc::downgrade(&vm);
    let command = vm.run_action();
    drop(vm);
    assert!(weak.upgrade().is_none());
    command.execute(Some(&"line".to_owned()));
}
