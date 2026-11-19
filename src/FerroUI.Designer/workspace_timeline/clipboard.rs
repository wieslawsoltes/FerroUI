#[path = "paste_picker.rs"]
mod paste_picker;

use super::*;
use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_controls::{MenuFlyout, MenuItem, TopLevel};
use ferroui_designer_animation::{KeyframeClipboard, KeyframeDestination, KeyframePaste};

#[derive(Clone, Copy)]
enum Command {
    Copy,
    Paste { reverse: bool },
    Mirror,
}

impl DesignerWorkspace {
    pub(super) fn timeline_keyframe_menu(
        self: &Rc<Self>,
        context: &TimelineContext,
        selected: bool,
    ) -> Ref<Button> {
        let control = button("");
        control.set_name(Some("TimelineKeyframeActions".into()));
        control.set_content(Some(Control::boxed(crate::icons::vector_icon(
            crate::icons::MORE,
            12.0,
        ))));
        control.classes().add("designer-timeline-tool");
        control.classes().add("nav");
        control.set_padding(Thickness::symmetric(5.0, 2.0));
        control.set_min_width(24.0);
        AutomationProperties::set_name(&control, Some("Keyframe Actions"));
        ToolTip::set_tip(&control, Some(Rc::new("Keyframe Actions".to_owned())));
        let menu = MenuFlyout::new();
        let writable = !self.session.borrow().is_read_only();
        for (label, command, enabled) in [
            ("Copy Keyframes", Command::Copy, selected),
            (
                "Paste at Playhead",
                Command::Paste { reverse: false },
                writable,
            ),
            (
                "Paste Reversed at Playhead",
                Command::Paste { reverse: true },
                writable,
            ),
            (
                "Mirror About Playhead",
                Command::Mirror,
                selected && writable,
            ),
        ] {
            let item = MenuItem::new();
            item.set_header(Some(Rc::new(label.to_owned())));
            item.set_is_enabled(enabled);
            let weak = Rc::downgrade(self);
            let anchor = control.downgrade();
            let context = context.clone();
            item.click(move |_, event| {
                if let (Some(workspace), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                    workspace.request_keyframe_transfer(context.clone(), &anchor, command, None);
                }
                event.set_handled(true);
            });
            menu.items().add(Some(Control::boxed(item)));
        }
        control.set_flyout(Some(menu.upcast()));
        crate::chrome::apply_designer_menu_resources(&control, &self.root);
        control
    }

    pub(super) fn connect_timeline_clipboard(
        self: &Rc<Self>,
        lane: &Ref<TimelineLane>,
        context: &TimelineContext,
        track: Option<&Track>,
    ) {
        if context.program.is_none() {
            return;
        }
        let weak = Rc::downgrade(self);
        let anchor = lane.downgrade();
        let context = context.clone();
        let destination = track.map(|track| KeyframeDestination {
            target: track.target,
            property: track.property.clone(),
        });
        lane.add_handler(InputElement::key_down_event(), move |_, event| {
            let modifiers = event.key_modifiers;
            if modifiers.contains(KeyModifiers::ALT)
                || !modifiers.intersects(KeyModifiers::META | KeyModifiers::CONTROL)
            {
                return;
            }
            let command = match event.key {
                Key::C => Command::Copy,
                Key::V => Command::Paste {
                    reverse: modifiers.contains(KeyModifiers::SHIFT),
                },
                _ => return,
            };
            if let (Some(workspace), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                event.set_handled(true);
                workspace.request_keyframe_transfer(
                    context.clone(),
                    &anchor,
                    command,
                    destination.clone(),
                );
            }
        });
    }

    pub(super) fn close_timeline_paste(&self) {
        let flyout = self.timeline_ui.borrow_mut().keyframe_paste.take();
        if let Some(flyout) = flyout {
            flyout.hide();
            flyout.set_content(None);
        }
    }

    fn request_keyframe_transfer(
        self: &Rc<Self>,
        context: TimelineContext,
        anchor: &Control,
        command: Command,
        preferred: Option<KeyframeDestination>,
    ) {
        let weak = Rc::downgrade(self);
        let anchor = anchor.to_ref().downgrade();
        Dispatcher::ui_thread().spawn_local(async move {
            let Some(workspace) = weak.upgrade() else {
                return;
            };
            let result = workspace
                .transfer_keyframes(context, anchor, command, preferred)
                .await;
            workspace.report(result);
        });
    }

    async fn transfer_keyframes(
        self: &Rc<Self>,
        context: TimelineContext,
        anchor: WeakRef<Control>,
        command: Command,
        preferred: Option<KeyframeDestination>,
    ) -> Result<(), String> {
        let program = context.transfer_program(self, !matches!(command, Command::Copy))?;
        let time = self.session.borrow().time();
        if matches!(command, Command::Mirror) {
            let ui = self.timeline_ui.borrow();
            let keys = ui.selection.clone();
            let primary = ui
                .selected
                .as_ref()
                .and_then(|primary| keys.iter().position(|key| key == primary));
            drop(ui);
            let keys = self
                .session
                .borrow_mut()
                .mirror_program_keyframes(program, &keys, time)?;
            self.accept_timeline_key_batch(primary, keys);
            self.refresh(true);
            return Ok(());
        }
        let clipboard = TopLevel::get_top_level(Some(&self.root))
            .and_then(|window| window.clipboard())
            .ok_or("The Studio window does not provide a clipboard")?;
        match command {
            Command::Copy => {
                let keys = self.timeline_ui.borrow().selection.clone();
                let data = self
                    .session
                    .borrow()
                    .copy_program_keyframes(program, &keys)?;
                let text = data.to_text().map_err(|error| error.to_string())?;
                clipboard
                    .set_text_async(Some(&text))
                    .await
                    .map_err(|error| error.to_string())?;
                self.status
                    .set_text(Some(&format!("Copied {} FerroUI keyframes", keys.len())));
                Ok(())
            }
            Command::Paste { reverse } => {
                let text = clipboard
                    .try_get_text_async()
                    .await
                    .map_err(|error| error.to_string())?;
                let clipboard = KeyframeClipboard::from_text(text.as_deref().unwrap_or_default())
                    .map_err(|error| error.to_string())?;
                context.transfer_program(self, true)?;
                let choices = self.paste_choices(&clipboard)?;
                let mut paste = KeyframePaste {
                    clipboard,
                    destinations: Vec::new(),
                    time,
                    reverse,
                };
                if let Some(destination) = preferred.filter(|target| {
                    choices.len() == 1
                        && choices[0]
                            .iter()
                            .any(|choice| &choice.destination == target)
                }) {
                    paste.destinations.push(destination);
                    return self.apply_timeline_paste(&context, paste);
                }
                let anchor = anchor
                    .upgrade()
                    .ok_or("The timeline changed before pasting")?;
                self.open_timeline_paste(context, &anchor, paste, choices);
                Ok(())
            }
            Command::Mirror => unreachable!("mirror edits do not read the clipboard"),
        }
    }

    fn apply_timeline_paste(
        self: &Rc<Self>,
        context: &TimelineContext,
        paste: KeyframePaste,
    ) -> Result<(), String> {
        let program = context.transfer_program(self, true)?;
        let keys = self
            .session
            .borrow_mut()
            .paste_program_keyframes(program, &paste)?;
        self.close_timeline_paste();
        self.accept_timeline_key_batch(None, keys);
        self.refresh(true);
        Ok(())
    }
}

impl TimelineContext {
    fn transfer_program(
        &self,
        workspace: &DesignerWorkspace,
        writing: bool,
    ) -> Result<NodeId, String> {
        self.check(workspace)?;
        let session = workspace.session.borrow();
        if workspace.pending_source.get()
            || session.is_playing()
            || (writing && session.is_read_only())
        {
            return Err("Pause playback and use the current writable timeline".into());
        }
        self.program
            .ok_or_else(|| "Choose a typed animation program".into())
    }
}
