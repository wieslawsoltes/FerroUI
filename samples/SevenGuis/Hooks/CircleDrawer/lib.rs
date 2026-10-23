//! Circle Drawer: a canvas where a click adds a circle, a right click on a
//! circle opens a window that adjusts its diameter, and undo and redo.
//!
//! Written as components that run again, with their state in hooks; `cargo run -p seven-guis-circle-drawer-hooks`.

use ferroui_base::input::{PointerEventArgs, PointerPressedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_controls::{Canvas, Dock};
use ferroui_view::prelude::*;
use seven_guis_model::state::Drawer;
use seven_guis_model::{circle_at, History};

/// The circle drawer, over its state. It runs again when the circles, the
/// circle under the pointer or the history change; the circles are a keyed
/// loop, so each keeps its ellipse.
#[component(rerender)]
pub fn CircleDrawer(#[prop(default = Drawer::with_example())] drawer: Drawer) -> Element {
    let canvas_ref = use_hook(NodeRef::<Canvas>::new);
    let circles = drawer.circles();
    let hovered = drawer.hovered.get();
    let (can_undo, can_redo) = drawer.history.with(|history| (history.can_undo(), history.can_redo()));

    let pressed = {
        let circles = circles.clone();
        move |e: &PointerPressedEventArgs| {
            let Some(canvas) = canvas_ref.get_untracked() else { return };
            let point = e.get_position(Some(&canvas));
            if e.properties().is_right_button_pressed {
                if let Some(circle) = circle_at(&circles, point.x, point.y).and_then(|id| circles.iter().find(|c| c.id == id)) {
                    let circle = *circle;
                    drawer.adjusting.set(Some((circle.id, circle.diameter)));
                    open_window(move || rsx! { AdjustDiameter { drawer, id: circle.id, x: circle.x, y: circle.y, diameter: circle.diameter } });
                }
            } else {
                drawer.add(point.x, point.y);
            }
        }
    };
    let moved = {
        let circles = circles.clone();
        move |e: &PointerEventArgs| {
            let Some(canvas) = canvas_ref.get_untracked() else { return };
            let point = e.get_position(Some(&canvas));
            drawer.hovered.set_if_changed(circle_at(&circles, point.x, point.y));
        }
    };

    rsx! {
        DockPanel { margin: 16.0,
            StackPanel { DockPanel.dock: Dock::Top, orientation: Orientation::Horizontal,
                horizontal_alignment: HorizontalAlignment::Center, spacing: 8.0, margin: (0.0, 0.0, 0.0, 8.0),
                Button { name: "Undo", is_enabled: can_undo, on_click: move |_| drawer.history.update(History::undo), "Undo" }
                Button { name: "Redo", is_enabled: can_redo, on_click: move |_| drawer.history.update(History::redo), "Redo" }
            }
            Border { border_brush: "Gray", border_thickness: 1.0,
                Canvas { name: "Canvas", background: "White", node_ref: canvas_ref,
                    on_pointer_pressed: pressed, on_pointer_moved: moved,
                    for circle in &circles {
                        Ellipse { key: circle.id, width: circle.diameter, height: circle.diameter,
                            Canvas.left: circle.x - circle.diameter / 2.0, Canvas.top: circle.y - circle.diameter / 2.0,
                            stroke: "Black", stroke_thickness: 1.0,
                            fill: if hovered == Some(circle.id) { "LightGray" } else { "Transparent" } }
                    }
                }
            }
        }
    }
}

/// The window that adjusts the diameter of a circle: the diameter shows as
/// the slider moves and is kept when the window closes.
#[component(rerender)]
fn AdjustDiameter(drawer: Drawer, id: u32, x: f64, y: f64, diameter: f64) -> Element {
    let value = use_signal(|| diameter);
    use_effect_with(value.get(), move |diameter| drawer.adjusting.set(Some((id, *diameter))));
    rsx! {
        Window { title: "Adjust diameter", width: 360.0, height: 120.0,
            with(move |window: &ferroui_base::Ref<ferroui_controls::Window>| {
                let _ = window.closed(move || drawer.commit_adjustment());
            }),
            StackPanel { margin: 16.0, spacing: 8.0,
                TextBlock { text: format!("Adjust the diameter of the circle at ({x:.0}, {y:.0}).") }
                Slider { minimum: 4.0, maximum: 200.0, value: value.two_way() }
            }
        }
    }
}
