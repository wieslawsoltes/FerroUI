//! On-demand value trace from the accepted preview on a native property row.
use super::{
    DesignerWorkspace,
    inspector_actions::{SOURCE, TRACE, icon_button, text, tool_button},
};
use crate::{
    design_time::DesignTimeSnapshot,
    property_trace::{TraceLocation, TraceTarget},
};
use ferroui_base::{
    Ref, WeakRef,
    media::{FontWeight, TextWrapping},
};
use ferroui_controls::{
    Button, ColumnDefinitions, Control, Flyout, Grid, PlacementMode, ScrollViewer, StackPanel,
};
use ferroui_designer_core::SourceSpan;
use std::rc::{Rc, Weak};

#[cfg(test)]
#[path = "workspace_property_trace_tests.rs"]
mod tests;

pub(super) type TraceAction = Rc<dyn Fn() -> Ref<Button>>;

#[derive(Clone)]
struct TraceContext {
    workspace: Weak<DesignerWorkspace>,
    snapshot: DesignTimeSnapshot,
    target: TraceTarget,
    property: String,
}

impl DesignerWorkspace {
    pub(super) fn property_trace_action(
        self: &Rc<Self>,
        snapshot: &DesignTimeSnapshot,
        target: TraceTarget,
        property: &str,
    ) -> TraceAction {
        let context = TraceContext {
            workspace: Rc::downgrade(self),
            snapshot: snapshot.clone(),
            target,
            property: property.into(),
        };
        Rc::new(move || context.button())
    }
}

impl TraceContext {
    fn current(&self, workspace: &DesignerWorkspace, session: &crate::DesignerSession) -> bool {
        let expected_runtime = match self.target {
            TraceTarget::Authored(_) => None,
            TraceTarget::Runtime(id) => Some(id),
        };
        !workspace.pending_source.get()
            && workspace.runtime_selection.get() == expected_runtime
            && self.snapshot.is_current(session)
    }

    fn button(&self) -> Ref<Button> {
        let button = tool_button(TRACE, &format!("Trace {} value", self.property));
        button.set_min_width(22.0);
        button.set_width(22.0);
        button.set_padding(ferroui_base::Thickness::uniform(2.0));
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Left);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let context = self.clone();
        let weak = flyout.downgrade();
        flyout
            .opening(move |_| {
                let Some(flyout) = weak.upgrade() else { return };
                let content = context
                    .content(&flyout)
                    .unwrap_or_else(|error| text(&error).upcast());
                flyout.set_content(Some(Control::boxed(content)));
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
        button
    }

    fn content(&self, flyout: &Ref<Flyout>) -> Result<Ref<Control>, String> {
        let workspace = self.workspace.upgrade().ok_or("The designer was closed")?;
        let session = workspace.session.borrow();
        if !self.current(&workspace, &session) {
            return Err("The document or selection changed. Reopen the value trace.".into());
        }
        let trace = session.property_trace(self.target, &self.property)?;
        let document = session.document();
        let source = session.authored_source();
        let map = session.rust_map();
        let span = |location: Option<&TraceLocation>| {
            location.and_then(|location| location.span(document, map))
        };
        let body = StackPanel::new();
        body.set_width(336.0);
        body.set_spacing(6.0);
        let heading = text(&format!(
            "{} · {}",
            trace.native.property, trace.native.effective
        ));
        heading.set_font_weight(FontWeight::SemiBold);
        heading.set_text_wrapping(TextWrapping::Wrap);
        body.children().add(heading.upcast::<Control>());
        if let Some(location) = &trace.authored {
            let display = location
                .text(document, source, map)
                .unwrap_or("Property element");
            self.add_row(
                &body,
                flyout,
                "Authored value",
                display,
                span(Some(location)),
            );
        }
        if let Some(reference) = span(trace.resource.as_ref()) {
            self.add_row(&body, flyout, "Referenced resource", "", Some(reference));
        }
        for (entry, location) in trace.native.contributors.iter().zip(&trace.sources) {
            let status = match entry.state_name() {
                "effective" => "Effective",
                "base" => "Animation base",
                "inactive" => "Inactive",
                "unset" => "Unset",
                _ => "Overridden",
            };
            let detail = entry
                .value
                .as_ref()
                .map_or_else(|| status.into(), |value| format!("{status} · {value}"));
            self.add_row(
                &body,
                flyout,
                &entry.label,
                &detail,
                span(location.as_ref()),
            );
        }
        let scroll = ScrollViewer::new();
        scroll.set_max_height(410.0);
        scroll.set_content(Some(Control::boxed(body)));
        Ok(scroll.upcast())
    }

    fn add_row(
        &self,
        body: &StackPanel,
        flyout: &Ref<Flyout>,
        label: &str,
        detail: &str,
        source: Option<SourceSpan>,
    ) {
        let row = Grid::new();
        row.set_column_definitions(ColumnDefinitions::parse("*,Auto").expect("trace columns"));
        let labels = StackPanel::new();
        let title = text(label);
        title.set_text_wrapping(TextWrapping::Wrap);
        labels.children().add(title.upcast::<Control>());
        if !detail.is_empty() {
            let value = text(detail);
            value.set_font_size(11.0);
            value.set_text_wrapping(TextWrapping::Wrap);
            labels.children().add(value.upcast::<Control>());
        }
        row.children().add(labels.upcast::<Control>());
        if let Some(source) = source {
            let context = self.clone();
            let popup = flyout.downgrade();
            let reveal = icon_button(SOURCE, &format!("Reveal {label} in source"), move || {
                context.reveal(&popup, source)
            });
            Grid::set_column(&reveal, 1);
            row.children().add(reveal.upcast::<Control>());
        }
        body.children().add(row.upcast::<Control>());
    }

    fn reveal(&self, flyout: &WeakRef<Flyout>, span: SourceSpan) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        if let Some(flyout) = flyout.upgrade() {
            flyout.hide();
        }
        if self.current(&workspace, &workspace.session.borrow()) {
            workspace.select_source_span(span);
        } else {
            workspace.report(Err(
                "The document or selection changed. Reopen the value trace.".into(),
            ));
        }
    }
}
