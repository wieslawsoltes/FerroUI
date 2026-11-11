//! Dynamic resources of a subtree that is attached to a tree: the panel that holds every tile of
//! the wall is taken out of the canvas and put back, and every text of it has a dynamic resource
//! of the root in its `Tag`, made as markup makes `Tag="{DynamicResource Accent}"`. A dynamic
//! resource started while its control is out of the tree finds nothing; when the panel is added
//! back the texts inherit the theme variant of the root, which looks the resource up, and the
//! attachment notifies them that the resources they see changed (docs/porting/desktop-performance.md,
//! section 9.44). `ResourceBindings` measures resource observers bound in an attached wall, and
//! `DetachReattach` the move of a section without resources.
//!
//! - `reattach`: the section added back to the canvas: every dynamic resource finds the resource
//!   of the root;
//! - `detach`: the section removed: every dynamic resource loses it;
//! - `reattach_with_inactive_setters`: `reattach`, with a style of the texts in the root whose
//!   selector does not match yet (`TextBlock:pointerover`) and whose four setters are dynamic
//!   resources: the frame of the style is attached to every text, the bindings of its setters
//!   are not instanced (section 9.43).

use super::wall::{tag, tag_value, Wall, CONTROLS};
use crate::harness::Registry;
use ferroui_base::data::BindingBase;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{BoxedValue, Ref, Visual};
use ferroui_controls::{Control, Panel, TextBlock};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::rc::Rc;

/// The key of the resource of the root.
const KEY: &str = "Accent";

/// `{DynamicResource key}`.
fn dynamic_resource(key: &str) -> Rc<DynamicResourceExtension> {
    let key: BoxedValue = Rc::new(key.to_string());
    DynamicResourceExtension::with_resource_key(Some(key))
}

pub struct DynamicResources {
    wall: Wall,
    /// The panel that holds the tiles.
    section: Ref<Panel>,
    attached: bool,
}

impl DynamicResources {
    fn new(inactive_setters: bool) -> Self {
        let wall = Wall::with_sections(CONTROLS, 1);
        let section = wall.sections[0].clone();
        let root = &wall.fixture.root;
        root.resources().add(KEY, tag(1));
        if inactive_setters {
            let properties = [
                Control::tag_property().as_property(),
                TextBlock::foreground_property().as_property(),
                TextBlock::background_property().as_property(),
                Visual::opacity_property().as_property(),
            ];
            let setters = properties.map(|property| {
                let binding: Rc<dyn BindingBase> = dynamic_resource(property.name());
                Setter::new_binding_base(property, binding)
            });
            root.styles().add(&Style::with_setters(Selectors::of_type::<TextBlock>().class(":pointerover"), setters));
        }
        let binding = dynamic_resource(KEY);
        for tile in &wall.tiles {
            let _ = tile.text.bind_binding(Control::tag_property().as_property(), &*binding);
        }
        wall.attach();
        let mut this = Self { wall, section, attached: true };
        this.expect(CONTROLS, "after the attach");
        this.detach();
        this.expect(0, "out of the tree");
        this.reattach();
        this.expect(CONTROLS, "after the section is added again");
        this.wall.fixture.lay_out();
        this
    }

    /// Checks that `texts` texts have the resource of the root.
    fn expect(&self, texts: usize, when: &str) {
        let found = self.wall.tiles.iter().filter(|tile| tag_value(&tile.text) == Some(1)).count();
        if found != texts {
            panic!("Invalid benchmark state: {found} texts have the resource {when}, {texts} are expected");
        }
    }

    /// Removes the section from the canvas.
    fn detach(&mut self) {
        if self.attached {
            self.wall.canvas.children().remove(&self.section);
            self.attached = false;
        }
    }

    /// Adds the detached section to the canvas.
    fn reattach(&mut self) {
        if !self.attached {
            self.wall.canvas.children().add(&self.section);
            self.attached = true;
        }
    }

    /// The state before a measured removal: the section is attached and laid out.
    fn prepare_detach(&mut self) {
        self.reattach();
        self.wall.fixture.lay_out();
    }

    /// The state before a measured addition: the section is detached.
    fn prepare_reattach(&mut self) {
        self.detach();
        self.wall.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "DynamicResources").min_iteration_time(200);
    let parameters = format!("Controls={CONTROLS}");
    class
        .benchmark_with_iteration_setup(
            "reattach",
            parameters.clone(),
            || DynamicResources::new(false),
            DynamicResources::prepare_reattach,
            DynamicResources::reattach,
        )
        .baseline();
    class.benchmark_with_iteration_setup(
        "detach",
        parameters.clone(),
        || DynamicResources::new(false),
        DynamicResources::prepare_detach,
        DynamicResources::detach,
    );
    class.benchmark_with_iteration_setup(
        "reattach_with_inactive_setters",
        parameters,
        || DynamicResources::new(true),
        DynamicResources::prepare_reattach,
        DynamicResources::reattach,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "DynamicResources");
    }
}
