//! The scenarios of the tasks (`seven_guis_model::scenarios`), run on the
//! user interface written with the `rsx!` macro.

use crate::cells::Cells;
use crate::circles::CircleDrawer;
use crate::counter::Counter;
use crate::crud::Crud;
use crate::flight::FlightBooker;
use crate::temperature::TemperatureConverter;
use crate::timer::TimerView;
use seven_guis_model::scenarios;

#[test]
fn counter() {
    scenarios::counter(Counter);
}

#[test]
fn temperature() {
    scenarios::temperature(TemperatureConverter);
}

#[test]
fn flight() {
    scenarios::flight(FlightBooker);
}

#[test]
fn timer() {
    scenarios::timer(TimerView);
}

#[test]
fn crud() {
    scenarios::crud(Crud);
}

#[test]
fn circles() {
    scenarios::circles(CircleDrawer);
}

#[test]
fn cells() {
    scenarios::cells(Cells);
}
