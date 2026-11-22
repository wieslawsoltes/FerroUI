use editor_demo::EditorDemo;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::{AppBuilder, Application, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_edit::core::editing::RectangleSelection;
use ferroui_edit::{ClipboardCopyOutcome, ClipboardPasteOutcome, EditorClipboard};
use std::ffi::OsString;
use std::process::ExitCode;
use std::rc::Rc;

const SOURCE: &str = "uvwxyz\nuijklm\nuopqrs";
const TARGET: &str = "12\n34\n56";

#[derive(Clone, Copy)]
pub(super) enum EditorFixture {
    Clipboard {
        copy: bool,
        rectangle: bool,
    },
    #[cfg(windows)]
    DragRender,
    #[cfg(any(windows, target_os = "macos"))]
    TextAutomation,
    #[cfg(windows)]
    InlineControl,
}

impl EditorFixture {
    pub(super) fn from_args(args: &[OsString]) -> Option<Self> {
        #[cfg(windows)]
        if matches!(args, [action] if action == "drag-render") {
            return Some(Self::DragRender);
        }
        #[cfg(any(windows, target_os = "macos"))]
        if matches!(args, [action] if action == "text-automation") {
            return Some(Self::TextAutomation);
        }
        #[cfg(windows)]
        if matches!(args, [action] if action == "inline-control") {
            return Some(Self::InlineControl);
        }
        let [action, shape] = args else {
            return None;
        };
        let copy = match action.to_str()? {
            "copy" => true,
            "paste" => false,
            _ => return None,
        };
        let rectangle = match shape.to_str()? {
            "rectangle" => true,
            "line" => false,
            _ => return None,
        };
        Some(Self::Clipboard { copy, rectangle })
    }

    pub(super) fn run(self) -> ExitCode {
        let code = AppBuilder::configure::<Application>()
            .use_platform_detect()
            .after_setup(move |builder| {
                let app = builder
                    .instance()
                    .expect("editor clipboard test application");
                let lifetime = app
                    .application_lifetime()
                    .expect("editor clipboard test lifetime");
                ferroui_markup_xaml::register_types();
                ferroui_edit::register_types();
                app.styles()
                    .add(ferroui_themes_fluent::FluentTheme::new().as_style());
                let demo = EditorDemo::new(false).expect("editor clipboard test view");
                let window = Window::new();
                #[cfg(windows)]
                if matches!(self, Self::DragRender) {
                    window.set_width(1080.0);
                    window.set_height(620.0);
                }
                #[cfg(target_os = "macos")]
                {
                    window.set_show_activated(false);
                    if matches!(self, Self::TextAutomation) {
                        window.set_width(850.0);
                        window.set_height(480.0);
                    }
                }
                window.set_content(Some(Rc::new(demo.view())));
                lifetime
                    .as_classic_desktop_style_application_lifetime()
                    .unwrap()
                    .set_main_window(Some(window.clone()));
                drop(Dispatcher::ui_thread().spawn_local(async move {
                    let result = match self {
                        Self::Clipboard { copy, rectangle } => {
                            exchange(&demo, &window, copy, rectangle).await
                        }
                        #[cfg(windows)]
                        Self::DragRender => {
                            crate::windows_pointer_render::check(&demo, &window).await
                        }
                        #[cfg(windows)]
                        Self::TextAutomation => {
                            crate::windows_text_automation::check(&demo, &window).await
                        }
                        #[cfg(target_os = "macos")]
                        Self::TextAutomation => crate::macos_text_automation::check(&demo, &window),
                        #[cfg(windows)]
                        Self::InlineControl => {
                            crate::windows_inline_control::check(&demo, &window).await
                        }
                    };
                    #[cfg(target_os = "macos")]
                    if result.is_ok() && matches!(self, Self::Clipboard { copy: true, .. }) {
                        // AppKit requests the promised custom formats while Chrome reads.
                        // The browser test terminates this peer after the paste and undo.
                        println!("FERROUI_EDITOR_CLIPBOARD_READY");
                        return;
                    }
                    let code = if let Err(error) = result {
                        eprintln!("{error}");
                        1
                    } else {
                        0
                    };
                    lifetime
                        .as_controlled_application_lifetime()
                        .unwrap()
                        .shutdown(code);
                }));
            })
            .start_with_classic_desktop_lifetime(&[]);
        ExitCode::from(code as u8)
    }
}

async fn exchange(
    demo: &EditorDemo,
    window: &Window,
    copy: bool,
    rectangle: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let clipboard = window
        .clipboard()
        .ok_or("Native editor window has no clipboard")?;
    let editor = demo.editor();
    if copy {
        editor.set_text(SOURCE)?;
        if rectangle {
            editor.set_selection(
                RectangleSelection {
                    anchor_line: 0,
                    anchor_column: 1,
                    caret_line: 2,
                    caret_column: 5,
                }
                .to_selection(SOURCE, 4),
            )?;
        } else {
            editor.set_caret_offset(1)?;
        }
        if EditorClipboard::copy_to_clipboard(&editor, clipboard.clone(), false).await?
            != ClipboardCopyOutcome::Copied
        {
            return Err("Native copy did not publish its selection".into());
        }
        clipboard.flush_async().await?;
    } else {
        editor.set_text(TARGET)?;
        editor.set_caret_offset(if rectangle { 1 } else { 4 })?;
        let expected = if rectangle {
            "1bcde2\n3hijk4\n5nopq6"
        } else {
            "12\nabcdef\n34\n56"
        };
        let outcome = EditorClipboard::paste_from_clipboard(&editor, clipboard).await?;
        if outcome != ClipboardPasteOutcome::Pasted
            || editor.text().replace("\r\n", "\n") != expected
        {
            return Err(format!("Unexpected native paste {outcome:?}: {:?}", editor.text()).into());
        }
        if !editor.undo()? || editor.text() != TARGET {
            return Err("Native paste did not undo in one step".into());
        }
    }
    Ok(())
}
