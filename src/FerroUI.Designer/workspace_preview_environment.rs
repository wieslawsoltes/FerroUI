//! Preview environment belongs to the workspace, independently of source history and roles.
use super::*;
use crate::{PreviewEnvironment, PreviewEnvironmentEditor, PreviewFlowDirection, PreviewTheme};
use ferroui_base::Visual;
use ferroui_base::media::{FlowDirection, SolidColorBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::{Flyout, ThemeVariantScope};

#[derive(Default)]
pub(super) struct PreviewEnvironmentUi {
    pub(super) value: PreviewEnvironment,
    editor: Option<Rc<PreviewEnvironmentEditor>>,
    theme_subscription: Option<Rc<dyn ferroui_base::reactive::IDisposable>>,
}

impl Drop for PreviewEnvironmentUi {
    fn drop(&mut self) {
        if let Some(subscription) = self.theme_subscription.take() {
            subscription.dispose();
        }
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
            // Layout and inherited theme/direction can change without a source revision.
            // Retire accepted gestures and editor drafts before the next native layout pass.
            let mut session = self.session.borrow_mut();
            session.definition_scope_epoch = session.definition_scope_epoch.wrapping_add(1);
            session.generation = session.generation.wrapping_add(1);
            self.last_revision.set(u64::MAX);
        }
        self.surface.invalidate_layout_helpers();
        self.surface.invalidate_visual();
        self.refresh_shell();
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
        let subscription = scope.property_changed(move |change| {
            if change.property() != ThemeVariantScope::actual_theme_variant_property().as_property()
            {
                return;
            }
            if let (Some(scope), Some(artboard)) = (weak_scope.upgrade(), weak_artboard.upgrade()) {
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
