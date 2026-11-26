use ferroui_view::prelude::*;

fn status() -> Element {
    rsx! {
        Border { background: "#EDF3F8", padding: 16.0,
            Border { name: "StatusMark", background: "#1E817A",
                border_brush: "#106763", border_thickness: 1.0,
                corner_radius: 8.0, width: 132.0, height: 40.0 }
        }
    }
}
