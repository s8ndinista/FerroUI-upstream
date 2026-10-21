use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Button, Control, ScrollViewer, StackPanel, TextBlock};
pub(crate) fn text(value: &str, size: f64) -> Ref<TextBlock> {
    let block = TextBlock::new();
    block.set_text(Some(value));
    block.set_font_size(size);
    block.set_text_wrapping(TextWrapping::Wrap);
    block
}
pub(crate) fn heading(value: &str) -> Ref<TextBlock> {
    let block = text(value, 28.0);
    block.set_font_weight(FontWeight::Bold);
    block
}
pub(crate) fn stack(spacing: f64) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(spacing);
    panel
}
pub(crate) fn action(label: &str, icon: &str, invoke: impl Fn() + 'static) -> Ref<Button> {
    let button = Button::new();
    button.set_min_height(44.0);
    button.set_min_width(44.0);
    button.set_content(Some(Control::boxed(&nav_content(
        icon, label, false, false,
    ))));
    AutomationProperties::set_name(&button, Some(label));
    button.click(move |_, _| invoke());
    button
}
pub(crate) fn nav_content(
    icon: &str,
    label: &str,
    compact: bool,
    vertical: bool,
) -> Ref<StackPanel> {
    let panel = stack(if vertical { 4.0 } else { 12.0 });
    panel.set_orientation(if vertical {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    });
    panel.set_vertical_alignment(VerticalAlignment::Center);
    let glyph = crate::icons::icon(icon);
    panel.children().add(glyph.upcast::<Control>());
    if !compact {
        let label = text(label, if vertical { 10.0 } else { 13.0 });
        label.set_vertical_alignment(VerticalAlignment::Center);
        label.set_horizontal_alignment(if vertical {
            HorizontalAlignment::Center
        } else {
            HorizontalAlignment::Left
        });
        panel.children().add(label.upcast::<Control>());
    }
    panel
}
pub(crate) fn scroll(control: Ref<Control>) -> Ref<Control> {
    let scroll = ScrollViewer::new();
    scroll.set_content(Some(Control::boxed(&control)));
    scroll.upcast::<Control>()
}
pub(crate) fn welcome(open_search: impl Fn() + 'static) -> Ref<Control> {
    let panel = stack(22.0);
    panel.set_margin(Thickness::new(38.0, 60.0, 38.0, 36.0));
    panel.set_max_width(660.0);
    panel.set_horizontal_alignment(HorizontalAlignment::Center);
    let eyebrow = text("STAY CURIOUS.", 11.0);
    eyebrow.classes().add("eyebrow");
    panel.children().add(eyebrow.upcast::<Control>());
    let title = text("A little signal.\nA lot to discover.", 42.0);
    title.set_font_weight(FontWeight::Bold);
    title.set_line_height(50.0);
    panel.children().add(title.upcast::<Control>());
    let body = text(
        "A considered space for the stories, ideas and conversations that make Hacker News worth reading.",
        17.0,
    );
    body.classes().add("muted");
    body.set_line_height(28.0);
    panel.children().add(body.upcast::<Control>());
    panel
        .children()
        .add(action("Find your next rabbit hole", "search", open_search).upcast::<Control>());
    for (icon, title, description) in [
        (
            "chat",
            "Follow the conversation",
            "Thoughtful threads, native rich text, and room to read.",
        ),
        (
            "bookmark",
            "Make it yours",
            "Save a story, hide the noise, and pick up where you left off.",
        ),
        (
            "download",
            "Take it with you",
            "Download discussions and keep reading offline.",
        ),
    ] {
        let row = stack(8.0);
        row.set_margin(Thickness::new(0.0, 12.0, 0.0, 0.0));
        row.children()
            .add(nav_content(icon, title, false, false).upcast::<Control>());
        let caption = text(description, 13.0);
        caption.classes().add("muted");
        row.children().add(caption.upcast::<Control>());
        panel.children().add(row.upcast::<Control>());
    }
    scroll(panel.upcast::<Control>())
}
