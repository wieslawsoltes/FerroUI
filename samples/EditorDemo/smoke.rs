//! Optional application-lifetime checks that allow overlays to render between
//! actions. The regular demo does not run or schedule these checks.

use crate::{EditorDemo, LanguageMode};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::Application;
use ferroui_edit::CompletionWindow;
use std::rc::Rc;
use std::time::Duration;

pub(crate) fn start(demo: Rc<EditorDemo>) {
    schedule(demo, 0);
}

fn schedule(demo: Rc<EditorDemo>, step: usize) {
    let _timer = DispatcherTimer::run_once(
        move || match run_step(&demo, step) {
            Ok(true) => schedule(demo.clone(), step + 1),
            Ok(false) => {
                println!("Editor smoke actions completed successfully");
                shutdown(0);
            }
            Err(error) => {
                eprintln!("Editor smoke action {step} failed: {error}");
                shutdown(1);
            }
        },
        Duration::from_millis(200),
        DispatcherPriority::NORMAL,
    );
}

fn verify(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.to_owned())
}

fn press_tab(editor: &ferroui_edit::TextEditor) -> Result<(), String> {
    let mut event = KeyEventArgs::new();
    event.key = Key::Tab;
    event.set_routed_event(Some(InputElement::key_down_event()));
    editor.text_area().raise_event(&event);
    verify(event.handled(), "Tab was not handled by the editor")
}

fn run_step(demo: &EditorDemo, step: usize) -> Result<bool, String> {
    let editor = demo.editor();
    let edit = |result: Result<(), ferroui_edit::core::editing::EditError>| {
        result.map_err(|error| error.to_string())
    };
    match step {
        0 => {
            let search = editor.search_panel();
            search.set_search_pattern("FerroUI");
            search.show(true);
            verify(search.is_open(), "Search overlay did not open")?;
            verify(search.find_next(), "Sample search returned no match")?;
            println!("Editor smoke: search overlay opened and selected a result");
        }
        1 => {
            let search = editor.search_panel();
            search.set_replacement_pattern("FerroUI Editor");
            verify(search.replace_next()?, "Search replacement did not run")?;
            search.close_panel();
            edit(editor.select(0, 0))?;
            edit(editor.replace_selection_with_text("pri"))?;
            demo.show_completion()?;
            let completion = CompletionWindow::active_for(&editor)
                .ok_or("Completion session did not stay active")?;
            verify(completion.is_open(), "Completion overlay did not open")?;
            println!("Editor smoke: replacement completed and completion overlay opened");
        }
        2 => {
            press_tab(&editor)?;
            verify(
                CompletionWindow::active_for(&editor).is_none(),
                "Tab did not close completion",
            )?;
            verify(
                editor.text().starts_with("println!("),
                "Tab did not commit the completion item",
            )?;
            edit(editor.select(0, 0))?;
            demo.insert_snippet()?;
            edit(editor.replace_selection_with_text("smoke_name"))?;
            verify(
                editor.text().matches("smoke_name").count() == 2,
                "Snippet bound field did not mirror the edit",
            )?;
            press_tab(&editor)?;
            verify(
                editor.selected_text().map_err(|error| error.to_string())? == "value",
                "Tab did not navigate to the second snippet field",
            )?;
            edit(editor.replace_selection_with_text("42"))?;
            press_tab(&editor)?;
            verify(
                editor.selection().is_empty(),
                "Final snippet caret is selected",
            )?;
            println!("Editor smoke: completion committed and snippet mirrored/navigated");
        }
        3 => {
            let manager = editor.folding_manager().ok_or("Folding is not installed")?;
            manager.set_all_folded(true, false);
            verify(
                manager
                    .all_foldings()
                    .iter()
                    .any(|section| section.is_folded()),
                "No sample region was folded",
            )?;
            editor.set_word_wrap(true);
            demo.toggle_theme()?;
            println!("Editor smoke: regions folded, wrapping enabled and theme changed");
        }
        4 => {
            editor
                .folding_manager()
                .ok_or("Folding is not installed")?
                .set_all_folded(false, false);
            demo.set_language(LanguageMode::CSharp)?;
            verify(
                editor.text().contains("class Program"),
                "Language switching did not load the C# example",
            )?;
            verify(
                editor
                    .folding_manager()
                    .is_some_and(|manager| manager.all_foldings().len() >= 2),
                "C# example regions were not recreated",
            )?;
            println!("Editor smoke: C# grammar and document installed");
        }
        5 => {
            let search = editor.search_panel();
            search.set_search_pattern("Console");
            search.show(false);
            verify(search.find_next(), "Search did not follow the new document")?;
            println!("Editor smoke: search reopened on the replacement document");
        }
        _ => {
            editor.search_panel().close_panel();
            return Ok(false);
        }
    }
    Ok(true)
}

fn shutdown(exit_code: i32) {
    let lifetime = Application::current().and_then(|app| app.application_lifetime());
    if let Some(lifetime) = lifetime
        .as_ref()
        .and_then(|lifetime| lifetime.as_controlled_application_lifetime())
    {
        lifetime.shutdown(exit_code);
    }
}
