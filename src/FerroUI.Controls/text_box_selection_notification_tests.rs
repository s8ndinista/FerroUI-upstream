//! Port of upstream's `TextBoxSelectionNotificationTests.cs`, on the test
//! fonts and the services of the text box tests.

use crate::text_box_tests::{create_template, raise_text_event, test_scope};
use crate::TextBox;
use ferroui_base::input::text_input::{TextInputMethodClient, TextInputMethodClientRequestedEventArgs, TextSelection};
use ferroui_base::input::InputElement;
use ferroui_base::Ref;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn setting_input_method_selection_notifies_after_both_endpoints_are_assigned() {
    for (start, end) in [(8, 18), (18, 8)] {
        let _app = test_scope();
        let text_box = create_text_box();
        let client = get_client(&text_box);
        let selections = Rc::new(RefCell::new(Vec::new()));
        let _subscription = client.selection_changed({
            let (client, selections) = (Rc::downgrade(&client), selections.clone());
            Rc::new(move || {
                if let Some(client) = client.upgrade() {
                    selections.borrow_mut().push(client.selection());
                }
            })
        });

        client.set_selection(TextSelection::new(start, end));

        assert_eq!(vec![TextSelection::new(start, end)], *selections.borrow(), "{start} {end}");
    }
}

#[test]
fn setting_input_method_selection_allows_reentrant_input_from_selection_changed() {
    for (start, end) in [(8, 18), (18, 8)] {
        let _app = test_scope();
        let text_box = create_text_box();
        let client = get_client(&text_box);
        let inserted = Rc::new(Cell::new(false));
        let _subscription = client.selection_changed({
            let (text_box, inserted) = (text_box.downgrade(), inserted.clone());
            Rc::new(move || {
                if inserted.get() {
                    return;
                }

                inserted.set(true);
                if let Some(text_box) = text_box.upgrade() {
                    raise_text_event(&text_box, "!");
                }
            })
        });

        client.set_selection(TextSelection::new(start, end));

        assert!(inserted.get(), "{start} {end}");
        assert_eq!(Some("abcdefgh!st".to_string()), text_box.text(), "{start} {end}");
        assert_eq!(TextSelection::new(9, 9), client.selection(), "{start} {end}");
        assert_eq!(9, text_box.caret_index(), "{start} {end}");
    }
}

#[test]
fn replacing_text_notifies_input_method_after_both_selection_endpoints_are_coerced() {
    for (start, end, expected_start, expected_end) in [(8, 18, 5, 5), (18, 8, 5, 5), (2, 18, 2, 5), (18, 2, 5, 2)] {
        let _app = test_scope();
        let text_box = create_text_box();
        let client = get_client(&text_box);
        client.set_selection(TextSelection::new(start, end));
        assert_eq!(0, text_box.caret_index(), "{start} {end}");

        let selections = Rc::new(RefCell::new(Vec::new()));
        let _subscription = client.selection_changed({
            let (client, selections) = (Rc::downgrade(&client), selections.clone());
            Rc::new(move || {
                if let Some(client) = client.upgrade() {
                    selections.borrow_mut().push(client.selection());
                }
            })
        });

        text_box.set_text(Some("short"));

        assert_eq!(vec![TextSelection::new(expected_start, expected_end)], *selections.borrow(), "{start} {end}");
        assert_eq!(Some("short".to_string()), text_box.text(), "{start} {end}");
    }
}

#[test]
fn replacing_text_allows_reentrant_input_from_input_method_selection_changed() {
    for (start, end) in [(8, 18), (18, 8)] {
        let _app = test_scope();
        let text_box = create_text_box();
        let client = get_client(&text_box);
        client.set_selection(TextSelection::new(start, end));
        assert_eq!(0, text_box.caret_index(), "{start} {end}");

        let inserted = Rc::new(Cell::new(false));
        let _subscription = client.selection_changed({
            let (text_box, inserted) = (text_box.downgrade(), inserted.clone());
            Rc::new(move || {
                if inserted.get() {
                    return;
                }

                inserted.set(true);
                if let Some(text_box) = text_box.upgrade() {
                    raise_text_event(&text_box, "!");
                }
            })
        });

        text_box.set_text(Some("short"));

        assert!(inserted.get(), "{start} {end}");
        assert_eq!(Some("short!".to_string()), text_box.text(), "{start} {end}");
        assert_eq!(TextSelection::new(6, 6), client.selection(), "{start} {end}");
        assert_eq!(6, text_box.caret_index(), "{start} {end}");
    }
}

#[test]
fn replacing_text_preserves_valid_partial_selection_without_notifying_input_method() {
    let _app = test_scope();
    let text_box = create_text_box();
    let client = get_client(&text_box);
    client.set_selection(TextSelection::new(2, 4));
    let notifications = Rc::new(Cell::new(0));
    let _subscription = client.selection_changed({
        let notifications = notifications.clone();
        Rc::new(move || notifications.set(notifications.get() + 1))
    });

    text_box.set_text(Some("short"));

    assert_eq!(TextSelection::new(2, 4), client.selection());
    assert_eq!("or", text_box.selected_text());
    assert_eq!(0, notifications.get());
}

fn create_text_box() -> Ref<TextBox> {
    let text_box = TextBox::new();
    text_box.set_template(create_template());
    text_box.set_text(Some("abcdefghijklmnopqrst"));
    text_box.set_caret_index(0);
    text_box.apply_template();
    text_box
}

fn get_client(text_box: &TextBox) -> Rc<dyn TextInputMethodClient> {
    let args = TextInputMethodClientRequestedEventArgs::new();
    args.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));
    text_box.raise_event(&args);

    args.client().expect("the text box provides an input method client")
}
