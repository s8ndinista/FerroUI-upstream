//! The input actions of upstream's tests: `OpenQA.Selenium.Interactions.Actions`
//! (a chain of mouse and keyboard steps run by `Perform`) and the pointer
//! sequences of `PointerInputDevice` and `ActionBuilder`.
//!
//! The rules are the ones of the drivers upstream runs against
//! (docs/porting/integration-tests.md, section 3): `MoveToElement(element)`
//! moves to the centre, `MoveToElement(element, x, y)` to an offset from the
//! top left corner (upstream's default configuration, Appium 1),
//! `MoveToElementCenter(element, x, y)` and a pointer move of a sequence to an
//! offset from the centre. A key or a button pressed by an action stays down
//! until an action releases it, also across `Perform`. A move while a button
//! is down takes 250 ms, in steps, so that a drag sees the moves between; a
//! move with no button down is immediate.

use super::keys::{key_of, Key, NamedKey};
use super::session::{Element, MouseButton, Session};
use std::time::Duration;

/// How long a move of the actions takes while a button is down.
const DRAG_MOVE_DURATION: Duration = Duration::from_millis(250);

enum Step {
    MoveToCenter(Element, f64, f64),
    MoveToTopLeft(Element, f64, f64),
    MoveByOffset(f64, f64),
    Down(MouseButton),
    Up(MouseButton),
    Click(MouseButton),
    DoubleClick,
    KeyDown(NamedKey),
    KeyUp(NamedKey),
    SendKeys(String),
}

/// `new Actions(Session)`.
pub struct Actions {
    session: Session,
    steps: Vec<Step>,
}

fn named_key(key: &str) -> NamedKey {
    match key.chars().next().map(key_of) {
        Some(Key::Named(named)) if key.chars().count() == 1 => named,
        _ => panic!("{key:?} is not a modifier key"),
    }
}

impl Actions {
    pub fn new(session: &Session) -> Actions {
        Actions { session: session.clone(), steps: Vec::new() }
    }

    /// `MoveToElement(element)`.
    pub fn move_to_element(mut self, element: &Element) -> Self {
        self.steps.push(Step::MoveToCenter(element.clone(), 0.0, 0.0));
        self
    }

    /// `MoveToElement(element, x, y)`: an offset from the top left corner.
    pub fn move_to_element_offset(mut self, element: &Element, x: i32, y: i32) -> Self {
        self.steps.push(Step::MoveToTopLeft(element.clone(), f64::from(x), f64::from(y)));
        self
    }

    /// `MoveToElementCenter(element, x, y)` of `AppiumDriverEx`.
    pub fn move_to_element_center(mut self, element: &Element, x: i32, y: i32) -> Self {
        self.steps.push(Step::MoveToCenter(element.clone(), f64::from(x), f64::from(y)));
        self
    }

    /// `MoveByOffset(x, y)`.
    pub fn move_by_offset(mut self, x: i32, y: i32) -> Self {
        self.steps.push(Step::MoveByOffset(f64::from(x), f64::from(y)));
        self
    }

    /// `Click()`.
    pub fn click(mut self) -> Self {
        self.steps.push(Step::Click(MouseButton::Left));
        self
    }

    /// `Click(element)`.
    pub fn click_on(self, element: &Element) -> Self {
        self.move_to_element(element).click()
    }

    /// `ClickAndHold()`.
    pub fn click_and_hold(mut self) -> Self {
        self.steps.push(Step::Down(MouseButton::Left));
        self
    }

    /// `ClickAndHold(element)`.
    pub fn click_and_hold_on(self, element: &Element) -> Self {
        self.move_to_element(element).click_and_hold()
    }

    /// `Release()`.
    pub fn release(mut self) -> Self {
        self.steps.push(Step::Up(MouseButton::Left));
        self
    }

    /// `Release(element)`.
    pub fn release_on(self, element: &Element) -> Self {
        self.move_to_element(element).release()
    }

    /// `DoubleClick()`.
    pub fn double_click(mut self) -> Self {
        self.steps.push(Step::DoubleClick);
        self
    }

    /// `DoubleClick(element)`.
    pub fn double_click_on(self, element: &Element) -> Self {
        self.move_to_element(element).double_click()
    }

    /// `ContextClick(element)`.
    pub fn context_click_on(mut self, element: &Element) -> Self {
        self = self.move_to_element(element);
        self.steps.push(Step::Click(MouseButton::Right));
        self
    }

    /// `KeyDown(key)`.
    pub fn key_down(mut self, key: &str) -> Self {
        self.steps.push(Step::KeyDown(named_key(key)));
        self
    }

    /// `KeyUp(key)`.
    pub fn key_up(mut self, key: &str) -> Self {
        self.steps.push(Step::KeyUp(named_key(key)));
        self
    }

    /// `SendKeys(text)`: to the element that has the keyboard focus.
    pub fn send_keys(mut self, text: &str) -> Self {
        self.steps.push(Step::SendKeys(text.to_owned()));
        self
    }

    fn move_to(&self, x: f64, y: f64) {
        let duration = if self.session.any_button_down() { DRAG_MOVE_DURATION } else { Duration::ZERO };
        self.session.move_pointer(x, y, duration);
    }

    /// `Perform()`.
    pub fn perform(self) {
        let session = &self.session;
        for step in &self.steps {
            match step {
                Step::MoveToCenter(element, dx, dy) => {
                    let (x, y) = element.rect().center();
                    self.move_to(x + dx, y + dy);
                }
                Step::MoveToTopLeft(element, dx, dy) => {
                    let rect = element.rect();
                    self.move_to(rect.x + dx, rect.y + dy);
                }
                Step::MoveByOffset(dx, dy) => {
                    let (x, y) = session.pointer();
                    self.move_to(x + dx, y + dy);
                }
                Step::Down(button) => session.button_down(*button, 1),
                Step::Up(button) => session.button_up(*button, 1),
                Step::Click(button) => {
                    session.button_down(*button, 1);
                    std::thread::sleep(Duration::from_millis(30));
                    session.button_up(*button, 1);
                }
                Step::DoubleClick => session.double_click_here(),
                Step::KeyDown(key) => session.key_down(*key),
                Step::KeyUp(key) => session.key_up(*key),
                Step::SendKeys(text) => session.type_keys(text, false),
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        session.settle();
    }
}

/// `CoordinateOrigin` of a pointer move.
#[derive(Clone)]
pub enum CoordinateOrigin {
    Element(Element),
    Pointer,
}

/// One action of a pointer sequence (`Interaction`).
pub enum Interaction {
    PointerMove { origin: CoordinateOrigin, x: i32, y: i32, duration: Duration },
    PointerDown(MouseButton),
    PointerUp(MouseButton),
    Pause(Duration),
}

/// `PointerInputDevice` of the kind `PointerKind.Mouse`.
pub struct PointerInputDevice;

impl PointerInputDevice {
    pub fn mouse() -> Self {
        PointerInputDevice
    }

    /// `CreatePointerMove(element, x, y, duration)`: an offset from the centre.
    pub fn create_pointer_move(&self, element: &Element, x: i32, y: i32, duration: Duration) -> Interaction {
        Interaction::PointerMove { origin: CoordinateOrigin::Element(element.clone()), x, y, duration }
    }

    /// `CreatePointerMove(origin, x, y, duration)`.
    pub fn create_pointer_move_from(&self, origin: CoordinateOrigin, x: i32, y: i32, duration: Duration) -> Interaction {
        Interaction::PointerMove { origin, x, y, duration }
    }

    pub fn create_pointer_down(&self, button: MouseButton) -> Interaction {
        Interaction::PointerDown(button)
    }

    pub fn create_pointer_up(&self, button: MouseButton) -> Interaction {
        Interaction::PointerUp(button)
    }

    pub fn create_pause(&self, duration: Duration) -> Interaction {
        Interaction::Pause(duration)
    }
}

/// `ActionBuilder`.
#[derive(Default)]
pub struct ActionBuilder {
    interactions: Vec<Interaction>,
}

impl ActionBuilder {
    pub fn new() -> Self {
        ActionBuilder::default()
    }

    pub fn add_action(&mut self, interaction: Interaction) {
        self.interactions.push(interaction);
    }

    pub fn to_action_sequence_list(self) -> Vec<Interaction> {
        self.interactions
    }
}

impl Session {
    /// `PerformActions(sequences)`.
    pub fn perform_actions(&self, interactions: Vec<Interaction>) {
        for interaction in interactions {
            match interaction {
                Interaction::PointerMove { origin, x, y, duration } => {
                    let (origin_x, origin_y) = match origin {
                        CoordinateOrigin::Element(element) => element.rect().center(),
                        CoordinateOrigin::Pointer => self.pointer(),
                    };
                    self.move_pointer(origin_x + f64::from(x), origin_y + f64::from(y), duration);
                }
                Interaction::PointerDown(button) => self.button_down(button, 1),
                Interaction::PointerUp(button) => self.button_up(button, 1),
                Interaction::Pause(duration) => std::thread::sleep(duration),
            }
        }
        self.settle();
    }
}
