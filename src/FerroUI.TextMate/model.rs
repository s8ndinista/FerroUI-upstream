use crate::{Error, Grammar, Result, StateStack, Token, TokenizeOptions};
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct LineTokens {
    pub tokens: Vec<Token>,
    pub end_state: StateStack,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenizationUpdate {
    /// Half-open zero-based range whose tokenization was recomputed.
    pub lines: Range<usize>,
    pub pending: bool,
    /// Number of regex transitions/searches is bounded separately per line.
    pub stopped_early: bool,
}

/// Incremental line cache. Edits preserve untouched suffix entries; tokenization
/// stops when the recomputed state reaches the old suffix state.
pub struct TokenizationModel {
    grammar: Grammar,
    lines: Vec<String>,
    cache: Vec<Option<LineTokens>>,
    invalid_from: Option<usize>,
    invalid_until: usize,
}
impl TokenizationModel {
    pub fn new(grammar: Grammar) -> Self {
        Self {
            grammar,
            lines: vec![String::new()],
            cache: vec![None],
            invalid_from: Some(0),
            invalid_until: 1,
        }
    }
    pub fn grammar(&self) -> &Grammar {
        &self.grammar
    }
    pub fn set_grammar(&mut self, grammar: Grammar) {
        self.grammar = grammar;
        self.cache.fill(None);
        self.invalid_from = Some(0);
        self.invalid_until = self.lines.len();
    }
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(String::as_str)
    }
    pub fn tokens(&self, index: usize) -> Option<&LineTokens> {
        if self.invalid_from.is_some_and(|first| index >= first) {
            return None;
        }
        self.cache.get(index).and_then(Option::as_ref)
    }
    pub fn set_text(&mut self, text: &str) {
        let mut lines = Vec::new();
        let mut start = 0;
        let bytes = text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if matches!(bytes[index], b'\r' | b'\n') {
                lines.push(text[start..index].into());
                if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
                start = index + 1;
            }
            index += 1;
        }
        lines.push(text[start..].into());
        self.lines = lines;
        self.cache = vec![None; self.lines.len()];
        self.invalid_from = Some(0);
        self.invalid_until = self.lines.len();
    }
    pub fn replace_lines(
        &mut self,
        range: Range<usize>,
        replacement: impl IntoIterator<Item = String>,
    ) -> Result<()> {
        if range.start > range.end || range.end > self.lines.len() {
            return Err(Error::InvalidLine(range.end));
        }
        let replacement: Vec<_> = replacement.into_iter().collect();
        let inserted = replacement.len();
        let removed = range.len();
        self.lines.splice(range.clone(), replacement);
        self.cache
            .splice(range.clone(), std::iter::repeat_n(None, inserted));
        let old_until = if self.invalid_until >= range.end {
            self.invalid_until - removed + inserted
        } else {
            self.invalid_until.min(range.start)
        };
        if self.lines.is_empty() {
            self.lines.push(String::new());
            self.cache.push(None);
        }
        let first = range.start.min(self.lines.len() - 1);
        self.invalid_from = Some(self.invalid_from.map_or(first, |old| old.min(first)));
        self.invalid_until = old_until.max(range.start + inserted).min(self.lines.len());
        Ok(())
    }
    pub fn retokenize(&mut self, max_lines: usize) -> Result<TokenizationUpdate> {
        self.retokenize_with_options(max_lines, TokenizeOptions::default(), None)
    }
    pub fn retokenize_with_options(
        &mut self,
        max_lines: usize,
        options: TokenizeOptions,
        cancelled: Option<&dyn Fn() -> bool>,
    ) -> Result<TokenizationUpdate> {
        let Some(start) = self.invalid_from else {
            return Ok(TokenizationUpdate {
                lines: 0..0,
                pending: false,
                stopped_early: false,
            });
        };
        let mut index = start;
        let mut previous = if start == 0 {
            self.grammar.initial_state()
        } else {
            self.cache[start - 1]
                .as_ref()
                .ok_or(Error::InvalidState)?
                .end_state
                .clone()
        };
        let mut stopped_early = false;
        while index < self.lines.len() && index - start < max_lines {
            let result = self.grammar.tokenize_line_with_options(
                &self.lines[index],
                Some(&previous),
                options,
                cancelled,
            )?;
            if result.stopped_early {
                stopped_early = true;
                break;
            }
            let converged = index >= self.invalid_until
                && self.cache[index].as_ref().is_some_and(|old| {
                    old.end_state == result.rule_stack && old.tokens == result.tokens
                });
            previous = result.rule_stack.clone();
            self.cache[index] = Some(LineTokens {
                tokens: result.tokens,
                end_state: result.rule_stack,
            });
            index += 1;
            if converged {
                self.invalid_from = None;
                self.invalid_until = 0;
                return Ok(TokenizationUpdate {
                    lines: start..index,
                    pending: false,
                    stopped_early: false,
                });
            }
        }
        self.invalid_from = (index < self.lines.len()).then_some(index);
        if self.invalid_from.is_none() {
            self.invalid_until = 0;
        }
        Ok(TokenizationUpdate {
            lines: start..index,
            pending: self.invalid_from.is_some(),
            stopped_early,
        })
    }
}
