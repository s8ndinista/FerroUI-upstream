//! The scenarios of the tasks (`seven_guis_model::scenarios`), run on the
//! user interface written with the builders.

use seven_guis_model::scenarios;

#[test]
fn counter() {
    scenarios::counter(crate::counter::counter);
}

#[test]
fn temperature() {
    scenarios::temperature(crate::temperature::temperature_converter);
}

#[test]
fn flight() {
    scenarios::flight(crate::flight::flight_booker);
}

#[test]
fn timer() {
    scenarios::timer(crate::timer::timer_view);
}

#[test]
fn crud() {
    scenarios::crud(crate::crud::crud);
}

#[test]
fn circles() {
    scenarios::circles(crate::circles::circle_drawer_with);
}

#[test]
fn cells() {
    scenarios::cells(crate::cells::cells_of);
}
