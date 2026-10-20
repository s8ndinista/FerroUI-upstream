//! The scenarios of the 7GUIs tasks (`seven_guis_model::scenarios`), run on the
//! user interface of each port: the builders and the `rsx!` macro of
//! `ferroui-view`. One crate for the fourteen ports, so that they link into
//! one test binary.

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
}
