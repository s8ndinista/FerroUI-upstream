use super::{HighlightingDefinition, HighlightingError};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

/// Lazily compiles embedded and application-provided XSHD definitions. Only the
/// transitive dependencies of the requested language are compiled.
pub struct HighlightingManager {
    sources: BTreeMap<String, Rc<str>>,
    extensions: HashMap<String, String>,
    cache: RefCell<HashMap<String, Rc<HighlightingDefinition>>>,
}
impl Default for HighlightingManager {
    fn default() -> Self {
        Self::new()
    }
}
impl HighlightingManager {
    pub fn new() -> Self {
        let mut manager = Self::empty();
        for source in BUILTINS {
            manager
                .register_highlighting(source)
                .expect("embedded highlighting metadata is validated by tests");
        }
        manager
    }
    pub fn empty() -> Self {
        Self {
            sources: BTreeMap::new(),
            extensions: HashMap::new(),
            cache: RefCell::new(HashMap::new()),
        }
    }
    pub fn highlighting_names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.sources.keys().map(String::as_str)
    }
    pub fn register_highlighting(&mut self, source: &str) -> Result<String, HighlightingError> {
        let document = roxmltree::Document::parse(source)
            .map_err(|e| HighlightingError::Xml(e.to_string()))?;
        let root = document.root_element();
        let name = root
            .attribute("name")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                HighlightingError::InvalidDefinition("Registered definitions need a name".into())
            })?
            .to_owned();
        self.extensions.retain(|_, value| value != &name);
        for extension in root
            .attribute("extensions")
            .unwrap_or("")
            .split(';')
            .filter(|s| !s.is_empty())
        {
            self.extensions
                .insert(extension.to_lowercase(), name.clone());
        }
        self.sources.insert(name.clone(), Rc::from(source));
        self.cache.borrow_mut().clear();
        Ok(name)
    }
    pub fn get_definition(
        &self,
        name: &str,
    ) -> Result<Option<Rc<HighlightingDefinition>>, HighlightingError> {
        if let Some(cached) = self.cache.borrow().get(name).cloned() {
            return Ok(Some(cached));
        }
        let Some(source) = self.sources.get(name) else {
            return Ok(None);
        };
        let mut visited = BTreeSet::new();
        let mut pending = vec![name.to_owned()];
        let mut dependencies = Vec::new();
        while let Some(current) = pending.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            let current_source = self
                .sources
                .get(&current)
                .ok_or_else(|| HighlightingError::Reference(current.clone()))?;
            let document = roxmltree::Document::parse(current_source)
                .map_err(|e| HighlightingError::Xml(e.to_string()))?;
            for node in document.descendants().filter(|n| n.is_element()) {
                for key in ["ruleSet", "color"] {
                    if let Some((dependency, _)) =
                        node.attribute(key).and_then(|s| s.rsplit_once('/'))
                    {
                        if !visited.contains(dependency) {
                            pending.push(dependency.to_owned());
                        }
                    }
                }
            }
            if current != name {
                dependencies.push(current_source.as_ref());
            }
        }
        let definition = Rc::new(HighlightingDefinition::load_with_dependencies(
            source,
            &dependencies,
        )?);
        self.cache
            .borrow_mut()
            .insert(name.to_owned(), definition.clone());
        Ok(Some(definition))
    }
    pub fn get_definition_by_extension(
        &self,
        extension: &str,
    ) -> Result<Option<Rc<HighlightingDefinition>>, HighlightingError> {
        let normalized = if extension.starts_with('.') {
            extension.to_lowercase()
        } else if let Some((_, ext)) = extension.rsplit_once('.') {
            format!(".{}", ext.to_lowercase())
        } else {
            format!(".{}", extension.to_lowercase())
        };
        match self.extensions.get(&normalized) {
            Some(name) => self.get_definition(name),
            None => Ok(None),
        }
    }
}

const BUILTINS: &[&str] = &[
    include_str!("resources/ASPX.xshd"),
    include_str!("resources/Boo.xshd"),
    include_str!("resources/Coco-Mode.xshd"),
    include_str!("resources/CPP-Mode.xshd"),
    include_str!("resources/CSharp-Mode.xshd"),
    include_str!("resources/CSS-Mode.xshd"),
    include_str!("resources/HTML-Mode.xshd"),
    include_str!("resources/Java-Mode.xshd"),
    include_str!("resources/JavaScript-Mode.xshd"),
    include_str!("resources/Json.xshd"),
    include_str!("resources/MarkDown-Mode.xshd"),
    include_str!("resources/MarkDownWithFontSize-Mode.xshd"),
    include_str!("resources/Patch-Mode.xshd"),
    include_str!("resources/PHP-Mode.xshd"),
    include_str!("resources/PowerShell.xshd"),
    include_str!("resources/Python-Mode.xshd"),
    include_str!("resources/Tex-Mode.xshd"),
    include_str!("resources/TSQL-Mode.xshd"),
    include_str!("resources/VB-Mode.xshd"),
    include_str!("resources/XML-Mode.xshd"),
    include_str!("resources/XmlDoc.xshd"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlighting::{HighlightingEngine, HighlightingState};
    #[test]
    fn all_bundled_definitions_compile_and_execute() {
        let manager = HighlightingManager::new();
        let mut failures = Vec::new();
        for name in manager.highlighting_names() {
            match manager.get_definition(name) {
                Ok(Some(definition)) => {
                    let engine = HighlightingEngine::new(definition);
                    for line in [
                        "",
                        "// comment",
                        "let value = \"😀\";",
                        "<tag a=\"x\">",
                        "/* comment */",
                    ] {
                        if let Err(error) =
                            engine.highlight_line(line, 1, 0, &HighlightingState::default())
                        {
                            failures.push(format!("{name}: {error}"));
                        }
                    }
                }
                Ok(None) => failures.push(format!("{name}: missing")),
                Err(error) => failures.push(format!("{name}: {error}")),
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        assert_eq!(manager.highlighting_names().len(), 21);
    }
}
