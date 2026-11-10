//! Resources found from, and observed by, controls of an attached tree. A lookup starts at the
//! control and asks each host above it, then the application; a host answers from its own
//! entries, then its theme dictionaries, then its merged dictionaries, the last added first.
//! An observer of a resource (what a dynamic resource reference in markup makes) looks the key
//! up when it is bound and again whenever the resources or the theme variant its control sees
//! change. `styling::ResourceBenchmarks` finds resources of the global styles from a chain
//! without dictionaries, and `trees::Styling.resource_changed` replaces a resource nothing
//! observes.
//!
//! - `find`: a resource of the root found from a text below a chain of borders, half of which
//!   hold a dictionary of 16 other entries: the walk up the hosts, by its depth;
//! - `find_merged`: the resource is in the first of 16 merged dictionaries of the root, the one
//!   asked last;
//! - `find_theme_variant`: the resource is in the theme dictionary of the root for the light
//!   variant and the lookup names that variant;
//! - `bind`: every text of a wall bound to a resource of the root: an observer made, subscribed
//!   to its control and the first value looked up and set;
//! - `replace_observed`: the resource replaced while every text observes it: every element is
//!   told, every observer looks the key up again and sets the new value;
//! - `switch_theme_variant`: the theme variant of a scope above the wall switched while every
//!   text observes a resource of the scope's theme dictionaries.
//!
//! The lookups run 100 times for each measured operation and the result is the time of one.

use super::wall::{tag, Wall, CONTROLS};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::controls::{ResourceDictionary, ResourceHostRef, ResourceKey};
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Border, Control, TextBlock, ThemeVariantScope};
use std::cell::Cell;
use std::rc::Rc;

/// The key of the resource the benchmarks find and observe.
const KEY: &str = "Accent";
/// The lookups of one measured operation.
const LOOKUPS: u32 = 100;
/// The entries of a dictionary that does not hold the key.
const ENTRIES: usize = 16;

/// The number a resource holds, when it holds one.
fn number(value: &Option<BoxedValue>) -> Option<i32> {
    value.as_ref().and_then(|value| value.downcast_ref::<i32>()).copied()
}

/// Adds the entries of a dictionary that does not hold the key.
fn fill(dictionary: &ResourceDictionary, name: &str) {
    for index in 0..ENTRIES {
        dictionary.add(format!("{name}{index}"), tag(0));
    }
}

/// A dictionary that does not hold the key.
fn dictionary(name: &str) -> Ref<ResourceDictionary> {
    let entries = ResourceDictionary::new();
    fill(&entries, name);
    entries
}

/// A dictionary of other entries and the key, with `value`.
fn dictionary_with_key(name: &str, value: i32) -> Ref<ResourceDictionary> {
    let entries = dictionary(name);
    entries.add(KEY, tag(value));
    entries
}

/// Where the root keeps the resource.
enum Place {
    Entries,
    /// The first of this many merged dictionaries.
    Merged(usize),
    ThemeDictionary,
}

pub struct ResourceLookup {
    _fixture: Fixture,
    leaf: Ref<TextBlock>,
    key: ResourceKey,
    /// The theme variant the lookup names.
    theme: Option<ThemeVariant>,
}

impl ResourceLookup {
    /// A text below `depth` borders in the root, whose resources hold the key at `place`.
    fn new(depth: usize, place: Place) -> Self {
        let fixture = Fixture::new();
        let leaf = TextBlock::new();
        let mut top = leaf.clone().upcast::<Control>();
        for level in 0..depth {
            let wrapper = Border::new();
            if level % 2 == 1 {
                fill(&wrapper.resources(), &format!("Level{level}Entry"));
            }
            wrapper.set_child(&top);
            top = wrapper.upcast::<Control>();
        }
        let resources = fixture.root.resources();
        let mut theme = None;
        match place {
            Place::Entries => resources.add(KEY, tag(1)),
            Place::Merged(count) => {
                for index in 0..count {
                    let merged = dictionary(&format!("Merged{index}Entry"));
                    if index == 0 {
                        merged.add(KEY, tag(1));
                    }
                    resources.add_merged_dictionary(&merged);
                }
            }
            Place::ThemeDictionary => {
                resources.set_theme_dictionary(ThemeVariant::light(), &dictionary_with_key("LightEntry", 1));
                resources.set_theme_dictionary(ThemeVariant::dark(), &dictionary_with_key("DarkEntry", 2));
                theme = Some(ThemeVariant::light());
            }
        }
        fixture.show(Some(&top));
        fixture.lay_out();
        let this = Self { _fixture: fixture, leaf, key: ResourceKey::from(KEY), theme };
        let found = this.leaf.try_find_resource(&this.key, this.theme.as_ref()).flatten();
        if number(&found) != Some(1) {
            panic!("Invalid benchmark state: the resource is not found from the text");
        }
        this
    }

    fn find(&self) -> usize {
        let mut found = 0;
        for _ in 0..LOOKUPS {
            let value = self.leaf.try_find_resource(&self.key, self.theme.as_ref());
            found += usize::from(std::hint::black_box(value).is_some());
        }
        found
    }
}

/// A wall whose texts observe a resource above them and keep its value in their `Tag`.
pub struct ResourceBindings {
    wall: Wall,
    /// The scope above the wall, when the resource comes from its theme dictionaries.
    scope: Option<Ref<ThemeVariantScope>>,
    bindings: Vec<Rc<dyn IDisposable>>,
    flip: Cell<bool>,
}

impl ResourceBindings {
    /// An attached wall. The resource is an entry of the root, or with `in_scope` an entry of
    /// the light and of the dark theme dictionary of a scope between the root and the wall.
    fn new(in_scope: bool) -> Self {
        let wall = Wall::new(CONTROLS);
        let scope = if in_scope {
            let scope = ThemeVariantScope::new();
            scope.set_requested_theme_variant(Some(ThemeVariant::light()));
            let resources = scope.resources();
            resources.set_theme_dictionary(ThemeVariant::light(), &dictionary_with_key("LightEntry", 1));
            resources.set_theme_dictionary(ThemeVariant::dark(), &dictionary_with_key("DarkEntry", 2));
            wall.attach_in(&scope);
            Some(scope)
        } else {
            wall.fixture.root.resources().add(KEY, tag(1));
            wall.attach();
            None
        };
        let mut this = Self { wall, scope, bindings: Vec::new(), flip: Cell::new(false) };
        this.bind();
        this.expect(1, "after they are bound");
        this.unbind();
        this
    }

    /// An attached wall whose texts observe the resource.
    fn bound(in_scope: bool) -> Self {
        let mut this = Self::new(in_scope);
        this.bind();
        this.change();
        this.expect(2, "after the resource changed");
        this.wall.fixture.lay_out();
        this.change();
        this.expect(1, "after the resource changed back");
        this.wall.fixture.lay_out();
        this
    }

    fn expect(&self, value: i32, when: &str) {
        let found = self.wall.texts_with_tag(value);
        if found != CONTROLS {
            panic!("Invalid benchmark state: {found} of {CONTROLS} texts have the resource {value} {when}");
        }
    }

    fn bind(&mut self) {
        for tile in &self.wall.tiles {
            let observable = ResourceHostRef::from(&tile.text).resource_observable(KEY, None);
            self.bindings.push(tile.text.bind(Control::tag_property(), observable, BindingPriority::LocalValue));
        }
    }

    fn unbind(&mut self) {
        for binding in self.bindings.drain(..) {
            binding.dispose();
        }
        self.wall.fixture.lay_out();
    }

    /// Changes what the texts observe: the theme variant of the scope, or without a scope the
    /// resource of the root.
    fn change(&mut self) {
        let other = !self.flip.get();
        self.flip.set(other);
        match &self.scope {
            Some(scope) => {
                let variant = if other { ThemeVariant::dark() } else { ThemeVariant::light() };
                scope.set_requested_theme_variant(Some(variant));
            }
            None => self.wall.fixture.root.resources().set(KEY, tag(if other { 2 } else { 1 })),
        }
    }

    fn settle(&mut self) {
        self.wall.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "ResourceLookup").min_iteration_time(200);
    for depth in [8, 32] {
        class
            .benchmark("find", format!("Depth={depth}"), move || ResourceLookup::new(depth, Place::Entries), |b| b.find())
            .operations_per_invoke(LOOKUPS);
    }
    class
        .benchmark("find_merged", "Depth=8, Merged=16", || ResourceLookup::new(8, Place::Merged(16)), |b| b.find())
        .operations_per_invoke(LOOKUPS);
    class
        .benchmark("find_theme_variant", "Depth=8", || ResourceLookup::new(8, Place::ThemeDictionary), |b| b.find())
        .operations_per_invoke(LOOKUPS);
    let parameters = format!("Controls={CONTROLS}");
    class.benchmark_with_iteration_setup(
        "bind",
        parameters.clone(),
        || ResourceBindings::new(false),
        ResourceBindings::unbind,
        ResourceBindings::bind,
    );
    class.benchmark_with_iteration_setup(
        "replace_observed",
        parameters.clone(),
        || ResourceBindings::bound(false),
        ResourceBindings::settle,
        ResourceBindings::change,
    );
    class.benchmark_with_iteration_setup(
        "switch_theme_variant",
        parameters,
        || ResourceBindings::bound(true),
        ResourceBindings::settle,
        ResourceBindings::change,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ResourceLookup");
    }
}
