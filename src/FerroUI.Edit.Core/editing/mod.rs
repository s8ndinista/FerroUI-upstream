mod caret_navigation;
mod editor_session;
mod read_only_section_provider;
mod rectangle_selection;
mod selection;

pub use caret_navigation::{next_caret_position, CaretMovement};
pub use editor_session::{EditError, EditorOptions, EditorSession};
pub use read_only_section_provider::{IReadOnlySectionProvider, NoReadOnlySections, TextSegmentReadOnlySectionProvider};
pub use rectangle_selection::RectangleSelection;
pub use selection::{Selection, SelectionSegment};
