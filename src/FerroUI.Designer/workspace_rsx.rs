use super::*;
use ferroui_base::media::TextWrapping;
use ferroui_controls::{ComboBox, Flyout, PlacementMode, ScrollViewer};
use ferroui_designer_language::LineIndex;

const XAML_TOOLS: &[&str] = &[
    "CompleteSource",
    "SourceDefinition",
    "SourceReferences",
    "SourceDiagnostics",
    "FormatSource",
    "SourceFormattingOptions",
    "RenameSource",
];

impl DesignerWorkspace {
    pub(super) fn connect_rust_source(self: &Rc<Self>) {
        let picker = self.root.get_control::<ComboBox>("RustInvocations");
        let weak = Rc::downgrade(self);
        let weak_picker = picker.downgrade();
        picker.selection_changed(move |_, _| {
            let (Some(this), Some(picker)) = (weak.upgrade(), weak_picker.upgrade()) else {
                return;
            };
            if this.guard.get() || this.pending_source.get() {
                return;
            }
            let Ok(index) = usize::try_from(picker.selected_index()) else {
                return;
            };
            let (start, revision) = {
                let session = this.session.borrow();
                let Some(span) = session.rust_map().and_then(|map| map.blocks.get(index)) else {
                    return;
                };
                (span.start, session.document().revision())
            };
            let weak = Rc::downgrade(&this);
            let weak_picker = picker.downgrade();
            // Let ComboBox finish resolving its selected item before rebuilding the picker.
            Dispatcher::ui_thread().post_local(
                move || {
                    let (Some(this), Some(picker)) = (weak.upgrade(), weak_picker.upgrade()) else {
                        return;
                    };
                    if this.pending_source.get()
                        || picker.selected_index() != index as i32
                        || this.session.borrow().document().revision() != revision
                    {
                        return;
                    }
                    let result = this.session.borrow_mut().select_rust_invocation(start);
                    this.refresh(true);
                    this.report(result);
                },
                ferroui_base::threading::DispatcherPriority::BACKGROUND,
            );
        });
        self.connect_animation_source_issues();
    }

    pub(super) fn refresh_rust_source_tools(self: &Rc<Self>) -> bool {
        let session = self.session.borrow();
        let rust = session.uses_rust_source();
        self.root
            .get_control::<Button>("Share")
            .set_is_visible(cfg!(feature = "collaboration-client") && !rust);
        let picker = self.root.get_control::<ComboBox>("RustInvocations");
        let issues = self.root.get_control::<Button>("AnimationSourceIssues");
        issues.set_is_visible(session.rust_map().is_some_and(|map| !map.issues.is_empty()));
        for name in XAML_TOOLS {
            self.root.get_control::<Button>(name).set_is_visible(!rust);
        }
        self.root
            .get_control::<TextBox>("RenameSymbol")
            .set_is_visible(!rust);
        let results = self.root.get_control::<StackPanel>("LanguageResults");
        if !rust {
            if picker.is_visible() {
                results.children().clear();
            }
            picker.set_is_visible(false);
            return false;
        }
        let index = LineIndex::new(session.authored_source());
        let guard = self.guard.replace(true);
        picker.items().clear();
        picker.set_placeholder_text(Some("RSX view".into()));
        if let Some(map) = session.rust_map() {
            for (i, block) in map.blocks.iter().enumerate() {
                let line = index
                    .position(block.start)
                    .map_or(1, |position| position.line + 1);
                picker
                    .items()
                    .add(Some(Rc::new(format!("RSX {} · Line {line}", i + 1))));
            }
            picker.set_selected_index(
                map.blocks
                    .iter()
                    .position(|block| *block == map.invocation)
                    .map_or(-1, |index| index as i32),
            );
        }
        picker.set_is_visible(true);
        picker.set_is_enabled(session.rust_map().is_some());
        self.guard.set(guard);

        let offset = super::source_tools::byte_from_utf16(
            session.authored_source(),
            self.source.caret_index(),
        );
        let label = index.position(offset).map_or_else(
            |_| "Rust".into(),
            |position| format!("Rust  {}:{}", position.line + 1, position.character + 1),
        );
        self.root
            .get_control::<TextBlock>("SourcePosition")
            .set_text(Some(&label));
        results.children().clear();
        for diagnostic in session.authored_diagnostics() {
            let row = button(&diagnostic.message);
            let span = diagnostic.span;
            let weak = Rc::downgrade(self);
            row.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.select_source_span(span);
                }
            });
            results.children().add(row.upcast::<Control>());
        }
        true
    }

    fn connect_animation_source_issues(self: &Rc<Self>) {
        let anchor = self.root.get_control::<Button>("AnimationSourceIssues");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Top);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let weak = Rc::downgrade(self);
        let weak_flyout = flyout.downgrade();
        flyout
            .opening(move |_| {
                let (Some(this), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade()) else {
                    return;
                };
                let session = this.session.borrow();
                let Some(map) = session.rust_map() else {
                    return;
                };
                let revision = session.document().revision();
                let rows = StackPanel::new();
                rows.set_width(320.0);
                rows.set_spacing(4.0);
                let index = LineIndex::new(session.authored_source());
                for (issue_index, issue) in map.issues.iter().enumerate() {
                    let row = Button::new();
                    row.classes().add("nav");
                    row.set_horizontal_alignment(HorizontalAlignment::Stretch);
                    row.set_horizontal_content_alignment(HorizontalAlignment::Left);
                    let text = TextBlock::new();
                    text.set_text_wrapping(TextWrapping::Wrap);
                    let line = index
                        .position(issue.declaration.start)
                        .map_or(1, |position| position.line + 1);
                    text.set_text(Some(&format!("Line {line} · {}", issue.reason)));
                    row.set_content(Some(Control::boxed(text)));
                    let weak_this = Rc::downgrade(&this);
                    let close = flyout.downgrade();
                    row.click(move |_, _| {
                        if let Some(flyout) = close.upgrade() {
                            flyout.hide();
                        }
                        if let Some(this) = weak_this.upgrade() {
                            this.report(this.reveal_rust_animation_issue(revision, issue_index));
                        }
                    });
                    rows.children().add(row.upcast::<Control>());
                }
                let scroll = ScrollViewer::new();
                scroll.set_max_height(220.0);
                scroll.set_content(Some(Control::boxed(rows)));
                flyout.set_content(Some(Control::boxed(scroll)));
            })
            .detach();
        anchor.set_flyout(Some(flyout.upcast()));
    }

    pub(crate) fn reveal_rust_animation_issue(
        &self,
        revision: u64,
        index: usize,
    ) -> Result<(), String> {
        let span = {
            let session = self.session.borrow();
            if session.document().revision() != revision || self.pending_source.get() {
                return Err("The source changed. Open the current animation issues.".into());
            }
            session
                .rust_map()
                .and_then(|map| map.issues.get(index))
                .ok_or("Choose an issue in the current RSX invocation")?
                .declaration
        };
        self.select_source_span(span);
        Ok(())
    }
}
