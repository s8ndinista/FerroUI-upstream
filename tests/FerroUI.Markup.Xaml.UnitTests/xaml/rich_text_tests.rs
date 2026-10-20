//! Rich document/control declarations through the runtime XAML host. The same
//! fixture belongs to the compiled Rust emitter corpus, and the production theme
//! crates execute their actual compiled templates in their own integration tests.

use crate::support::app::{styled_window_application, xaml_test_base};
use crate::support::loader::parse;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FontStyle, FontWeight};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::Ref;
use ferroui_controls::documents::{InlineSnapshotKind, TextHighlightRange};
use ferroui_controls::{
    Button, Control, RichEditBox, RichTextBlock, RichTextBlockOverflow, TextBox, UserControl,
    Window,
};

const DOCUMENT: &str = include_str!("../emitter/fixtures/rich_text_document.xaml");

#[test]
fn rich_text_markup_preserves_nested_document_content_and_copied_highlight_values() {
    let _scope = xaml_test_base();
    let root = parse::<Ref<UserControl>>(DOCUMENT);
    let display = root.get_control::<RichTextBlock>("display");
    assert_eq!(display.text(), "Bold italic 😀\nSecond");
    let snapshot = display.document().snapshot();
    assert_eq!(snapshot.paragraphs.len(), 2);
    assert!(matches!(
        snapshot.paragraphs[0].inlines[0].kind,
        InlineSnapshotKind::Bold(_)
    ));
    assert!(matches!(
        snapshot.paragraphs[0].inlines[1].kind,
        InlineSnapshotKind::Italic(_)
    ));
    assert!(matches!(
        snapshot.paragraphs[0].inlines[2].kind,
        InlineSnapshotKind::Underline(_)
    ));
    assert_eq!(
        display
            .document()
            .character_format_at(0)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        display
            .document()
            .character_format_at(5)
            .unwrap()
            .font_style,
        Some(FontStyle::Italic)
    );
    let highlighter = display.text_highlighters().get(0);
    let copied = highlighter
        .ranges()
        .get(0)
        .expect("the declared highlight range is loaded");
    assert_eq!(copied, TextHighlightRange::new(0, 4));
    copied.set_length(9);
    assert_eq!(copied.length(), 9);
    assert_eq!(
        highlighter
            .ranges()
            .get(0)
            .expect("the stored highlight range remains present")
            .length(),
        4
    );
}

#[test]
fn rich_edit_markup_commands_bind_the_real_owner_and_follow_read_only_state() {
    let _app = styled_window_application();
    let root = parse::<Ref<UserControl>>(DOCUMENT);
    let editor = root.get_control::<RichEditBox>("editor");
    let button = root.get_control::<Button>("bold");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root)));
    window.show();
    editor.select_all();
    assert_eq!(editor.text(), "Edit 😀");
    assert_eq!(
        from_markup_value::<String>(&editor.header()),
        Some("Document".to_owned())
    );
    assert!(button.is_enabled());
    button
        .command()
        .expect("the markup command resolves ToggleBold")
        .execute(None);
    assert!(editor.is_bold());
    assert!(editor.can_undo());
    editor.set_is_read_only(true);
    assert!(!button.is_enabled());
    window.close();
}

#[test]
fn rich_edit_text_binding_updates_the_document_in_both_directions() {
    let _app = styled_window_application();
    let root = parse::<Ref<UserControl>>(
        r#"
      <UserControl xmlns="https://github.com/ferroui">
        <StackPanel>
          <TextBox Name="source" Text="first" />
          <RichEditBox Name="editor" Text="{Binding #source.Text, Mode=TwoWay}" />
        </StackPanel>
      </UserControl>"#,
    );
    let source = root.get_control::<TextBox>("source");
    let editor = root.get_control::<RichEditBox>("editor");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root)));
    window.show();
    assert_eq!(editor.text(), "first");
    source.set_text(Some("second 😀"));
    assert_eq!(editor.text(), "second 😀");
    editor.select_all();
    editor.insert_text("replacement").unwrap();
    assert_eq!(source.text().as_deref(), Some("replacement"));
    assert!(editor.undo().unwrap());
    assert_eq!(source.text().as_deref(), Some("second 😀"));
    window.close();
}

#[test]
fn rich_text_markup_links_overflow_to_the_same_document() {
    let _app = styled_window_application();
    let _text = TextTestScope::new();
    let root = parse::<Ref<UserControl>>(
        r#"
      <UserControl xmlns="https://github.com/ferroui">
        <StackPanel>
          <RichTextBlock Name="source" Height="20" Width="80" TextWrapping="Wrap" OverflowContentTarget="{Binding #overflow}">
            <Paragraph><Run Text="First second third fourth fifth sixth" /></Paragraph>
          </RichTextBlock>
          <RichTextBlockOverflow Name="overflow" Width="80" Height="120" />
        </StackPanel>
      </UserControl>"#,
    );
    let source = root.get_control::<RichTextBlock>("source");
    let overflow = root.get_control::<RichTextBlockOverflow>("overflow");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root)));
    window.show();
    window.update_layout();
    assert_eq!(source.overflow_content_target(), Some(overflow.clone()));
    assert_eq!(source.document(), overflow.document());
    assert!(source.has_overflow_content());
    assert!(!overflow.line_metrics().is_empty());
    window.close();
}
