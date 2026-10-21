use super::*;
use ferroui_base::input::text_input::TextInputContentType;

#[test]
fn rich_input_scope_and_prediction_changes_reach_the_focused_backend_without_resetting_composition()
{
    let f = Fixture::new(true);
    let connection = f.connection();
    let session = connection.begin_composition().unwrap();
    session.update("preview", None).unwrap();
    let before = f.owner.document().snapshot();
    let resets = f.native.resets.get();
    let installations = f.native.installations.get();

    f.owner.set_input_scope(TextInputContentType::Email);
    f.owner.set_is_text_prediction_enabled(false);
    assert_eq!(
        f.native.options.borrow().content_type,
        TextInputContentType::Email
    );
    assert_eq!(f.native.options.borrow().show_suggestions, Some(false));
    assert_eq!(f.native.resets.get(), resets);
    assert_eq!(f.native.installations.get(), installations);
    assert!(Rc::ptr_eq(&connection, &f.connection()));
    assert!(session.is_current());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());

    assert!(session.commit("accepted", 0).unwrap());
    assert_eq!(f.owner.text(), "aacceptedd");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn rich_input_options_follow_the_semantic_owner_while_its_flyout_has_keyboard_focus() {
    for overlay in [false, true] {
        let f = Fixture::new(overlay);
        let connection = f.connection();
        let session = connection.begin_composition().unwrap();
        session.update("preview", None).unwrap();
        let child = Border::new();
        child.set_focusable(true);
        child.set_width(60.0);
        child.set_height(20.0);
        TextInputOptions::set_content_type(&child, TextInputContentType::Digits);
        let flyout = f.flyout(child.clone().upcast());
        let resets = f.native.resets.get();
        let installations = f.native.installations.get();
        flyout.show_at(&f.owner);
        assert!(child.is_focused());

        f.owner.set_input_scope(TextInputContentType::Url);
        f.owner.set_is_text_prediction_enabled(false);
        assert_eq!(
            f.native.options.borrow().content_type,
            TextInputContentType::Url
        );
        assert_eq!(f.native.options.borrow().show_suggestions, Some(false));
        TextInputOptions::set_content_type(&child, TextInputContentType::Number);
        assert_eq!(
            f.native.options.borrow().content_type,
            TextInputContentType::Url
        );
        assert_eq!(f.native.resets.get(), resets);
        assert_eq!(f.native.installations.get(), installations);
        assert!(Rc::ptr_eq(&connection, &f.connection()));
        assert!(session.is_current());
        assert_eq!(f.owner.text(), "apreviewd");
        assert!(session.cancel().unwrap());

        f.outside.focus();
        assert!(!connection.is_current());
        let last_options = f.native.options.borrow().clone();
        f.owner.set_input_scope(TextInputContentType::Email);
        f.owner.set_is_text_prediction_enabled(true);
        assert_eq!(*f.native.options.borrow(), last_options);
        flyout.hide();
    }
}
