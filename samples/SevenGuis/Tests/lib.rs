//! The scenarios of the 7GUIs tasks (`seven_guis_model::scenarios`), run on the
//! user interface of each port: the builders, the `rsx!` macro, and
//! components that run again with hooks, of `ferroui-view`. One crate for the
//! twenty-one ports, so that they link into one test binary.

#[cfg(test)]
mod tests {
    use seven_guis_model::scenarios;

    #[test]
    fn counter_builders() {
        scenarios::counter(seven_guis_counter_builders::counter);
    }

    #[test]
    fn counter_macros() {
        scenarios::counter(seven_guis_counter_macros::Counter);
    }

    #[test]
    fn temperature_builders() {
        scenarios::temperature(seven_guis_temperature_builders::temperature_converter);
    }

    #[test]
    fn temperature_macros() {
        scenarios::temperature(seven_guis_temperature_macros::TemperatureConverter);
    }

    #[test]
    fn flight_booker_builders() {
        scenarios::flight(seven_guis_flight_booker_builders::flight_booker);
    }

    #[test]
    fn flight_booker_macros() {
        scenarios::flight(seven_guis_flight_booker_macros::FlightBooker);
    }

    #[test]
    fn timer_builders() {
        scenarios::timer(seven_guis_timer_builders::timer_view);
    }

    #[test]
    fn timer_macros() {
        scenarios::timer(seven_guis_timer_macros::TimerView);
    }

    #[test]
    fn crud_builders() {
        scenarios::crud(seven_guis_crud_builders::crud);
    }

    #[test]
    fn crud_macros() {
        scenarios::crud(seven_guis_crud_macros::Crud);
    }

    #[test]
    fn circle_drawer_builders() {
        scenarios::circles(seven_guis_circle_drawer_builders::circle_drawer_with);
    }

    #[test]
    fn circle_drawer_macros() {
        scenarios::circles(seven_guis_circle_drawer_macros::CircleDrawer);
    }

    #[test]
    fn cells_builders() {
        scenarios::cells(seven_guis_cells_builders::cells_of);
    }

    #[test]
    fn cells_macros() {
        scenarios::cells(seven_guis_cells_macros::Cells);
    }

    #[test]
    fn counter_hooks() {
        scenarios::counter(|| ferroui_view::IntoElement::into_element(seven_guis_counter_hooks::counter()));
    }

    #[test]
    fn temperature_hooks() {
        scenarios::temperature(|| ferroui_view::IntoElement::into_element(seven_guis_temperature_hooks::temperature_converter()));
    }

    #[test]
    fn flight_booker_hooks() {
        scenarios::flight(|| ferroui_view::IntoElement::into_element(seven_guis_flight_booker_hooks::flight_booker()));
    }

    #[test]
    fn timer_hooks() {
        scenarios::timer(|elapsed, duration| {
            ferroui_view::IntoElement::into_element(seven_guis_timer_hooks::timer_view().elapsed(elapsed).duration(duration))
        });
    }

    #[test]
    fn crud_hooks() {
        scenarios::crud(|| ferroui_view::IntoElement::into_element(seven_guis_crud_hooks::crud()));
    }

    #[test]
    fn circle_drawer_hooks() {
        scenarios::circles(|drawer| ferroui_view::IntoElement::into_element(seven_guis_circle_drawer_hooks::circle_drawer().drawer(drawer)));
    }

    #[test]
    fn cells_hooks() {
        scenarios::cells(|sheet| ferroui_view::IntoElement::into_element(seven_guis_cells_hooks::cells().sheet(sheet)));
    }
}
