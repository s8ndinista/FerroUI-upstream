//! Tests of the command state of [`NativeMenuItem`].

use crate::test_command::TestCommand;
use crate::test_support::test_scope;
use crate::NativeMenuItem;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// A command that can execute with an integer parameter only.
fn int_parameter_command() -> Rc<TestCommand> {
    TestCommand::with_can_execute(|parameter| parameter.is_some_and(|parameter| parameter.is::<i32>()))
}

fn int_parameter(value: i32) -> Option<BoxedValue> {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

#[test]
fn setting_command_parameter_after_command_should_enable_item() {
    let _scope = test_scope();
    let target = NativeMenuItem::new();
    target.set_command(int_parameter_command().as_command());

    assert!(!target.is_enabled());

    target.set_command_parameter(int_parameter(123));

    assert!(target.is_enabled());
}

#[test]
fn clearing_command_parameter_should_disable_item() {
    let _scope = test_scope();
    let target = NativeMenuItem::new();
    target.set_command_parameter(int_parameter(123));
    target.set_command(int_parameter_command().as_command());

    assert!(target.is_enabled());

    target.set_command_parameter(None);

    assert!(!target.is_enabled());
}

#[test]
fn setting_command_parameter_without_command_should_not_enable_item() {
    let _scope = test_scope();
    let target = NativeMenuItem::new();
    target.set_is_enabled(false);

    target.set_command_parameter(int_parameter(123));

    assert!(!target.is_enabled());
}
