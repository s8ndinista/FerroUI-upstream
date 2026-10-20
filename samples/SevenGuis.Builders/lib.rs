//! seven-guis-builders
//!
//! The seven tasks of 7GUIs (<https://eugenkiss.github.io/7guis/tasks>)
//! written with the builders of `ferroui-view`: chained setters, signals
//! the properties follow, children that follow state. `seven-guis-macros`
//! writes the same user interfaces with the `rsx!` macro; the logic they
//! share is in `seven-guis-model`.
//!
//! ```text
//! cargo run -p seven-guis-builders
//! ```

pub mod cells;
pub mod circles;
pub mod counter;
pub mod crud;
pub mod flight;
pub mod temperature;
pub mod timer;

use ferroui_view::prelude::*;

/// The main window: one tab per task.
pub fn app() -> Element {
    window()
        .title("7GUIs: builders")
        .width(920.0)
        .height(680.0)
        .child(
            tab_control()
                .child(tab_item().header("Counter").content(counter::counter()))
                .child(tab_item().header("Temperature").content(temperature::temperature_converter()))
                .child(tab_item().header("Flight Booker").content(flight::flight_booker()))
                .child(tab_item().header("Timer").content(timer::timer()))
                .child(tab_item().header("CRUD").content(crud::crud()))
                .child(tab_item().header("Circle Drawer").content(circles::circle_drawer()))
                .child(tab_item().header("Cells").content(cells::cells())),
        )
        .into()
}

#[cfg(test)]
mod tests;
