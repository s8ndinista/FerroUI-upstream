//! Rebase paragraph styles for provisional IME text only. The application owns
//! committed ranges; this projection never mutates its immutable collection.
use ferroui_base::media::text_formatting::TextParagraphStyles;
use ferroui_base::utilities::ValueSpan;
use std::ops::Range;
use std::rc::Rc;

fn paragraphs(text: &[u16]) -> Vec<Range<usize>> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < text.len() {
        let unit = text[cursor];
        cursor += 1;
        if !matches!(unit, 10 | 13 | 0x85 | 0x2028 | 0x2029) { continue; }
        if unit == 13 && text.get(cursor) == Some(&10) { cursor += 1; }
        output.push(start..cursor);
        start = cursor;
    }
    output.push(start..text.len());
    output
}

pub(super) fn compose(styles: Option<TextParagraphStyles>, original: &[u16], visible: &[u16]) -> Option<TextParagraphStyles> {
    let styles = styles?;
    if original == visible { return Some(styles); }
    let mut prefix = original.iter().zip(visible).take_while(|(a, b)| a == b).count();
    // A changed Unicode scalar cannot be divided at its shared high surrogate.
    if prefix > 0 && original.get(prefix).is_some_and(|unit| (0xdc00..=0xdfff).contains(unit)) { prefix -= 1; }
    let mut suffix = original[prefix..].iter().rev().zip(visible[prefix..].iter().rev())
        .take_while(|(a, b)| a == b).count();
    if suffix > 0 && original.get(original.len() - suffix).is_some_and(|unit| (0xdc00..=0xdfff).contains(unit)) { suffix -= 1; }
    let old_end = original.len() - suffix;
    let new_end = visible.len() - suffix;
    let original_paragraphs = paragraphs(original);
    let mut result = Vec::new();
    for paragraph in paragraphs(visible) {
        let source = if paragraph.start <= prefix { paragraph.start }
            else if paragraph.start >= new_end { old_end + paragraph.start - new_end }
            else { prefix };
        let index = original_paragraphs.partition_point(|range| range.start <= source).saturating_sub(1);
        let start = original_paragraphs[index].start as i32;
        if let Some(style) = styles.iter().rev().find(|style| style.start() <= start
            && (style.start() == start || start < style.start().saturating_add(style.length()))) {
            result.push(ValueSpan::new(paragraph.start as i32, (paragraph.end - paragraph.start) as i32, style.value().clone()));
        }
    }
    Some(Rc::from(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::FlowDirection;
    use ferroui_base::media::text_formatting::TextParagraphStyle;

    #[test]
    fn provisional_paragraph_splits_and_merges_preserve_utf16_styles_without_mutation() {
        let before: Vec<_> = "A🦀\r\nשלום\nlast".encode_utf16().collect();
        let styles: TextParagraphStyles = Rc::from(vec![
            ValueSpan::new(0, 5, TextParagraphStyle { indent: 12.0, ..Default::default() }),
            ValueSpan::new(5, 5, TextParagraphStyle { flow_direction: Some(FlowDirection::RightToLeft), ..Default::default() }),
        ]);
        let split: Vec<_> = "A🦀\n\r\nשלום\nlast".encode_utf16().collect();
        let actual = compose(Some(styles.clone()), &before, &split).unwrap();
        assert_eq!(actual.len(), 3);
        assert_eq!(actual.iter().map(|style| (style.start(), style.length())).collect::<Vec<_>>(), vec![(0,4),(4,2),(6,5)]);
        assert_eq!(actual[1].value().indent, 12.0);
        assert_eq!(actual[2].value().flow_direction, Some(FlowDirection::RightToLeft));
        let joined: Vec<_> = "A🦀שלום\nlast".encode_utf16().collect();
        let joined = compose(Some(styles.clone()), &before, &joined).unwrap();
        assert_eq!(joined.len(), 1);
        assert_eq!(joined[0].value().indent, 12.0);
        assert_eq!((styles[1].start(), styles[1].length()), (5,5));
        assert!(Rc::ptr_eq(&compose(Some(styles.clone()), &before, &before).unwrap(), &styles));
    }
}
