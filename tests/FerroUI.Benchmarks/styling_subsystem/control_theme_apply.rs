//! Control themes of templated controls: what a button, a check box and a text box cost on top of
//! the plain elements of the other classes. A control finds its theme by its type among the
//! resources in scope, the theme's setters give it a template, the template builds its parts,
//! and each part is styled in turn by the theme's own styles. The benchmarks split that chain:
//!
//! - `attach`: new controls shown in the root. Each finds its control theme (section 9.21: the
//!   application keeps the theme per type and theme variant), applies the theme's setters and
//!   the styles in scope; no template is built yet;
//! - `apply_template`: the template of each attached, styled control built: the parts are
//!   created, attached below the control and styled. Templated parts keep their own templates
//!   for their first measure, so one level is built;
//! - `reattach`: controls that were laid out, so their templates are built, taken out of the root
//!   and shown again: the controls and their parts are styled again, the theme is found the same
//!   and no template is built;
//! - `switch_theme_variant`: the theme variant of a scope above laid out buttons switched between
//!   light and dark: the values the theme takes from the resources of the variant are read again
//!   (compare `trees::Propagation.theme_variant`, whose wall has no control themes).

use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::Ref;
use ferroui_controls::{Button, Canvas, CheckBox, Control, TextBox, ThemeVariantScope};
use std::cell::Cell;

/// The controls of a benchmark.
const CONTROLS: usize = 200;

type Make = fn() -> Ref<Control>;

const KINDS: [(&str, Make); 3] = [
    ("Button", || Button::new().upcast::<Control>()),
    ("CheckBox", || CheckBox::new().upcast::<Control>()),
    ("TextBox", || TextBox::new().upcast::<Control>()),
];

pub struct ControlThemeApply {
    fixture: Fixture,
    make: Make,
    /// The scope above the controls, when the benchmark switches the theme variant.
    scope: Option<Ref<ThemeVariantScope>>,
    /// What the root shows: the canvas of the controls, or the scope around it.
    top: Option<Ref<Control>>,
    controls: Vec<Ref<Control>>,
    flip: Cell<bool>,
}

impl ControlThemeApply {
    fn new(make: Make) -> Self {
        Self { fixture: Fixture::new(), make, scope: None, top: None, controls: Vec::new(), flip: Cell::new(false) }
    }

    /// New controls shown in the root: themed and styled, without their templates.
    fn attached(make: Make) -> Self {
        let mut this = Self::new(make);
        this.build_attached();
        if this.controls.iter().any(|control| control.get_effective_theme().is_none()) {
            panic!("Invalid benchmark state: an attached control has no control theme");
        }
        if this.templates() != 0 {
            panic!("Invalid benchmark state: {} controls have a template before their first measure", this.templates());
        }
        this
    }

    /// New controls shown in the root whose templates were built once, outside a layout pass.
    fn templated(make: Make) -> Self {
        let mut this = Self::attached(make);
        this.apply_templates();
        this.expect_templates();
        this
    }

    /// Controls shown in the root and laid out: themed, styled, with their templates.
    fn laid_out(make: Make) -> Self {
        let mut this = Self::attached(make);
        this.fixture.lay_out();
        this.expect_templates();
        this.detach();
        this.attach();
        this.fixture.lay_out();
        this.expect_templates();
        this
    }

    /// Laid out controls below a scope of the light theme variant.
    fn in_scope(make: Make) -> Self {
        let mut this = Self::new(make);
        this.build();
        let scope = ThemeVariantScope::new();
        scope.set_requested_theme_variant(Some(ThemeVariant::light()));
        scope.set_child(this.top.as_ref().expect("the controls"));
        this.top = Some(scope.clone().upcast::<Control>());
        this.scope = Some(scope);
        this.attach();
        this.fixture.lay_out();
        this.expect_templates();
        this.switch_theme_variant();
        if this.controls.iter().any(|control| control.actual_theme_variant() != Some(ThemeVariant::dark())) {
            panic!("Invalid benchmark state: a control does not follow the theme variant of the scope");
        }
        this.fixture.lay_out();
        this.switch_theme_variant();
        this
    }

    /// The controls whose template built a part.
    fn templates(&self) -> usize {
        self.controls.iter().filter(|control| control.visual_children_count() > 0).count()
    }

    fn expect_templates(&self) {
        if self.templates() != self.controls.len() {
            panic!("Invalid benchmark state: {} of {} controls have a template", self.templates(), self.controls.len());
        }
    }

    fn detach(&mut self) {
        self.fixture.show(None);
        // The layout queue holds what was detached until its next pass: freed here, unmeasured.
        self.fixture.lay_out();
    }

    /// Empties the root and builds a canvas of new controls, not shown.
    fn build(&mut self) {
        self.detach();
        let canvas = Canvas::new();
        self.controls.clear();
        for index in 0..CONTROLS {
            let control = (self.make)();
            Canvas::set_left(&control, (index % 20) as f64 * 64.0);
            Canvas::set_top(&control, (index / 20) as f64 * 40.0);
            canvas.children().add(&control);
            self.controls.push(control);
        }
        self.top = Some(canvas.upcast::<Control>());
    }

    fn attach(&mut self) {
        let top = self.top.clone().expect("the controls");
        self.fixture.show(Some(&top));
    }

    fn build_attached(&mut self) {
        self.build();
        self.attach();
    }

    fn apply_templates(&mut self) {
        for control in &self.controls {
            control.apply_template();
        }
    }

    fn settle(&mut self) {
        self.fixture.lay_out();
    }

    fn switch_theme_variant(&mut self) {
        let dark = !self.flip.get();
        self.flip.set(dark);
        let variant = if dark { ThemeVariant::dark() } else { ThemeVariant::light() };
        self.scope.as_ref().expect("a scope").set_requested_theme_variant(Some(variant));
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "ControlThemeApply").min_iteration_time(200);
    for (name, make) in KINDS {
        let parameters = format!("Control={name}, Controls={CONTROLS}");
        class.benchmark_with_iteration_setup(
            "attach",
            parameters.clone(),
            move || ControlThemeApply::attached(make),
            ControlThemeApply::build,
            ControlThemeApply::attach,
        );
        class.benchmark_with_iteration_setup(
            "apply_template",
            parameters.clone(),
            move || ControlThemeApply::templated(make),
            ControlThemeApply::build_attached,
            ControlThemeApply::apply_templates,
        );
        class.benchmark_with_iteration_setup(
            "reattach",
            parameters,
            move || ControlThemeApply::laid_out(make),
            ControlThemeApply::detach,
            ControlThemeApply::attach,
        );
    }
    let (name, make) = KINDS[0];
    class.benchmark_with_iteration_setup(
        "switch_theme_variant",
        format!("Control={name}, Controls={CONTROLS}"),
        move || ControlThemeApply::in_scope(make),
        ControlThemeApply::settle,
        ControlThemeApply::switch_theme_variant,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ControlThemeApply");
    }
}
