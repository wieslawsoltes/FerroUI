//! The untyped paths to the properties of a control: what markup, bindings, styles, resources and
//! the C ABI reach a property through, the routes of the property (#997 shares them between the
//! value types). Each path is measured for five kinds of value type:
//!
//! - `Number`: `Opacity`, an `f64`;
//! - `Enumeration`: `HorizontalAlignment`;
//! - `Handle`: `Clip`, a handle to a geometry;
//! - `Brush`: `Background`, a trait object;
//! - `Structure`: `Margin`, a thickness.
//!
//! The paths:
//!
//! - `set_untyped`: `set_value_untyped` with a boxed value of the property's type, one of two in
//!   turn;
//! - `get_untyped`: `get_value_untyped`, the value in a box of its own;
//! - `bind_untyped`: a local binding of boxed values (`bind_property_untyped`) told the other
//!   value;
//! - `style_setter`: a style of the root whose setter holds a boxed value, as markup builds one,
//!   matched and unmatched in turn by a class of every border;
//! - `theme_switch` (brushes only): the background of every border a dynamic resource whose value
//!   differs between the light and the dark dictionary of the root; the variant of the scope of
//!   the borders changed.
//!
//! The borders of `set_untyped`, `get_untyped` and `bind_untyped` are in no tree, so the cost is
//! the property system and the class handlers; those of `style_setter` and `theme_switch` are the
//! children of a panel that is shown. Each invocation visits every border once.

use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::{BindingBase, BindingPriority};
use ferroui_base::layout::{HorizontalAlignment, Layoutable};
use ferroui_base::media::{Geometry, IBrush, RectangleGeometry};
use ferroui_base::reactive::{IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::{Selectors, Setter, Style, ThemeVariant};
use ferroui_base::{BoxedValue, FerroProperty, Rect, Ref, Thickness, Visual};
use ferroui_controls::{Border, Control, StackPanel, ThemeVariantScope};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::rc::Rc;

const OBJECTS: usize = 1_000;

/// The key of the brush of the theme dictionaries.
const KEY: &str = "Surface";

/// The class that matches the style of `style_setter`.
const CLASS: &str = "styled";

/// A kind of value type: the property of a border and the two values the paths give it.
#[derive(Clone, Copy)]
enum Kind {
    Number,
    Enumeration,
    Handle,
    Brush,
    Structure,
}

impl Kind {
    const ALL: [Kind; 5] = [Kind::Number, Kind::Enumeration, Kind::Handle, Kind::Brush, Kind::Structure];

    fn name(self) -> &'static str {
        match self {
            Kind::Number => "Number",
            Kind::Enumeration => "Enumeration",
            Kind::Handle => "Handle",
            Kind::Brush => "Brush",
            Kind::Structure => "Structure",
        }
    }

    fn property(self) -> &'static FerroProperty {
        match self {
            Kind::Number => Visual::opacity_property().as_property(),
            Kind::Enumeration => Layoutable::horizontal_alignment_property().as_property(),
            Kind::Handle => Visual::clip_property().as_property(),
            Kind::Brush => Border::background_property().as_property(),
            Kind::Structure => Layoutable::margin_property().as_property(),
        }
    }

    /// The two values of the property, boxed as markup and bindings box them.
    fn values(self, fixture: &Fixture) -> [BoxedValue; 2] {
        match self {
            Kind::Number => [Rc::new(0.25), Rc::new(0.75)],
            Kind::Enumeration => [Rc::new(HorizontalAlignment::Left), Rc::new(HorizontalAlignment::Right)],
            Kind::Handle => {
                let clip = |width| -> Option<Ref<Geometry>> {
                    Some(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, width, 4.0)).upcast())
                };
                [Rc::new(clip(8.0)), Rc::new(clip(16.0))]
            }
            Kind::Brush => [Rc::new(Some(fixture.brushes.tile.clone())), Rc::new(Some(fixture.brushes.other.clone()))],
            Kind::Structure => [Rc::new(Thickness::uniform(1.0)), Rc::new(Thickness::uniform(2.0))],
        }
    }
}

pub struct UntypedRoutes {
    fixture: Fixture,
    borders: Vec<Ref<Border>>,
    property: &'static FerroProperty,
    values: [BoxedValue; 2],
    /// The sources of the local bindings of `bind_untyped`, one a border.
    sources: Vec<Rc<LightweightSubject<BoxedValue>>>,
    /// The scope of the borders of `theme_switch`.
    scope: Option<Ref<ThemeVariantScope>>,
    /// Which of the two values (or states) the next invocation gives.
    flip: bool,
}

impl UntypedRoutes {
    /// Borders in no tree, each holding the first value of the property.
    fn loose(kind: Kind) -> Self {
        let fixture = Fixture::new();
        let values = kind.values(&fixture);
        let property = kind.property();
        let borders: Vec<Ref<Border>> = (0..OBJECTS).map(|_| Border::new()).collect();
        for border in &borders {
            let _ = border.set_value_untyped(property, values[0].as_any(), BindingPriority::LocalValue);
        }
        Self { fixture, borders, property, values, sources: Vec::new(), scope: None, flip: false }
    }

    /// Borders in no tree, the property of each bound to a source of boxed values that has given
    /// the first value.
    fn bound(kind: Kind) -> Self {
        let mut this = Self::loose(kind);
        for border in &this.borders {
            border.clear_value_untyped(this.property);
            let source = Rc::new(LightweightSubject::new());
            let observable: Rc<dyn IObservable<BoxedValue>> = source.clone();
            let _ = border.bind_property_untyped(this.property, observable, BindingPriority::LocalValue);
            source.on_next(this.values[0].clone());
            this.sources.push(source);
        }
        this
    }

    /// Borders shown in a panel, with a style of the root that sets the property to the second
    /// value on a border of the class.
    fn styled(kind: Kind) -> Self {
        let mut this = Self::loose(kind);
        for border in &this.borders {
            border.clear_value_untyped(this.property);
        }
        let setter = Setter::new_untyped(this.property, this.values[1].clone());
        this.fixture.root.styles().add(&Style::with_setters(Selectors::of_type::<Border>().class(CLASS), [setter]));
        let panel = StackPanel::new();
        for border in &this.borders {
            panel.children().add(border);
        }
        this.fixture.show(Some(&panel.upcast::<Control>()));
        this.fixture.lay_out();
        this
    }

    /// Borders shown in a theme variant scope, the background of each the brush of the theme
    /// dictionary of the variant.
    fn themed() -> Self {
        let mut this = Self::loose(Kind::Brush);
        let dictionary = |brush: BoxedValue| {
            let dictionary = ResourceDictionary::new();
            dictionary.add(KEY, Some(brush));
            dictionary
        };
        let resources = this.fixture.root.resources();
        resources.add_theme_dictionary(ThemeVariant::light(), dictionary(Rc::new(this.fixture.brushes.tile.clone())));
        resources.add_theme_dictionary(ThemeVariant::dark(), dictionary(Rc::new(this.fixture.brushes.other.clone())));
        let key: BoxedValue = Rc::new(KEY.to_string());
        let binding: Rc<dyn BindingBase> = DynamicResourceExtension::with_resource_key(Some(key));
        let panel = StackPanel::new();
        for border in &this.borders {
            border.clear_value_untyped(this.property);
            let _ = border.bind_binding(this.property, &*binding);
            panel.children().add(border);
        }
        let scope = ThemeVariantScope::new();
        scope.set_requested_theme_variant(Some(ThemeVariant::light()));
        scope.set_child(&panel);
        this.fixture.show(Some(&scope.clone().upcast::<Control>()));
        this.fixture.lay_out();
        this.scope = Some(scope);
        this.expect_brush(&this.fixture.brushes.tile.clone(), "in the light variant");
        this.theme_switch();
        this.expect_brush(&this.fixture.brushes.other.clone(), "in the dark variant");
        this.theme_switch();
        this
    }

    fn expect_brush(&self, brush: &Rc<dyn IBrush>, when: &str) {
        let found = self
            .borders
            .iter()
            .filter(|border| border.background().is_some_and(|background| Rc::ptr_eq(&background, brush)))
            .count();
        if found != OBJECTS {
            panic!("Invalid benchmark state: {found} borders have the brush {when}, {OBJECTS} are expected");
        }
    }

    /// The other of the two values.
    fn next(&mut self) -> &BoxedValue {
        self.flip = !self.flip;
        &self.values[usize::from(self.flip)]
    }

    fn set_untyped(&mut self) {
        let property = self.property;
        let value = self.next().clone();
        for border in &self.borders {
            let _ = border.set_value_untyped(property, value.as_any(), BindingPriority::LocalValue);
        }
    }

    fn get_untyped(&mut self) {
        for border in &self.borders {
            std::hint::black_box(border.get_value_untyped(self.property));
        }
    }

    fn bind_untyped(&mut self) {
        let value = self.next().clone();
        for source in &self.sources {
            source.on_next(value.clone());
        }
    }

    fn style_setter(&mut self) {
        self.flip = !self.flip;
        for border in &self.borders {
            border.classes().set(CLASS, self.flip);
        }
        self.fixture.lay_out();
    }

    fn theme_switch(&mut self) {
        self.flip = !self.flip;
        let variant = if self.flip { ThemeVariant::dark() } else { ThemeVariant::light() };
        if let Some(scope) = &self.scope {
            scope.set_requested_theme_variant(Some(variant));
        }
    }
}

pub fn register(registry: &mut Registry) {
    type Setup = fn(Kind) -> UntypedRoutes;
    type Body = fn(&mut UntypedRoutes);
    const PATHS: [(&str, Setup, Body); 4] = [
        ("set_untyped", UntypedRoutes::loose, UntypedRoutes::set_untyped),
        ("get_untyped", UntypedRoutes::loose, UntypedRoutes::get_untyped),
        ("bind_untyped", UntypedRoutes::bound, UntypedRoutes::bind_untyped),
        ("style_setter", UntypedRoutes::styled, UntypedRoutes::style_setter),
    ];
    let mut class = registry.class("property_subsystem", "UntypedRoutes").min_iteration_time(200);
    for (name, setup, body) in PATHS {
        for kind in Kind::ALL {
            let parameters = format!("Kind={}, Objects={OBJECTS}", kind.name());
            class.benchmark(name, parameters, move || setup(kind), body).operations_per_invoke(OBJECTS as u32);
        }
    }
    let parameters = format!("Kind=Brush, Objects={OBJECTS}");
    class
        .benchmark("theme_switch", parameters, UntypedRoutes::themed, UntypedRoutes::theme_switch)
        .operations_per_invoke(OBJECTS as u32);
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "UntypedRoutes");
    }
}
