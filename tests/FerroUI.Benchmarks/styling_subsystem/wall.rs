//! The wall of the styling benchmarks: a canvas of tiles, each a chain of four elements of four
//! types (a border, a stack panel, a panel and a text), so a selector can name the parent of a
//! text, an ancestor two levels up, or the canvas four levels up. The texts are the controls the
//! benchmarks style. The styles set their `Tag`, a property nothing reads: a style applied or
//! switched costs what the styling system does, and no layout or drawing after it.

use crate::trees::fixture::Fixture;
use ferroui_base::styling::Setter;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Border, Canvas, Control, Panel, StackPanel, TextBlock, ThemeVariantScope};
use std::rc::Rc;

/// The styled controls of a benchmark, by default.
pub const CONTROLS: usize = 1_000;

/// A value of the `Tag` property.
pub fn tag(value: i32) -> Option<BoxedValue> {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

/// The setter of the styles of the benchmarks: a plain value for the `Tag` of a control.
pub fn tag_setter() -> Rc<Setter> {
    Setter::new(Control::tag_property(), tag(1))
}

/// The number in the `Tag` of a control, when it holds one.
pub fn tag_value(control: &Control) -> Option<i32> {
    control.tag().as_ref().and_then(|value| value.downcast_ref::<i32>()).copied()
}

/// The elements of a tile, from the outside in.
pub struct Tile {
    pub tile: Ref<Border>,
    pub content: Ref<StackPanel>,
    pub line: Ref<Panel>,
    pub text: Ref<TextBlock>,
}

impl Tile {
    fn new(index: usize) -> Self {
        let (width, height) = (5.0, 4.0);
        let text = TextBlock::new();
        let line = Panel::new();
        line.children().add(&text);
        let content = StackPanel::new();
        content.children().add(&line);
        let tile = Border::new();
        tile.set_width(width);
        tile.set_height(height);
        tile.set_child(&content);
        Canvas::set_left(&tile, (index % 200) as f64 * (width + 1.0));
        Canvas::set_top(&tile, (index / 200) as f64 * (height + 1.0));
        Self { tile, content, line, text }
    }
}

/// An application, a root and a wall of tiles. The wall is built detached: a benchmark adds its
/// styles and classes, then attaches it.
pub struct Wall {
    pub fixture: Fixture,
    /// Borders around the canvas, from the inside out: style hosts between the root and the wall.
    pub wrappers: Vec<Ref<Border>>,
    pub canvas: Ref<Canvas>,
    /// Panels of the canvas that hold the tiles in equal parts; without them the canvas holds the
    /// tiles.
    pub sections: Vec<Ref<Panel>>,
    pub tiles: Vec<Tile>,
    top: Ref<Control>,
}

impl Wall {
    /// A canvas of `count` tiles.
    pub fn new(count: usize) -> Self {
        Self::build(count, 0, 0)
    }

    /// A canvas of `count` tiles inside `levels` nested borders.
    pub fn with_wrappers(count: usize, levels: usize) -> Self {
        Self::build(count, levels, 0)
    }

    /// A canvas of `sections` panels that share `count` tiles.
    pub fn with_sections(count: usize, sections: usize) -> Self {
        Self::build(count, 0, sections)
    }

    fn build(count: usize, levels: usize, sections: usize) -> Self {
        let fixture = Fixture::new();
        let canvas = Canvas::new();
        let sections: Vec<Ref<Panel>> = (0..sections)
            .map(|_| {
                let section = Panel::new();
                canvas.children().add(&section);
                section
            })
            .collect();
        let mut tiles = Vec::with_capacity(count);
        for index in 0..count {
            let tile = Tile::new(index);
            match sections.len() {
                0 => canvas.children().add(&tile.tile),
                len => sections[index * len / count].children().add(&tile.tile),
            }
            tiles.push(tile);
        }
        let mut top = canvas.clone().upcast::<Control>();
        let mut wrappers = Vec::with_capacity(levels);
        for _ in 0..levels {
            let wrapper = Border::new();
            wrapper.set_child(&top);
            top = wrapper.clone().upcast::<Control>();
            wrappers.push(wrapper);
        }
        Self { fixture, wrappers, canvas, sections, tiles, top }
    }

    /// Shows the wall in the root and lays it out: every element is attached and styled.
    pub fn attach(&self) {
        self.fixture.show(Some(&self.top));
        self.fixture.lay_out();
    }

    /// Shows the wall in the root below `scope` and lays it out.
    pub fn attach_in(&self, scope: &Ref<ThemeVariantScope>) {
        scope.set_child(&self.top);
        self.fixture.show(Some(&scope.clone().upcast::<Control>()));
        self.fixture.lay_out();
    }

    /// Takes the styles off the texts; their next styling pass applies them again.
    pub fn invalidate_texts(&self) {
        for tile in &self.tiles {
            tile.text.invalidate_styles(false);
        }
    }

    /// A styling pass of every text: the styles in scope are evaluated and those that match are
    /// applied. A text that is styled already does nothing.
    pub fn style_texts(&self) {
        for tile in &self.tiles {
            tile.text.apply_styling();
        }
    }

    /// A styling pass of every element from the root down, parents first.
    pub fn style_all(&self) {
        self.fixture.root.apply_styling();
        for wrapper in self.wrappers.iter().rev() {
            wrapper.apply_styling();
        }
        self.canvas.apply_styling();
        for section in &self.sections {
            section.apply_styling();
        }
        for tile in &self.tiles {
            tile.tile.apply_styling();
            tile.content.apply_styling();
            tile.line.apply_styling();
            tile.text.apply_styling();
        }
    }

    /// The texts whose `Tag` has a value.
    pub fn tagged_texts(&self) -> usize {
        self.tiles.iter().filter(|tile| tile.text.tag().is_some()).count()
    }

    /// The texts whose `Tag` holds `value`.
    pub fn texts_with_tag(&self, value: i32) -> usize {
        self.tiles.iter().filter(|tile| tag_value(&tile.text) == Some(value)).count()
    }

    /// Before a measured styling pass of the texts: what the last pass queued is laid out, then
    /// the styles are taken off the texts.
    pub fn prepare_restyle(&mut self) {
        self.fixture.lay_out();
        self.invalidate_texts();
    }

    /// Runs what a restyle benchmark prepares and measures once, outside every measurement, and
    /// checks each step: the attached texts are styled, the preparation takes their styles off,
    /// and the pass puts them back. `tagged` says whether an applied style sets the `Tag` of
    /// every text (a style whose activator is off, or no style at all, sets none).
    pub fn check_restyle(&mut self, tagged: bool) {
        let expected = if tagged { self.tiles.len() } else { 0 };
        if self.tagged_texts() != expected {
            panic!("Invalid benchmark state: {} of {} texts are styled after the attach", self.tagged_texts(), self.tiles.len());
        }
        self.prepare_restyle();
        if self.tagged_texts() != 0 {
            panic!("Invalid benchmark state: {} texts keep a style that was taken off", self.tagged_texts());
        }
        self.style_texts();
        if self.tagged_texts() != expected {
            panic!("Invalid benchmark state: {} of {} texts are styled after the pass", self.tagged_texts(), self.tiles.len());
        }
    }
}
