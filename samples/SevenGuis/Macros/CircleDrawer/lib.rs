//! Circle drawer: a click on the canvas adds a circle, the circle nearest to
//! the pointer is filled, a right click on it opens a window with a slider of
//! its diameter, and undo and redo go through the changes.
//!
//! Written with the `rsx!` macro of `ferroui-view`; `cargo run -p seven-guis-circle-drawer-macros`.

use ferroui_base::input::{PointerEventArgs, PointerPressedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_controls::{Canvas, Dock};
use ferroui_view::prelude::*;
use seven_guis_model::state::Drawer;
use seven_guis_model::{circle_at, Circle, History};

/// The circle drawer, over its state.
#[component]
pub fn CircleDrawer(#[prop(default = Drawer::with_example())] drawer: Drawer) -> Element {
    let canvas_ref = NodeRef::<Canvas>::new();
    let shown = memo(move |_| drawer.circles());

    let pressed = move |e: &PointerPressedEventArgs| {
        let Some(canvas) = canvas_ref.get_untracked() else { return };
        let point = e.get_position(Some(&canvas));
        if e.properties().is_right_button_pressed {
            let circles = shown.get_untracked();
            if let Some(circle) = circle_at(&circles, point.x, point.y).and_then(|id| circles.into_iter().find(|c| c.id == id)) {
                adjust(drawer, circle);
            }
        } else {
            drawer.add(point.x, point.y);
        }
    };
    let moved = move |e: &PointerEventArgs| {
        let Some(canvas) = canvas_ref.get_untracked() else { return };
        let point = e.get_position(Some(&canvas));
        drawer.hovered.set_if_changed(circle_at(&shown.get_untracked(), point.x, point.y));
    };
    let circle = move |circle: Circle| {
        let radius = circle.diameter / 2.0;
        rsx! {
            Ellipse { width: circle.diameter, height: circle.diameter,
                Canvas.left: circle.x - radius, Canvas.top: circle.y - radius,
                stroke: "Black", stroke_thickness: 1.0,
                fill: move || if drawer.hovered.get() == Some(circle.id) { "LightGray" } else { "Transparent" } }
        }
    };

    rsx! {
        DockPanel { margin: 16.0,
            StackPanel { DockPanel.dock: Dock::Top, orientation: Orientation::Horizontal,
                horizontal_alignment: HorizontalAlignment::Center, spacing: 8.0, margin: (0.0, 0.0, 0.0, 8.0),
                Button { name: "Undo", is_enabled: move || drawer.history.with(History::can_undo),
                    on_click: move |_| drawer.history.update(History::undo), "Undo" }
                Button { name: "Redo", is_enabled: move || drawer.history.with(History::can_redo),
                    on_click: move |_| drawer.history.update(History::redo), "Redo" }
            }
            Border { border_brush: "Gray", border_thickness: 1.0,
                Canvas { name: "Canvas", background: "White", node_ref: canvas_ref,
                    on_pointer_pressed: pressed, on_pointer_moved: moved,
                    { for_each(move || shown.get(), |circle| circle.id, circle) }
                }
            }
        }
    }
}

/// Opens the window that adjusts the diameter of a circle; the diameter
/// shows as the slider moves and is kept when the window closes.
fn adjust(drawer: Drawer, circle: Circle) {
    drawer.adjusting.set(Some((circle.id, circle.diameter)));
    open_window(move || {
        let diameter = signal(circle.diameter);
        effect(move || {
            let diameter = diameter.get();
            untrack(|| drawer.adjusting.set(Some((circle.id, diameter))));
        });
        rsx! {
            Window { title: "Adjust diameter", width: 360.0, height: 120.0,
                with(move |window: &ferroui_base::Ref<ferroui_controls::Window>| {
                    let _ = window.closed(move || drawer.commit_adjustment());
                }),
                StackPanel { margin: 16.0, spacing: 8.0,
                    TextBlock { text: format!("Adjust the diameter of the circle at ({:.0}, {:.0}).", circle.x, circle.y) }
                    Slider { minimum: 4.0, maximum: 200.0, value: diameter.two_way() }
                }
            }
        }
    });
}
