//! Preview environment belongs to the workspace, independently of source history and roles.
use super::*;
use crate::{PreviewEnvironment, PreviewEnvironmentEditor, PreviewFlowDirection, PreviewTheme};
use ferroui_base::Visual;
use ferroui_base::media::{FlowDirection, SolidColorBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::{Flyout, ThemeVariantScope};
use std::rc::Weak;

#[derive(Default)]
pub(super) struct PreviewEnvironmentUi {
    pub(super) value: PreviewEnvironment,
    editor: Option<Rc<PreviewEnvironmentEditor>>,
    theme_subscription: Option<Rc<dyn ferroui_base::reactive::IDisposable>>,
    invalidation: Option<Rc<PreviewEnvironmentInvalidation>>,
}

impl Drop for PreviewEnvironmentUi {
    fn drop(&mut self) {
        if let Some(subscription) = self.theme_subscription.take() {
            subscription.dispose();
        }
    }
}

/// Native inheritance can change synchronously while the session is already borrowed
/// by layout or preview replacement. Coalesce one explicit environment update, then
/// retire its accepted models immediately when possible or on the next UI turn.
struct PreviewEnvironmentInvalidation {
    workspace: Weak<DesignerWorkspace>,
    update_depth: Cell<usize>,
    pending: Cell<bool>,
    queued: Cell<bool>,
    retry: RefCell<Option<Rc<dyn ferroui_base::reactive::IDisposable>>>,
}

impl PreviewEnvironmentInvalidation {
    fn begin(self: &Rc<Self>) -> PreviewEnvironmentUpdate {
        self.update_depth.set(self.update_depth.get() + 1);
        PreviewEnvironmentUpdate(self.clone())
    }

    fn changed(self: &Rc<Self>) {
        self.pending.set(true);
        self.flush();
    }

    fn try_flush(&self) -> bool {
        if !self.pending.get() {
            return true;
        }
        if self.update_depth.get() != 0 {
            return false;
        }
        let Some(workspace) = self.workspace.upgrade() else {
            return true;
        };
        let Ok(mut session) = workspace.session.try_borrow_mut() else {
            return false;
        };
        self.pending.set(false);
        session.definition_scope_epoch = session.definition_scope_epoch.wrapping_add(1);
        session.generation = session.generation.wrapping_add(1);
        drop(session);
        workspace.last_revision.set(u64::MAX);
        workspace.surface.invalidate_layout_helpers();
        workspace.surface.invalidate_visual();
        workspace.refresh_shell();
        true
    }

    fn flush(self: &Rc<Self>) {
        if self.update_depth.get() != 0 || self.try_flush() || self.queued.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                if this.try_flush() {
                    this.queued.set(false);
                    return;
                }
                // A nested dispatcher pump may still be inside the original borrow.
                // Retry on a timer instead of continuously posting into that pump.
                let weak = Rc::downgrade(&this);
                let retry = DispatcherTimer::run(
                    move || {
                        let Some(this) = weak.upgrade() else {
                            return false;
                        };
                        if this.try_flush() {
                            this.queued.set(false);
                            false
                        } else {
                            true
                        }
                    },
                    Duration::from_millis(1),
                    DispatcherPriority::SEND,
                );
                *this.retry.borrow_mut() = Some(retry);
            },
            DispatcherPriority::SEND,
        );
    }
}

impl Drop for PreviewEnvironmentInvalidation {
    fn drop(&mut self) {
        if let Some(retry) = self.retry.get_mut().take() {
            retry.dispose();
        }
    }
}

struct PreviewEnvironmentUpdate(Rc<PreviewEnvironmentInvalidation>);

impl Drop for PreviewEnvironmentUpdate {
    fn drop(&mut self) {
        self.0.update_depth.set(self.0.update_depth.get() - 1);
        self.0.flush();
    }
}

impl DesignerWorkspace {
    pub fn preview_environment(&self) -> PreviewEnvironment {
        let (width, height) = self.artboard_size();
        PreviewEnvironment {
            width,
            height,
            ..self.preview_environment.borrow().value
        }
    }

    /// Applies inherited preview conditions. Explicit document values retain normal native
    /// precedence. The source buffer, undo history, collaboration and editor chrome are intact.
    pub fn set_preview_environment(&self, value: PreviewEnvironment) -> Result<(), String> {
        value.validate()?;
        let changed = self.preview_environment() != value;
        let invalidation = self
            .preview_environment
            .borrow()
            .invalidation
            .as_ref()
            .expect("Preview environment is connected before use")
            .clone();
        let _update = invalidation.begin();
        self.artboard.set_width(value.width);
        self.artboard.set_height(value.height);
        let scope = self
            .root
            .get_control::<ThemeVariantScope>("PreviewThemeScope");
        scope.set_requested_theme_variant(match value.theme {
            PreviewTheme::Light => Some(ThemeVariant::light()),
            PreviewTheme::Dark => Some(ThemeVariant::dark()),
            PreviewTheme::Inherit => None,
        });
        match value.flow_direction {
            PreviewFlowDirection::Inherit => scope.clear_value(Visual::flow_direction_property()),
            PreviewFlowDirection::LeftToRight => {
                scope.set_flow_direction(FlowDirection::LeftToRight)
            }
            PreviewFlowDirection::RightToLeft => {
                scope.set_flow_direction(FlowDirection::RightToLeft)
            }
        }
        self.artboard.set_background(Some(
            SolidColorBrush::parse(
                if scope.actual_theme_variant() == Some(ThemeVariant::dark()) {
                    "#FF202020"
                } else {
                    "#FFFFFFFF"
                },
            )
            .unwrap()
            .into(),
        ));
        self.root
            .get_control::<Button>("PreviewTheme")
            .set_content(Some(Rc::new(
                match value.theme {
                    PreviewTheme::Light => "Light preview",
                    PreviewTheme::Dark => "Dark preview",
                    PreviewTheme::Inherit => "App theme",
                }
                .to_owned(),
            )));
        self.preview_environment.borrow_mut().value = value;
        if changed {
            invalidation.changed();
        }
        Ok(())
    }

    pub fn rotate_preview(&self) -> Result<(), String> {
        self.set_preview_environment(self.preview_environment().rotated())
    }

    pub(super) fn connect_preview_environment(self: &Rc<Self>) {
        let scope = self
            .root
            .get_control::<ThemeVariantScope>("PreviewThemeScope");
        let weak_scope = scope.downgrade();
        let weak_artboard = self.artboard.downgrade();
        let invalidation = Rc::new(PreviewEnvironmentInvalidation {
            workspace: Rc::downgrade(self),
            update_depth: Cell::new(0),
            pending: Cell::new(false),
            queued: Cell::new(false),
            retry: RefCell::new(None),
        });
        self.preview_environment.borrow_mut().invalidation = Some(invalidation.clone());
        let subscription = scope.property_changed(move |change| {
            let theme = change.property()
                == ThemeVariantScope::actual_theme_variant_property().as_property();
            let direction = change.property() == Visual::flow_direction_property().as_property();
            if !change.is_effective_value_change() || (!theme && !direction) {
                return;
            }
            if theme {
                if let (Some(scope), Some(artboard)) =
                    (weak_scope.upgrade(), weak_artboard.upgrade())
                {
                    artboard.set_background(Some(
                        SolidColorBrush::parse(
                            if scope.actual_theme_variant() == Some(ThemeVariant::dark()) {
                                "#FF202020"
                            } else {
                                "#FFFFFFFF"
                            },
                        )
                        .unwrap()
                        .into(),
                    ));
                }
            }
            invalidation.changed();
        });
        self.preview_environment.borrow_mut().theme_subscription = Some(subscription);
        let button = self.root.get_control::<Button>("PreviewEnvironment");
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        button.set_flyout(Some(flyout.clone().upcast()));
        let weak = Rc::downgrade(self);
        let weak_flyout = flyout.downgrade();
        flyout.opening(move |_| {
            let (Some(this), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade()) else {
                return;
            };
            let editor = PreviewEnvironmentEditor::new(this.preview_environment());
            let (width, height) = this.session.borrow().design_preview_size();
            editor.set_design_size_hint(width, height);
            let weak = Rc::downgrade(&this);
            let close = flyout.downgrade();
            editor.on_apply(move |value| {
                let this = weak.upgrade().ok_or("The designer workspace was closed")?;
                this.set_preview_environment(value)?;
                if let Some(flyout) = close.upgrade() {
                    flyout.hide();
                }
                Ok(())
            });
            let close = flyout.downgrade();
            editor.on_cancel(move || {
                if let Some(flyout) = close.upgrade() {
                    flyout.hide();
                }
            });
            flyout.set_content(Some(Control::boxed(&editor.control())));
            this.preview_environment.borrow_mut().editor = Some(editor);
        });
        let weak = Rc::downgrade(self);
        flyout.closed(move || {
            if let Some(this) = weak.upgrade() {
                if let Some(editor) = this.preview_environment.borrow_mut().editor.take() {
                    editor.retire();
                }
            }
        });
    }
}
