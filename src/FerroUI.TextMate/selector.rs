use crate::{Error, Result};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectorPriority {
    Left,
    #[default]
    Normal,
    Right,
}

#[derive(Clone, Debug)]
enum Expr {
    Path(Vec<String>),
    All(Vec<Expr>),
    Any(Vec<Expr>),
    Not(Box<Expr>),
}

/// TextMate scope selector: ancestry, alternatives, grouped expressions and exclusions.
#[derive(Clone, Debug)]
pub struct ScopeSelector {
    expression: Expr,
    pub priority: SelectorPriority,
}

impl ScopeSelector {
    pub fn parse(value: &str) -> Result<Self> {
        let mut source = value.trim();
        let priority = if let Some(rest) = source.strip_prefix("L:") {
            source = rest;
            SelectorPriority::Left
        } else if let Some(rest) = source.strip_prefix("R:") {
            source = rest;
            SelectorPriority::Right
        } else {
            SelectorPriority::Normal
        };
        let mut parser = Parser {
            tokens: lex(source),
            pos: 0,
        };
        let expression = parser.alternative()?;
        if parser.pos != parser.tokens.len() {
            return Err(Error::InvalidSelector(value.into()));
        }
        Ok(Self {
            expression,
            priority,
        })
    }

    pub fn matches<S: AsRef<str>>(&self, scopes: &[S]) -> bool {
        self.specificity(scopes).is_some()
    }

    pub(crate) fn specificity<S: AsRef<str>>(&self, scopes: &[S]) -> Option<Vec<usize>> {
        self.expression.rank(scopes)
    }
}

impl Expr {
    fn rank<S: AsRef<str>>(&self, scopes: &[S]) -> Option<Vec<usize>> {
        match self {
            Self::Path(names) => {
                if names.is_empty() {
                    return Some(vec![0]);
                }
                let mut cursor = scopes.len();
                let mut rank = Vec::new();
                for name in names.iter().rev() {
                    let found = (0..cursor)
                        .rev()
                        .find(|&i| matches_name(name, scopes[i].as_ref()))?;
                    rank.push(found + 1);
                    rank.push(name.len());
                    cursor = found;
                }
                Some(rank)
            }
            Self::Any(items) => items.iter().filter_map(|item| item.rank(scopes)).max(),
            Self::All(items) => {
                let mut best = vec![0];
                for item in items {
                    best = best.max(item.rank(scopes)?);
                }
                Some(best)
            }
            Self::Not(item) => {
                if item.rank(scopes).is_some() {
                    None
                } else {
                    Some(vec![0])
                }
            }
        }
    }
}

fn matches_name(pattern: &str, scope: &str) -> bool {
    pattern == "*"
        || scope == pattern
        || scope
            .strip_prefix(pattern)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn lex(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    for c in source.chars() {
        if c.is_whitespace() || matches!(c, '(' | ')' | ',' | '|') {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            if !c.is_whitespace() {
                out.push(c.to_string());
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

struct Parser {
    tokens: Vec<String>,
    pos: usize,
}
impl Parser {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(String::as_str)
    }
    fn alternative(&mut self) -> Result<Expr> {
        let mut alternatives = vec![self.conjunction()?];
        while matches!(self.peek(), Some("," | "|")) {
            self.pos += 1;
            alternatives.push(self.conjunction()?);
        }
        Ok(if alternatives.len() == 1 {
            alternatives.remove(0)
        } else {
            Expr::Any(alternatives)
        })
    }
    fn conjunction(&mut self) -> Result<Expr> {
        let mut items = Vec::new();
        let mut names = Vec::new();
        while let Some(token) = self.peek() {
            if matches!(token, "," | "|" | ")") {
                break;
            }
            if matches!(token, "(" | "-") {
                if !names.is_empty() {
                    items.push(Expr::Path(std::mem::take(&mut names)));
                }
                let negate = token == "-";
                if negate {
                    self.pos += 1;
                }
                let expr = if self.peek() == Some("(") {
                    self.pos += 1;
                    let expr = self.alternative()?;
                    if self.peek() != Some(")") {
                        return Err(Error::InvalidSelector("missing ')'".into()));
                    }
                    self.pos += 1;
                    expr
                } else {
                    let token = self
                        .peek()
                        .ok_or_else(|| Error::InvalidSelector("missing exclusion".into()))?
                        .to_owned();
                    self.pos += 1;
                    Expr::Path(vec![token])
                };
                items.push(if negate {
                    Expr::Not(Box::new(expr))
                } else {
                    expr
                });
            } else {
                names.push(token.to_owned());
                self.pos += 1;
            }
        }
        if !names.is_empty() {
            items.push(Expr::Path(names));
        }
        Ok(if items.is_empty() {
            Expr::Path(Vec::new())
        } else if items.len() == 1 {
            items.remove(0)
        } else {
            Expr::All(items)
        })
    }
}
