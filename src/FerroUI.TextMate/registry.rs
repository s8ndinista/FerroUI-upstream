use crate::{grammar::Program, Grammar, RawGrammar, Result, Theme};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

/// Owns grammar sources and compiled registries. Replacing a source invalidates
/// future loads; existing grammar handles retain their immutable program.
#[derive(Default)]
pub struct Registry {
    grammars: BTreeMap<String, RawGrammar>,
    program: RefCell<Option<Rc<Program>>>,
    compiled: RefCell<BTreeMap<String, Grammar>>,
    theme: Theme,
}
impl Registry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_grammar_json(&mut self, source: &str) -> Result<()> {
        self.add_grammar(RawGrammar::from_json(source)?);
        Ok(())
    }
    pub fn add_grammar(&mut self, grammar: RawGrammar) {
        self.grammars.insert(grammar.scope_name.clone(), grammar);
        *self.program.get_mut() = None;
        self.compiled.get_mut().clear();
    }
    pub fn scope_names(&self) -> impl Iterator<Item = &str> {
        self.grammars.keys().map(String::as_str)
    }
    pub fn load_grammar(&self, scope: &str) -> Result<Grammar> {
        if let Some(grammar) = self.compiled.borrow().get(scope) {
            return Ok(grammar.clone());
        }
        let program = self.program.borrow().clone();
        let program = match program {
            Some(program) => program,
            None => {
                let program = Rc::new(Program::compile(&self.grammars)?);
                *self.program.borrow_mut() = Some(program.clone());
                program
            }
        };
        let grammar = Grammar::new(program, scope)?;
        self.compiled
            .borrow_mut()
            .insert(scope.into(), grammar.clone());
        Ok(grammar)
    }
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
    pub fn theme(&self) -> &Theme {
        &self.theme
    }
}
