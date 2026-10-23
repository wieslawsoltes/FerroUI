//! The task in a window of its own.

fn main() {
    seven_guis_model::run("hooks-flight-booker", "Flight Booker", 300.0, 270.0, || {
        ferroui_view::IntoElement::into_element(seven_guis_flight_booker_hooks::flight_booker())
    });
}
