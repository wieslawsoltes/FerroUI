//! Optional application-lifetime checks that allow overlays to render between
//! actions. The regular demo does not run or schedule these checks.

use crate::{EditorDemo, LanguageMode};
use ferroui_base::{Point, Ref};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::automation::provider::{ITextProvider, ITextProvider2};
use ferroui_controls::{Application, Border, Button};
use ferroui_edit::{
    CompletionWindow, FoldingMargin, ITextRunConstructionContext, ITextViewConnect, InlineObjectElement,
    KnownLayer, LayerInsertionPosition, LineNumberMargin, TextViewMargin, VisualLineElement, VisualLineElementGenerator,
    VisualLineError,
};
use std::{cell::RefCell, rc::Rc};
use std::time::Duration;

pub(crate) fn start(demo: Rc<EditorDemo>) {
    let button = Button::new();
    button.set_content(Some(Rc::new("Inline".to_owned())));
    button.set_width(72.0);
    button.set_height(28.0);
    schedule(demo, Rc::new(InlineProbe(button)), 0);
}

fn schedule(demo: Rc<EditorDemo>, inline: Rc<InlineProbe>, step: usize) {
    DispatcherTimer::run_once(
        move || match run_step(&demo, &inline, step) {
            Ok(true) => schedule(demo.clone(), inline.clone(), step + 1),
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
    )
    .detach();
}

fn verify(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.to_owned())
}

fn press_tab(editor: &ferroui_edit::TextEditor) -> Result<(), String> {
    press_key(editor, Key::Tab, KeyModifiers::empty())
}

fn press_key(
    editor: &ferroui_edit::TextEditor,
    key: Key,
    modifiers: KeyModifiers,
) -> Result<(), String> {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.key_modifiers = modifiers;
    event.set_routed_event(Some(InputElement::key_down_event()));
    editor.text_area().raise_event(&event);
    verify(event.handled(), &format!("{key:?} was not handled by the editor"))
}

fn verify_accessible_caret(editor: &ferroui_edit::TextEditor) -> Result<(), String> {
    editor.set_text("alpha beta\r\nomega gamma").map_err(|error| error.to_string())?;
    editor.set_caret_offset(0).map_err(|error| error.to_string())?;
    editor.text_area().focus();
    let peer = ControlAutomationPeer::create_peer_for_element(editor);
    verify(peer.has_keyboard_focus(), "The automation peer did not report keyboard focus")?;
    let text = peer.get_provider::<dyn ITextProvider>().ok_or("Editor Text pattern is missing")?;
    let text2 = peer.get_provider::<dyn ITextProvider2>().ok_or("Editor Text2 pattern is missing")?;
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    let subscription = text.clone().text_selection_changed(Rc::new(move || {
        let caret = text2.get_caret_range().ok().filter(|caret| caret.is_active)
            .and_then(|caret| caret.range?.text_offsets().ok().flatten());
        let selection = text.get_selection().ok().and_then(|ranges| match ranges.as_slice() {
            [range] => range.text_offsets().ok().flatten(),
            _ => None,
        });
        observed.borrow_mut().push(caret.zip(selection));
    })).map_err(|error| error.to_string())?;
    for (key, modifiers, caret, selection) in [
        (Key::Down, KeyModifiers::empty(), 12..12, 12..12),
        (Key::Right, KeyModifiers::empty(), 13..13, 13..13),
        (Key::Right, KeyModifiers::SHIFT, 14..14, 13..14),
    ] {
        press_key(editor, key, modifiers)?;
        verify(
            changes.borrow_mut().drain(..).eq([Some((caret, selection))]),
            "The keyboard move did not publish one selection notification with the updated active caret and selection",
        )?;
    }
    subscription.dispose();
    println!("Editor smoke: keyboard navigation published the active caret and selection ranges");
    Ok(())
}

fn run_step(demo: &EditorDemo, inline: &Rc<InlineProbe>, step: usize) -> Result<bool, String> {
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
            let completion = CompletionWindow::active_for(&editor)
                .ok_or("Completion closed before its description rendered")?;
            let tip = completion.opened_popups().into_iter().next()
                .ok_or("The completion description popup did not open")?;
            let list = completion.child().ok_or("Completion has no list surface")?;
            let tip_host = tip.host().ok_or("The description has no host")?.as_control();
            let tip_origin = tip_host.translate_point(Point::default(), &list)
                .ok_or("The description is outside the editor window")?;
            verify(
                tip_origin.x >= list.bounds().width
                    || tip_origin.x + tip_host.bounds().width <= 2.5,
                "The description did not render beside its list",
            )?;
            press_tab(&editor)?;
            verify(
                CompletionWindow::active_for(&editor).is_none() && !tip.is_open(),
                "Tab did not close completion and its description",
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
            println!("Editor smoke: completion description rendered beside the list and closed on commit");
            println!("Editor smoke: snippet mirrored and navigated");
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
        6 => {
            editor.search_panel().close_panel();
            let mut bytes = Vec::new();
            editor.set_encoding(ferroui_edit::core::utils::TextEncoding::Utf16Le);
            editor.set_write_bom(true);
            let original = editor.text();
            editor.save(&mut bytes).map_err(|error| error.to_string())?;
            edit(editor.replace_selection_with_text("changed"))?;
            verify(
                editor.is_modified(),
                "Edited document is not marked modified",
            )?;
            editor
                .load(bytes.as_slice())
                .map_err(|error| error.to_string())?;
            verify(
                editor.text() == original && !editor.is_modified(),
                "Encoded file round trip failed",
            )?;
            println!("Editor smoke: encoded save and load restored the clean document");
        }
        7 => editor
            .text_area()
            .element_generators()
            .add(inline.clone().into()),
        8 => {
            verify(
                inline.0.visual_parent().is_some() && inline.0.bounds().width == 72.0,
                "The inline button was not hosted and arranged",
            )?;
            let peer = ControlAutomationPeer::create_peer_for_element(&editor);
            let child = ControlAutomationPeer::create_peer_for_element(&inline.0);
            let text = peer.get_provider::<dyn ITextProvider>().ok_or("Editor Text pattern is missing")?;
            let document = text.document_range().map_err(|error| error.to_string())?;
            verify(
                document.get_text(None).map_err(|error| error.to_string())? == editor.text(),
                "The accessible document differs from the live editor",
            )?;
            verify(
                document.get_children().map_err(|error| error.to_string())?.contains(&child),
                "The accessible range did not report the hosted inline button",
            )?;
            let bounds = child.get_bounding_rectangle();
            let point = Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0);
            let hit = text.embedded_range_from_point(point)
                .map_err(|error| error.to_string())?
                .ok_or("An accessibility hit on the inline button did not return its text range")?;
            let expected = text.range_from_child(&child).map_err(|error| error.to_string())?;
            verify(hit.compare(&*expected).map_err(|error| error.to_string())?,
                "The accessibility hit selected the wrong inline object range")?;
            verify(
                hit.get_enclosing_element().map_err(|error| error.to_string())? == child
                    && expected.get_enclosing_element().map_err(|error| error.to_string())? == child
                    && expected.get_children().map_err(|error| error.to_string())?.is_empty(),
                "The inline range did not resolve to its own accessibility peer",
            )?;
            println!("Editor smoke: Text pattern exposed the document and inline button");
            let area = editor.text_area();
            let margin = Border::new();
            margin.set_width(24.0);
            let margin = TextViewMargin::new(margin.upcast());
            margin.set_text_view(Some(&area)).map_err(|error| error.to_string())?;
            area.insert_left_margin(0, margin)
                .map_err(|error| error.to_string())?;
            let overlay = Border::new();
            overlay.set_is_hit_test_visible(false);
            area.insert_layer(overlay.upcast(), KnownLayer::Caret, LayerInsertionPosition::Above)
                .map_err(|error| error.to_string())?;
        }
        9 => {
            let area = editor.text_area();
            let numbers = area.visual_children().snapshot().iter()
                .find_map(|child| child.cast::<LineNumberMargin>())
                .ok_or("The editor's native line-number margin is missing")?;
            let folding = area.visual_children().snapshot().iter()
                .find_map(|child| child.cast::<FoldingMargin>())
                .ok_or("The editor's native folding margin is missing")?;
            let margin = area.left_margins().into_iter().next()
                .ok_or("The custom margin was not registered")?
                .control();
            let overlay = area.layers().into_iter().next()
                .ok_or("The custom layer was not registered")?
                .control;
            let canvas = overlay.visual_parent().ok_or("The editor layer has no visual parent")?;
            verify(
                margin.bounds().width == 24.0
                    && numbers.bounds().width > 0.0
                    && numbers.text_view().is_some_and(|view| view.ptr_eq(&area))
                    && folding.bounds().width > 0.0
                    && folding.text_view().is_some_and(|view| view.ptr_eq(&area))
                    && canvas.bounds() == area.content_viewport()
                    && canvas.clip_to_bounds()
                    && canvas.is_visual_ancestor_of(&inline.0),
                "Native margins and layers did not arrange around the inline control",
            )?;
            let detached = area.remove_left_margin(0).map_err(|error| error.to_string())?;
            area.remove_layer(&overlay).map_err(|error| error.to_string())?;
            verify(
                margin.visual_parent().is_none() && overlay.visual_parent().is_none(),
                "Removed editor adornments kept a native parent",
            )?;
            verify(
                detached.text_view().is_some_and(|view| view.ptr_eq(&area))
                    && detached.document() == Some(editor.document()),
                "The explicitly bound margin lost its view when detached",
            )?;
            detached.set_text_view(None).map_err(|error| error.to_string())?;
            verify(detached.document().is_none(), "The cleared margin retained the document")?;
            println!("Editor smoke: custom margin and layer arranged and detached, and explicit view binding survived removal");
            area
                .element_generators()
                .remove(&inline.clone().into());
        }
        _ => {
            verify(
                inline.0.visual_parent().is_none() && inline.0.parent().is_none(),
                "The inline button kept a parent after its generator was removed",
            )?;
            println!("Editor smoke: inline control arranged and detached");
            editor.search_panel().close_panel();
            verify_accessible_caret(&editor)?;
            return Ok(false);
        }
    }
    Ok(true)
}

struct InlineProbe(Ref<Button>);

impl ITextViewConnect for InlineProbe {}

impl VisualLineElementGenerator for InlineProbe {
    fn get_first_interested_offset(
        &self,
        start: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        Ok((start == 0).then_some(0))
    }

    fn construct_element(
        &self,
        _offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        Ok(Some(Box::new(InlineObjectElement::new(
            0,
            self.0.clone().upcast(),
            &**context.global_text_run_properties(),
        )?)))
    }
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
