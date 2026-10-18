//! Text replacement keeps rich-text ranges in UTF-8 document coordinates.
use crate::{Node, NodeKind, TextRun};

impl Node {
    /// Replace a text layer's content while preserving the styles of unchanged
    /// characters. The smallest changed character range inherits the style at
    /// the selection start; typing at a boundary inherits the preceding run.
    /// This also keeps IME replacement and non-BMP text on valid byte boundaries.
    pub fn replace_text_preserving_runs(&mut self, text: String) -> bool {
        if self.kind != NodeKind::Text || self.text == text {
            return false;
        }
        let prefix = self
            .text
            .chars()
            .zip(text.chars())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        let suffix = self.text[prefix..]
            .chars()
            .rev()
            .zip(text[prefix..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        let old_end = self.text.len() - suffix;
        let new_end = text.len() - suffix;
        let mut runs = Vec::with_capacity(self.text_runs.len() + 2);
        for run in &self.text_runs {
            if run.start >= run.end
                || run.end > self.text.len()
                || !self.text.is_char_boundary(run.start)
                || !self.text.is_char_boundary(run.end)
            {
                continue;
            }
            let mut parts = Vec::with_capacity(3);
            if run.start < prefix {
                push_part(&mut parts, run, run.start, run.end.min(prefix));
            }
            let inherits = if old_end > prefix {
                run.start <= prefix && run.end > prefix
            } else {
                (run.start < prefix && run.end >= prefix) || (prefix == 0 && run.start == 0)
            };
            if inherits {
                push_part(&mut parts, run, prefix, new_end);
            }
            if run.end > old_end {
                let start = run.start.max(old_end) - old_end + new_end;
                let end = run.end - old_end + new_end;
                push_part(&mut parts, run, start, end);
            }
            runs.extend(parts);
        }
        self.text = text;
        self.text_runs = runs;
        true
    }
}

fn push_part(parts: &mut Vec<TextRun>, template: &TextRun, start: usize, end: usize) {
    if start >= end {
        return;
    }
    if let Some(last) = parts.last_mut() {
        if last.end == start {
            last.end = end;
            return;
        }
    }
    let mut run = template.clone();
    run.start = start;
    run.end = end;
    parts.push(run);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(text: &str, ranges: &[(usize, usize)]) -> Node {
        let mut n = Node::new(NodeKind::Text, "Text", 0.0, 0.0, 100.0, 30.0);
        n.text = text.into();
        n.text_runs = ranges
            .iter()
            .enumerate()
            .map(|(i, &(start, end))| TextRun {
                start,
                end,
                font_family: None,
                font_size: None,
                font_weight: Some(400 + i as u16 * 100),
                color: None,
                italic: None,
                decoration: None,
            })
            .collect();
        n
    }
    fn ranges(n: &Node) -> Vec<(usize, usize)> {
        assert!(n.text_runs.iter().all(|r| r.start < r.end
            && r.end <= n.text.len()
            && n.text.is_char_boundary(r.start)
            && n.text.is_char_boundary(r.end)));
        n.text_runs.iter().map(|r| (r.start, r.end)).collect()
    }
    #[test]
    fn unicode_insertion_inherits_previous_run_and_shifts_suffix() {
        let mut n = node("A🦀B", &[(0, 5), (5, 6)]);
        n.replace_text_preserving_runs("A🦀éB".into());
        assert_eq!(ranges(&n), [(0, 7), (7, 8)]);
        assert_eq!(n.text_runs[1].font_weight, Some(500));
    }
    #[test]
    fn replacement_preserves_unaffected_styles() {
        let mut n = node("abcDEFghi", &[(0, 3), (3, 6), (6, 9)]);
        n.replace_text_preserving_runs("abc🦀ghi".into());
        assert_eq!(ranges(&n), [(0, 3), (3, 7), (7, 10)]);
    }
    #[test]
    fn deletion_across_runs_leaves_only_surviving_ranges() {
        let mut n = node("abcDEFghi", &[(0, 3), (3, 6), (6, 9)]);
        n.replace_text_preserving_runs("ai".into());
        assert_eq!(ranges(&n), [(0, 1), (1, 2)]);
        assert_eq!(n.text_runs[1].font_weight, Some(600));
    }
    #[test]
    fn combining_character_and_ime_replacement_are_valid() {
        let mut n = node("e\u{301}日本語", &[(0, 12)]);
        n.replace_text_preserving_runs("e\u{301}東京".into());
        assert_eq!(ranges(&n), [(0, 9)]);
    }
    #[test]
    fn entire_replacement_and_empty_text_do_not_leave_invalid_runs() {
        let mut n = node("🦀", &[(0, 4)]);
        n.replace_text_preserving_runs("Hi".into());
        assert_eq!(ranges(&n), [(0, 2)]);
        n.replace_text_preserving_runs(String::new());
        assert!(ranges(&n).is_empty());
    }
    #[test]
    fn suffix_does_not_overlap_common_prefix() {
        let mut n = node("aaaa", &[(0, 4)]);
        n.replace_text_preserving_runs("aaa".into());
        assert_eq!(ranges(&n), [(0, 3)]);
        assert!(!n.replace_text_preserving_runs("aaa".into()));
    }
}
