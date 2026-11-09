//! The life of a wall of tiles: building it, attaching it to a root built from its leaves up or
//! from its root down, laying it out the first time, detaching it, and clearing a panel of many
//! children.
//!
//! - `build`: the elements of the tiles, their properties set, put together from the leaves up
//!   (the wall is dropped after the measurement);
//! - `attach`: a built wall shown in the root (logical and visual attachment, styling, inherited
//!   values; section 9.21);
//! - `attach_top_down`: the tiles built into a wall the root already shows, each element added to
//!   an attached parent before its children are added to it;
//! - `lay_out`: the first layout pass of an attached wall (measure, arrange, text layouts);
//! - `detach`: an attached, laid out wall taken out of the root (section 9.18);
//! - `clear`: the children of a canvas of tiles removed at once.

use super::fixture::{tile, tile_parts, wall, Fixture, TILES};
use crate::harness::Registry;
use ferroui_base::Ref;
use ferroui_controls::{Canvas, Control};

pub struct Lifetime {
    fixture: Fixture,
    tiles: usize,
    pending: Option<Ref<Canvas>>,
}

impl Lifetime {
    fn new(tiles: usize) -> Self {
        Self { fixture: Fixture::new(), tiles, pending: None }
    }

    fn build(&self) -> Ref<Canvas> {
        wall(self.tiles, &self.fixture.brushes)
    }

    /// The root empty and a new wall waiting to be attached.
    fn prepare_detached(&mut self) {
        self.fixture.show(None);
        // The layout queue holds what was detached until its next pass: freed here, unmeasured.
        self.fixture.lay_out();
        self.pending = Some(self.build());
    }

    /// A new wall attached to the root.
    fn prepare_attached(&mut self) {
        self.prepare_detached();
        let canvas = self.pending.clone().expect("a wall").upcast::<Control>();
        self.fixture.show(Some(&canvas));
    }

    /// A new wall attached and laid out.
    fn prepare_laid_out(&mut self) {
        self.prepare_attached();
        self.fixture.lay_out();
    }

    /// An empty canvas attached to the root.
    fn prepare_empty(&mut self) {
        self.fixture.show(None);
        // The layout queue holds what was detached until its next pass: freed here, unmeasured.
        self.fixture.lay_out();
        let canvas = Canvas::new();
        self.fixture.show(Some(&canvas.clone().upcast::<Control>()));
        self.pending = Some(canvas);
    }

    fn attach(&mut self) {
        let canvas = self.pending.clone().expect("a wall").upcast::<Control>();
        self.fixture.show(Some(&canvas));
    }

    fn attach_top_down(&mut self) {
        let canvas = self.pending.clone().expect("a canvas");
        for index in 0..self.tiles {
            let parts = tile_parts(index, &self.fixture.brushes);
            canvas.children().add(&parts.tile);
            parts.tile.set_child(&parts.content);
            parts.content.children().add(&parts.name);
            parts.content.children().add(&parts.line);
            parts.line.children().add(&parts.spot);
            parts.line.children().add(&parts.value);
        }
    }

    fn clear(&mut self) {
        let canvas = self.pending.clone().expect("a wall");
        canvas.children().clear();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("trees", "Lifetime").min_iteration_time(200);
    let parameters = format!("Tiles={TILES}");
    class.benchmark("build", parameters.clone(), || Lifetime::new(TILES), |b| b.build()).baseline();
    class.benchmark_with_iteration_setup("attach", parameters.clone(), || Lifetime::new(TILES), Lifetime::prepare_detached, Lifetime::attach);
    class.benchmark_with_iteration_setup(
        "attach_top_down",
        parameters.clone(),
        || Lifetime::new(TILES),
        Lifetime::prepare_empty,
        Lifetime::attach_top_down,
    );
    class.benchmark_with_iteration_setup("lay_out", parameters.clone(), || Lifetime::new(TILES), Lifetime::prepare_attached, |b| {
        b.fixture.lay_out()
    });
    class.benchmark_with_iteration_setup("detach", parameters.clone(), || Lifetime::new(TILES), Lifetime::prepare_laid_out, |b| {
        b.fixture.show(None)
    });
    class.benchmark_with_iteration_setup(
        "clear",
        parameters,
        || Lifetime::new(TILES),
        |b: &mut Lifetime| {
            b.fixture.show(None);
            b.fixture.lay_out();
            let canvas = Canvas::new();
            for index in 0..b.tiles {
                canvas.children().add(tile(index, &b.fixture.brushes));
            }
            b.pending = Some(canvas);
        },
        Lifetime::clear,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Lifetime");
    }
}
