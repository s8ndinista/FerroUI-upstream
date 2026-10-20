//! seven-guis-macros
//!
//! The seven tasks of 7GUIs (<https://eugenkiss.github.io/7guis/tasks>)
//! written with the `rsx!` macro and the components (`#[component]`) of
//! `ferroui-view`. `seven-guis-builders` writes the same user interfaces
//! with the builders; the logic they share is in `seven-guis-model`.
//!
//! ```text
//! cargo run -p seven-guis-macros
//! ```

pub mod cells;
pub mod circles;
pub mod counter;
pub mod crud;
pub mod flight;
pub mod temperature;
pub mod timer;

use cells::Cells;
use circles::CircleDrawer;
use counter::Counter;
use crud::Crud;
use ferroui_view::prelude::*;
use flight::FlightBooker;
use temperature::TemperatureConverter;
use timer::Timer;

/// The main window: one tab per task.
#[component]
pub fn App() -> Element {
    rsx! {
        Window { title: "7GUIs: macros", width: 920.0, height: 680.0,
            TabControl {
                TabItem { header: "Counter", Counter {} }
                TabItem { header: "Temperature", TemperatureConverter {} }
                TabItem { header: "Flight Booker", FlightBooker {} }
                TabItem { header: "Timer", Timer {} }
                TabItem { header: "CRUD", Crud {} }
                TabItem { header: "Circle Drawer", CircleDrawer {} }
                TabItem { header: "Cells", Cells {} }
            }
        }
    }
}

#[cfg(test)]
mod tests;
