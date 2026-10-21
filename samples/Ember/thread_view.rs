//! The virtualized native conversation surface. Only realized rows create rich-text controls.

use ember_content::{HtmlDocument, RenderOptions};
use ember_core::{
    HnItem, ReaderState, ThreadOptions, ThreadRow, ThreadSnapshot, ThreadSort, flatten_thread,
    relative_time,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{Color, FontWeight, IBrush, SolidColorBrush, TextWrapping};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    BoxedValue, CornerRadius, FerroObject, FerroProperty, Point, Ref, Thickness, Vector,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    Application, Border, Button, ColumnDefinitions, ComboBox, Control, Grid, ItemsSource, ListBox,
    ProgressBar, ScrollViewer, StackPanel, TextBlock, WrapPanel,
};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::cell::{Cell, OnceCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum ThreadAction {
    OpenUrl(String),
    Profile(String),
    ToggleCollapse(u64),
    Save,
    Share,
    Hide,
    MuteDomain,
    Reply(u64),
    Vote(u64),
    LoadMore,
    Sort(ThreadSort),
    Parent(u64),
}

#[derive(Clone)]
enum RowKind {
    Header,
    Comment(ThreadRow),
    Empty,
    Footer,
}

#[derive(Clone)]
struct ConversationRow {
    key: usize,
    kind: RowKind,
    document: OnceCell<HtmlDocument>,
}

#[derive(PartialEq)]
struct ThreadAnchors {
    root: u64,
    indices: BTreeMap<u64, i32>,
    parents: BTreeMap<u64, u64>,
}
impl PartialEq for ConversationRow {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

/// The parent must arrange this control into a finite-height viewport; putting it inside an
/// outer ScrollViewer would disable ListBox's VirtualizingStackPanel.
pub fn build(
    snapshot: &ThreadSnapshot,
    state: &ReaderState,
    query: &str,
    sort: ThreadSort,
    on_action: Rc<dyn Fn(ThreadAction)>,
) -> Ref<Control> {
    let query = query.trim();
    let local = state.items.get(&snapshot.root.id);
    let options = ThreadOptions {
        collapsed: local.map(|s| s.collapsed.clone()).unwrap_or_default(),
        query: query.to_owned(),
        sort,
        show_dead: state.preferences.show_dead,
        ..Default::default()
    };
    let comments = flatten_thread(snapshot, &options);
    let mut rows = vec![Rc::new(ConversationRow {
        key: 0,
        kind: RowKind::Header,
        document: OnceCell::new(),
    })];
    for comment in comments {
        rows.push(Rc::new(ConversationRow {
            key: rows.len(),
            kind: RowKind::Comment(comment),
            document: OnceCell::new(),
        }));
    }
    if rows.len() == 1 {
        rows.push(Rc::new(ConversationRow {
            key: rows.len(),
            kind: RowKind::Empty,
            document: OnceCell::new(),
        }));
    }
    if !snapshot.complete || snapshot.warning.is_some() {
        rows.push(Rc::new(ConversationRow {
            key: rows.len(),
            kind: RowKind::Footer,
            document: OnceCell::new(),
        }));
    }
    let list = ListBox::new();
    list.classes().add("reader-list");
    list.classes().add("thread-list");
    list.set_padding(Thickness::new(20.0, 0.0, 20.0, 16.0));
    list.set_auto_scroll_to_selected_item(false);
    ScrollViewer::set_horizontal_scroll_bar_visibility_on(&list, ScrollBarVisibility::Disabled);
    ScrollViewer::set_vertical_scroll_bar_visibility_on(&list, ScrollBarVisibility::Auto);
    let indices = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| match &row.kind {
            RowKind::Comment(comment) => Some((comment.id, index as i32)),
            _ => None,
        })
        .collect();
    let mut parents = BTreeMap::new();
    for item in snapshot
        .items
        .values()
        .chain(std::iter::once(&snapshot.root))
    {
        for &child in &item.kids {
            parents.entry(child).or_insert(item.id);
        }
    }
    list.set_tag(Some(Rc::new(ThreadAnchors {
        root: snapshot.root.id,
        indices,
        parents,
    }) as BoxedValue));
    list.set_items_source(Some(ItemsSource::from_values(rows)));
    AutomationProperties::set_name(&list, Some("Hacker News conversation"));
    let snapshot = Rc::new(snapshot.clone());
    let state = Rc::new(state.clone());
    let query = query.to_owned();
    list.set_item_template(Some(FuncDataTemplate::for_type::<Rc<ConversationRow>>(
        move |row, _| {
            Some(match &row.kind {
                RowKind::Header => header(&snapshot, &state, sort, &on_action),
                RowKind::Comment(comment) => comment_row(
                    comment,
                    &row.document,
                    &snapshot.root,
                    &state,
                    !query.is_empty(),
                    &on_action,
                ),
                RowKind::Empty => empty(&snapshot, &query, &on_action),
                RowKind::Footer => footer(&snapshot, &on_action),
            })
        },
        // Native containers are virtualized; document controls are rebuilt for a different
        // item so a recycled row can never retain another comment's action closures.
        false,
    )));
    list.upcast()
}

/// The first comment intersecting the viewport, including a partially visible comment.
/// This uses realized containers rather than estimating an index from variable row heights.
pub fn viewport_anchor(control: &Control) -> Option<u64> {
    let list = conversation_list(control)?;
    let height = list.bounds().height;
    list.get_realized_containers()
        .into_iter()
        .filter_map(|container| {
            if !container.is_effectively_visible() {
                return None;
            }
            let origin = container.translate_point(Point::new(0.0, 0.0), &list)?;
            if origin.y >= height || origin.y + container.bounds().height <= 0.0 {
                return None;
            }
            let item = list.item_from_container(&container)?;
            let row = item.downcast_ref::<Rc<ConversationRow>>()?;
            match &row.kind {
                RowKind::Comment(comment) => Some((origin.y, comment.id)),
                _ => None,
            }
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// Restores reading context after rebuilding a thread. If an ancestor was collapsed,
/// the nearest visible ancestor is used. A filtered-out branch falls back to its root.
/// Calling this before attaching the newly built control is supported.
pub fn restore_anchor(control: &Control, id: u64) {
    let Some(list) = conversation_list(control) else {
        return;
    };
    let Some(index) = anchor_index(&list, id) else {
        return;
    };
    if list.is_loaded() {
        let weak = list.downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(list) = weak.upgrade().filter(|list| list.is_loaded()) {
                    scroll_anchor_into_view(&list, index);
                }
            },
            DispatcherPriority::LOADED,
        );
    } else {
        let weak = list.downgrade();
        let restored = Cell::new(false);
        list.loaded(move |_, _| {
            if !restored.replace(true) {
                if let Some(list) = weak.upgrade() {
                    scroll_anchor_into_view(&list, index);
                }
            }
        });
    }
}

fn conversation_list(control: &Control) -> Option<Ref<ListBox>> {
    control
        .to_ref()
        .cast::<ListBox>()
        .filter(|list| list.classes().contains("thread-list"))
        .or_else(|| {
            control
                .get_visual_descendants()
                .filter_map(|node| node.cast::<ListBox>())
                .find(|list| list.classes().contains("thread-list"))
        })
}

fn anchor_index(list: &ListBox, mut id: u64) -> Option<i32> {
    let tag = list.tag()?;
    let anchors = tag.downcast_ref::<ThreadAnchors>()?;
    let mut seen = BTreeSet::new();
    while seen.insert(id) {
        if id == anchors.root {
            return Some(0);
        }
        if let Some(&index) = anchors.indices.get(&id) {
            return Some(index);
        }
        id = *anchors.parents.get(&id)?;
    }
    None
}

fn scroll_anchor_into_view(list: &ListBox, index: i32) {
    list.scroll_into_view(index);
    if let (Some(container), Some(scroll)) = (
        list.container_from_index(index),
        list.scroll()
            .and_then(|scroll| scroll.cast::<ScrollViewer>()),
    ) {
        // VirtualizingStackPanel uses physical scroll offsets. Align the restored row
        // at the top instead of merely making its final pixel visible at the bottom.
        if let Some(origin) = container.translate_point(Point::new(0.0, 0.0), &scroll) {
            let offset = scroll.offset();
            scroll.set_offset(Vector::new(offset.x, (offset.y + origin.y).max(0.0)));
        }
    }
}

fn header(
    snapshot: &ThreadSnapshot,
    state: &ReaderState,
    sort: ThreadSort,
    actions: &Rc<dyn Fn(ThreadAction)>,
) -> Ref<Control> {
    let story = &snapshot.root;
    let body = StackPanel::new();
    body.set_spacing(14.0);
    body.set_margin(Thickness::new(0.0, 16.0, 0.0, 20.0));
    let eyebrow = label(
        &format!("DISCUSSION  /  {}", story.domain().to_uppercase()),
        10.0,
    );
    eyebrow.classes().add("eyebrow");
    body.children().add(eyebrow);

    let headline = HtmlDocument::parse(story.title_text()).plain_text();
    let title = label(&headline, 28.0);
    title.set_font_weight(FontWeight::SemiBold);
    title.set_line_height(36.0);
    AutomationProperties::set_name(&title, Some(&headline));
    body.children().add(title);

    let meta = WrapPanel::new();
    meta.set_orientation(Orientation::Horizontal);
    if let Some(author) = &story.by {
        meta.children().add(action_button(
            author,
            ThreadAction::Profile(author.clone()),
            actions,
        ));
    }
    let score = story
        .score
        .map(|value| format!("{value} points"))
        .unwrap_or_else(|| "HN post".to_owned());
    let count = story
        .descendants
        .map(|value| format!("{value} comments"))
        .unwrap_or_else(|| format!("{} replies", story.kids.len()));
    let metadata = label(
        &format!(
            "{score}  ·  {count}  ·  {}",
            relative_time(story.time, crate::services::now())
        ),
        12.0,
    );
    metadata.classes().add("muted");
    metadata.set_vertical_alignment(VerticalAlignment::Center);
    metadata.set_margin(Thickness::symmetric(6.0, 4.0));
    meta.children().add(metadata);
    body.children().add(meta);

    let primary = WrapPanel::new();
    if let Some(url) = story.article_url() {
        primary.children().add(action_button(
            "Read article",
            ThreadAction::OpenUrl(url),
            actions,
        ));
    }
    primary.children().add(action_button(
        if state.is_saved(story.id) {
            "Saved"
        } else {
            "Save story"
        },
        ThreadAction::Save,
        actions,
    ));
    primary
        .children()
        .add(action_button("Share link", ThreadAction::Share, actions));
    primary.children().add(action_button(
        "Open on HN",
        ThreadAction::OpenUrl(story.discussion_url()),
        actions,
    ));
    body.children().add(primary);

    if story.deleted {
        body.children()
            .add(muted("This item was deleted on Hacker News."));
    } else if story.dead && !state.preferences.show_dead {
        body.children().add(muted("This item is marked dead on Hacker News. Enable dead items in Reader settings to show its content."));
    } else if let Some(html) = &story.text {
        if !html.trim().is_empty() {
            body.children().add(rich(html, state, actions));
        }
    }
    if !story.parts.is_empty() {
        body.children().add(poll_options(snapshot, actions));
    }
    let secondary = WrapPanel::new();
    secondary.children().add(action_button(
        "Vote on HN",
        ThreadAction::Vote(story.id),
        actions,
    ));
    secondary.children().add(action_button(
        "Reply on HN",
        ThreadAction::Reply(story.id),
        actions,
    ));
    secondary.children().add(action_button(
        if state.items.get(&story.id).is_some_and(|s| s.hidden) {
            "Unhide"
        } else {
            "Hide story"
        },
        ThreadAction::Hide,
        actions,
    ));
    secondary.children().add(action_button(
        "Mute domain",
        ThreadAction::MuteDomain,
        actions,
    ));
    if let Some(parent) = story.parent {
        secondary.children().add(action_button(
            "Parent context",
            ThreadAction::Parent(parent),
            actions,
        ));
    }
    body.children().add(secondary);

    let separator = Border::new();
    separator.set_height(1.0);
    bind_resource(
        &separator,
        Border::background_property().as_property(),
        "EmberBorder",
    );
    body.children().add(separator);

    let sort_row = WrapPanel::new();
    let heading = label("Conversation", 18.0);
    heading.set_font_weight(FontWeight::SemiBold);
    heading.set_margin(Thickness::new(0.0, 8.0, 16.0, 8.0));
    sort_row.children().add(heading);
    let sort_picker = ComboBox::new();
    sort_picker.set_items_source(Some(ItemsSource::from_values([
        "Ranked threads".to_owned(),
        "Newest first".to_owned(),
        "Oldest first".to_owned(),
    ])));
    sort_picker.set_selected_index(match sort {
        ThreadSort::Ranked => 0,
        ThreadSort::Newest => 1,
        ThreadSort::Oldest => 2,
    });
    sort_picker.set_min_height(44.0);
    sort_picker.set_min_width(160.0);
    AutomationProperties::set_name(&sort_picker, Some("Comment sort order"));
    let weak = sort_picker.downgrade();
    let actions = actions.clone();
    sort_picker.selection_changed(move |_, _| {
        if let Some(picker) = weak.upgrade() {
            let value = match picker.selected_index() {
                1 => ThreadSort::Newest,
                2 => ThreadSort::Oldest,
                _ => ThreadSort::Ranked,
            };
            actions(ThreadAction::Sort(value));
        }
    });
    sort_row.children().add(sort_picker);
    body.children().add(sort_row);
    if snapshot.stale {
        body.children().add(muted(
            "Showing a saved copy. Refresh to check for new replies.",
        ));
    }
    body.upcast()
}

fn comment_row(
    row: &ThreadRow,
    document: &OnceCell<HtmlDocument>,
    root: &HnItem,
    state: &ReaderState,
    searching: bool,
    actions: &Rc<dyn Fn(ThreadAction)>,
) -> Ref<Control> {
    let outer = Border::new();
    outer.set_corner_radius(CornerRadius::uniform(10.0));
    outer.set_border_thickness(Thickness::uniform(1.0));
    outer.set_margin(Thickness::new(
        (row.depth.min(4) * 10) as f64,
        0.0,
        0.0,
        0.0,
    ));
    bind_resource(
        &outer,
        Border::background_property().as_property(),
        "EmberCard",
    );
    bind_resource(
        &outer,
        Border::border_brush_property().as_property(),
        if row.matches_query {
            "SystemControlHighlightAccentBrush"
        } else {
            "EmberBorder"
        },
    );
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse("3,*").expect("constant columns"));
    let rail = Border::new();
    rail.set_margin(Thickness::new(0.0, 12.0, 0.0, 12.0));
    let depth_colors = [
        &state.preferences.accent[..],
        "#6CAEE7",
        "#9D8CE8",
        "#5BB7A0",
        "#D7A66E",
    ];
    if let Ok(color) = Color::parse(depth_colors[row.depth % depth_colors.len()]) {
        rail.set_background(Some(SolidColorBrush::with_color(color).into()));
    }
    grid.children().add(rail);
    let body = StackPanel::new();
    body.set_spacing(10.0);
    body.set_margin(Thickness::new(12.0, 8.0, 12.0, 10.0));
    Grid::set_column(&body, 1);
    let meta = WrapPanel::new();
    if let Some(item) = &row.item {
        if !item.deleted {
            if let Some(author) = &item.by {
                let author_button =
                    action_button(author, ThreadAction::Profile(author.clone()), actions);
                author_button.set_font_weight(FontWeight::SemiBold);
                meta.children().add(author_button);
                if item.by == root.by && root.by.is_some() {
                    meta.children().add(badge("AUTHOR"));
                }
            }
        }
        let age = muted(&relative_time(item.time, crate::services::now()));
        age.set_vertical_alignment(VerticalAlignment::Center);
        age.set_margin(Thickness::symmetric(4.0, 8.0));
        meta.children().add(age);
        let was_read = state.items.get(&root.id);
        if was_read
            .is_some_and(|s| !s.seen_comments.is_empty() && !s.seen_comments.contains(&row.id))
        {
            meta.children().add(badge("NEW"));
        }
        if row.matches_query {
            meta.children().add(badge("MATCH"));
        }
    } else {
        meta.children().add(muted(&format!("Comment {}", row.id)));
    }
    body.children().add(meta);

    if row.collapsed && !searching {
        let summary = match &row.item {
            Some(item) if item.deleted => "[deleted]".to_owned(),
            Some(item) if item.dead && !state.preferences.show_dead => "[dead]".to_owned(),
            Some(item) => item
                .text
                .as_deref()
                .map(|text| {
                    document
                        .get_or_init(|| HtmlDocument::parse(text))
                        .plain_text()
                })
                .unwrap_or_else(|| "Collapsed conversation".to_owned()),
            None => "Collapsed conversation".to_owned(),
        };
        let preview: String = summary.chars().take(150).collect();
        body.children().add(muted(&format!(
            "{preview}{}",
            if summary.chars().count() > 150 {
                "…"
            } else {
                ""
            }
        )));
    } else {
        match &row.item {
            None => body.children().add(muted(
                "This comment is not available in the local copy yet.",
            )),
            Some(item) if item.deleted => body
                .children()
                .add(muted("[deleted] — replies remain available below.")),
            Some(item) if item.dead && !state.preferences.show_dead => body.children().add(muted(
                "[dead] — enable dead comments in Reader settings to show this text.",
            )),
            Some(item) => {
                if let Some(html) = item.text.as_deref().filter(|s| !s.trim().is_empty()) {
                    body.children().add(rich_document(
                        document.get_or_init(|| HtmlDocument::parse(html)),
                        state,
                        actions,
                    ));
                } else {
                    body.children().add(muted("No comment text is available."));
                }
            }
        }
    }

    let toolbar = WrapPanel::new();
    if row.child_count > 0 || row.collapsed {
        toolbar.children().add(action_button(
            &format!(
                "{} {} replies",
                if row.collapsed { "Expand" } else { "Collapse" },
                row.child_count
            ),
            ThreadAction::ToggleCollapse(row.id),
            actions,
        ));
    }
    if row.item.is_some() {
        toolbar.children().add(action_button(
            "Reply on HN",
            ThreadAction::Reply(row.id),
            actions,
        ));
        toolbar.children().add(action_button(
            "Permalink",
            ThreadAction::OpenUrl(ember_core::item_url(row.id)),
            actions,
        ));
    } else {
        toolbar.children().add(action_button(
            "Retry loading",
            ThreadAction::LoadMore,
            actions,
        ));
    }
    if row.depth > 0 {
        toolbar.children().add(action_button(
            "Parent",
            ThreadAction::Parent(row.parent_id),
            actions,
        ));
    }
    body.children().add(toolbar);
    AutomationProperties::set_name(
        &outer,
        Some(&format!(
            "Comment by {}, level {}",
            row.item
                .as_ref()
                .map(HnItem::author)
                .unwrap_or("unavailable"),
            row.depth + 1
        )),
    );
    grid.children().add(body);
    outer.set_child(grid);
    outer.upcast()
}

fn poll_options(snapshot: &ThreadSnapshot, actions: &Rc<dyn Fn(ThreadAction)>) -> Ref<Control> {
    let panel = StackPanel::new();
    panel.set_spacing(12.0);
    panel.children().add(muted("POLL RESULTS"));
    let total: i64 = snapshot
        .root
        .parts
        .iter()
        .filter_map(|id| snapshot.items.get(id))
        .map(|item| item.score.unwrap_or(0).max(0))
        .fold(0i64, i64::saturating_add);
    for id in &snapshot.root.parts {
        let Some(option) = snapshot.items.get(id) else {
            panel
                .children()
                .add(muted("A poll option is still loading."));
            continue;
        };
        let item = StackPanel::new();
        item.set_spacing(6.0);
        let text = HtmlDocument::parse(
            option
                .text
                .as_deref()
                .or(option.title.as_deref())
                .unwrap_or("Poll option"),
        )
        .plain_text();
        item.children().add(label(
            &format!("{}  ·  {} votes", text, option.score.unwrap_or(0)),
            14.0,
        ));
        let progress = ProgressBar::new();
        progress.set_maximum(total.max(1) as f64);
        progress.set_range_value(option.score.unwrap_or(0).max(0) as f64);
        progress.set_height(5.0);
        AutomationProperties::set_name(&progress, Some(&text));
        item.children().add(progress);
        panel.children().add(item);
    }
    panel.children().add(action_button(
        "Vote in poll on HN",
        ThreadAction::OpenUrl(snapshot.root.discussion_url()),
        actions,
    ));
    panel.upcast()
}

fn footer(snapshot: &ThreadSnapshot, actions: &Rc<dyn Fn(ThreadAction)>) -> Ref<Control> {
    let panel = StackPanel::new();
    panel.set_spacing(10.0);
    panel.set_margin(Thickness::uniform(12.0));
    panel.children().add(muted(&format!(
        "{} comments loaded · {} pending · {} unavailable",
        snapshot
            .items
            .values()
            .filter(|item| item.id != snapshot.root.id && !snapshot.root.parts.contains(&item.id))
            .count(),
        snapshot.pending_ids.len(),
        snapshot.missing_ids.len()
    )));
    if let Some(warning) = &snapshot.warning {
        panel.children().add(muted(warning));
    }
    if !snapshot.complete {
        panel.children().add(action_button(
            "Load more comments",
            ThreadAction::LoadMore,
            actions,
        ));
    }
    panel.upcast()
}

fn empty(
    snapshot: &ThreadSnapshot,
    query: &str,
    actions: &Rc<dyn Fn(ThreadAction)>,
) -> Ref<Control> {
    let panel = StackPanel::new();
    panel.set_spacing(12.0);
    panel.set_margin(Thickness::uniform(20.0));
    let text = if query.is_empty() && !snapshot.complete {
        "Comments are not yet available in the local copy.".to_owned()
    } else if query.is_empty() {
        "A good conversation starts with one thought.".to_owned()
    } else {
        format!("No loaded comments match “{}”.", query)
    };
    panel.children().add(label(&text, 18.0));
    panel.children().add(action_button(
        "Join the conversation on HN",
        ThreadAction::Reply(snapshot.root.id),
        actions,
    ));
    panel.upcast()
}

fn rich(html: &str, state: &ReaderState, actions: &Rc<dyn Fn(ThreadAction)>) -> Ref<Control> {
    let document = HtmlDocument::parse(html);
    rich_document(&document, state, actions)
}

fn rich_document(
    document: &HtmlDocument,
    state: &ReaderState,
    actions: &Rc<dyn Fn(ThreadAction)>,
) -> Ref<Control> {
    let options = render_options(state);
    let actions = actions.clone();
    ember_content::render(
        document,
        &options,
        Rc::new(move |url| actions(ThreadAction::OpenUrl(url.to_owned()))),
    )
}

/// Shared typography, palette and touch metrics for comments, story bodies and biographies.
pub fn render_options(state: &ReaderState) -> RenderOptions {
    RenderOptions {
        font_size: state.preferences.font_size,
        line_height: state.preferences.font_size * state.preferences.line_height,
        foreground: brush("EmberText"),
        link_brush: brush("SystemControlHighlightAccentBrush"),
        code_background: brush("EmberCode"),
        quote_brush: brush("EmberMuted"),
        link_min_height: 44.0,
        max_link_width: 260.0,
        ..Default::default()
    }
}

fn action_button(
    text: &str,
    action: ThreadAction,
    actions: &Rc<dyn Fn(ThreadAction)>,
) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Control::boxed(label(text, 12.0))));
    button.set_min_height(44.0);
    button.set_padding(Thickness::symmetric(10.0, 8.0));
    button.set_margin(Thickness::new(0.0, 0.0, 6.0, 6.0));
    button.set_font_size(12.0);
    button.set_max_width(260.0);
    button.set_horizontal_alignment(HorizontalAlignment::Left);
    AutomationProperties::set_name(&button, Some(text));
    let actions = actions.clone();
    button.click(move |_, _| actions(action.clone()));
    button
}

fn label(text: &str, size: f64) -> Ref<TextBlock> {
    let block = TextBlock::new();
    block.set_text(Some(text));
    block.set_font_size(size);
    block.set_text_wrapping(TextWrapping::Wrap);
    block
}

fn muted(text: &str) -> Ref<TextBlock> {
    let block = label(text, 12.0);
    block.classes().add("muted");
    block
}

fn badge(text: &str) -> Ref<Border> {
    let border = Border::new();
    border.set_corner_radius(CornerRadius::uniform(5.0));
    border.set_padding(Thickness::symmetric(6.0, 3.0));
    border.set_margin(Thickness::symmetric(4.0, 10.0));
    border.set_vertical_alignment(VerticalAlignment::Center);
    bind_resource(
        &border,
        Border::background_property().as_property(),
        "EmberTint",
    );
    let label = label(text, 9.0);
    label.set_font_weight(FontWeight::SemiBold);
    bind_resource(
        &label,
        TextBlock::foreground_property().as_property(),
        "SystemControlHighlightAccentBrush",
    );
    border.set_child(label);
    border
}

fn bind_resource(target: &FerroObject, property: &'static FerroProperty, key: &str) {
    let resource =
        DynamicResourceExtension::with_resource_key(Some(Rc::new(key.to_owned()) as BoxedValue));
    target.bind_binding(property, &*resource.as_binding());
}

fn brush(key: &str) -> Option<Rc<dyn IBrush>> {
    let app = Application::current()?;
    let value = app
        .try_get_resource(&ResourceKey::from(key), app.actual_theme_variant().as_ref())
        .flatten();
    from_markup_value(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::Window;
    use ferroui_controls::testing::UnitTestApplication;
    use sample_testing::{Shell, TestGlobalClock};
    use std::cell::RefCell;

    #[test]
    fn collapsed_deleted_and_dead_comments_do_not_expose_retained_html() {
        let clock = Rc::new(TestGlobalClock::default());
        let _application = UnitTestApplication::start(sample_testing::services(clock));
        let root = HnItem {
            id: 1000,
            ..Default::default()
        };
        let state = ReaderState::default();
        let actions: Rc<dyn Fn(ThreadAction)> = Rc::new(|_| {});
        for (deleted, dead, placeholder) in [(true, false, "[deleted]"), (false, true, "[dead]")] {
            let row = ThreadRow {
                id: 2001,
                parent_id: root.id,
                depth: 0,
                item: Some(HnItem {
                    id: 2001,
                    text: Some("<p>retained source must stay hidden</p>".into()),
                    deleted,
                    dead,
                    ..Default::default()
                }),
                collapsed: true,
                child_count: 1,
                matches_query: false,
            };
            let document = OnceCell::new();
            let control = comment_row(&row, &document, &root, &state, false, &actions);
            let text = control
                .get_logical_descendants()
                .filter_map(|node| node.cast::<TextBlock>())
                .filter_map(|block| block.text())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains(placeholder));
            assert!(!text.contains("retained source"));
            assert!(document.get().is_none(), "hidden HTML must not be parsed");
        }
    }

    #[test]
    fn long_threads_virtualize_rows_and_keep_scrolled_actions_attached_to_their_item() {
        let shell = Shell::start(&crate::SAMPLE, 390.0, 620.0, || crate::App::new().upcast());
        let root = HnItem {
            id: 1000,
            title: Some("A long conversation".into()),
            kids: (2001..=2200).collect(),
            ..Default::default()
        };
        let snapshot = ThreadSnapshot {
            items: root
                .kids
                .iter()
                .map(|&id| {
                    (
                        id,
                        HnItem {
                            id,
                            parent: Some(root.id),
                            by: Some(format!("author{id}")),
                            text: Some(format!("<p>Comment {id} has <b>native</b> content.</p>")),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
            root,
            complete: true,
            ..Default::default()
        };
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = calls.clone();
        let content = build(
            &snapshot,
            &ReaderState::default(),
            "",
            ThreadSort::Ranked,
            Rc::new(move |action| observed.borrow_mut().push(action)),
        );
        let list = content.cast::<ListBox>().unwrap();
        restore_anchor(&content, 2020);
        let window = Window::new();
        window.set_width(390.0);
        window.set_height(620.0);
        window.set_content(Some(Control::boxed(&content)));
        window.show();
        for _ in 0..4 {
            shell.settle();
        }
        assert_eq!(list.items().count(), 201);
        assert_eq!(viewport_anchor(&content), Some(2020));
        for index in [20, 170, 10] {
            restore_anchor(&content, 2000 + index as u64);
            for _ in 0..4 {
                shell.settle();
            }
            assert_eq!(viewport_anchor(&content), Some(2000 + index as u64));
            assert!(
                list.get_realized_containers().len() < 50,
                "a phone viewport must not realize the entire conversation"
            );
            let row = list
                .container_from_index(index)
                .expect("requested comment should be realized");
            let permalink = row
                .get_visual_descendants()
                .filter_map(|node| node.cast::<Button>())
                .find(|button| {
                    AutomationProperties::get_name(button).as_deref() == Some("Permalink")
                })
                .expect("realized comment has an accessible permalink action");
            permalink.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            let expected = ember_core::item_url(2000 + index as u64);
            assert!(
                matches!(calls.borrow().last(), Some(ThreadAction::OpenUrl(url)) if url == &expected)
            );
        }
        assert_eq!(calls.borrow().len(), 3);
        assert!(shell.take_binding_reports().is_empty());
        window.close();
    }
}
