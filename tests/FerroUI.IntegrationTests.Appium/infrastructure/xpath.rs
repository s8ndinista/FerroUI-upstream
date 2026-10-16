//! The XPath of upstream's tests over the tree of the platform accessibility
//! API, as the automation drivers evaluate it: the tree below the root of the
//! search is a document whose root element is that root (the application or
//! the current window of a session, the element of an element), and a relative
//! path starts at that root element.
//!
//! The subset is what the tests use: absolute and relative location paths,
//! `/` and `//`, the axes `child`, `parent`, `self`, `descendant`,
//! `descendant-or-self` (with `.` and `..`), the node tests `*`, a name and
//! `node()`, and the predicates `[n]`, `[@attribute]` and
//! `[@attribute='value']`. Anything else is an error of the expression.

use std::collections::BTreeSet;

/// What the evaluation reads of the tree.
pub trait Tree {
    type Node: Clone;
    fn children(&self, node: &Self::Node) -> Vec<Self::Node>;
    /// The name of the element in the document: the control type on
    /// Windows, the `XCUIElementType` of the role on macOS.
    fn tag_name(&self, node: &Self::Node) -> String;
    fn attribute(&self, node: &Self::Node, name: &str) -> Option<String>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Axis {
    Child,
    Parent,
    SelfAxis,
    Descendant,
    DescendantOrSelf,
}

#[derive(Clone, Debug, PartialEq)]
enum NodeTest {
    Any,
    Node,
    Name(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Predicate {
    Position(usize),
    HasAttribute(String),
    AttributeEquals(String, String),
}

#[derive(Clone, Debug, PartialEq)]
struct Step {
    axis: Axis,
    test: NodeTest,
    predicates: Vec<Predicate>,
}

#[derive(Debug, PartialEq)]
struct Path {
    absolute: bool,
    steps: Vec<Step>,
}

fn descendant_or_self_node() -> Step {
    Step { axis: Axis::DescendantOrSelf, test: NodeTest::Node, predicates: Vec::new() }
}

struct Parser<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.position..]
    }

    fn skip_spaces(&mut self) {
        while self.rest().starts_with(' ') {
            self.position += 1;
        }
    }

    fn eat(&mut self, token: &str) -> bool {
        self.skip_spaces();
        if self.rest().starts_with(token) {
            self.position += token.len();
            true
        } else {
            false
        }
    }

    fn error(&self, what: &str) -> String {
        format!("XPath {:?}: {what} at {}", self.text, self.position)
    }

    fn name(&mut self) -> Option<String> {
        self.skip_spaces();
        let length = self
            .rest()
            .char_indices()
            .find(|(_, c)| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.')))
            .map_or(self.rest().len(), |(index, _)| index);
        // A name does not start with a digit.
        let name = &self.rest()[..length];
        if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        self.position += name.len();
        Some(name.to_owned())
    }

    fn path(&mut self) -> Result<Path, String> {
        let mut steps = Vec::new();
        let absolute;
        if self.eat("//") {
            absolute = true;
            steps.push(descendant_or_self_node());
        } else {
            absolute = self.eat("/");
        }
        loop {
            steps.push(self.step()?);
            if self.eat("//") {
                steps.push(descendant_or_self_node());
            } else if !self.eat("/") {
                break;
            }
        }
        self.skip_spaces();
        if !self.rest().is_empty() {
            return Err(self.error("unexpected text"));
        }
        Ok(Path { absolute, steps })
    }

    fn step(&mut self) -> Result<Step, String> {
        if self.eat("..") {
            return Ok(Step { axis: Axis::Parent, test: NodeTest::Node, predicates: Vec::new() });
        }
        if self.eat(".") {
            return Ok(Step { axis: Axis::SelfAxis, test: NodeTest::Node, predicates: Vec::new() });
        }
        let mut axis = Axis::Child;
        for (name, value) in [
            ("descendant-or-self::", Axis::DescendantOrSelf),
            ("descendant::", Axis::Descendant),
            ("child::", Axis::Child),
            ("parent::", Axis::Parent),
            ("self::", Axis::SelfAxis),
        ] {
            if self.eat(name) {
                axis = value;
                break;
            }
        }
        let test = if self.eat("*") {
            NodeTest::Any
        } else if self.eat("node()") {
            NodeTest::Node
        } else {
            NodeTest::Name(self.name().ok_or_else(|| self.error("a node test"))?)
        };
        let mut predicates = Vec::new();
        while self.eat("[") {
            predicates.push(self.predicate()?);
            if !self.eat("]") {
                return Err(self.error("`]`"));
            }
        }
        Ok(Step { axis, test, predicates })
    }

    fn predicate(&mut self) -> Result<Predicate, String> {
        self.skip_spaces();
        let digits = self.rest().chars().take_while(char::is_ascii_digit).count();
        if digits > 0 {
            let value = self.rest()[..digits].parse().map_err(|_| self.error("a position"))?;
            self.position += digits;
            return Ok(Predicate::Position(value));
        }
        if !self.eat("@") {
            return Err(self.error("a position or an attribute"));
        }
        let name = self.name().ok_or_else(|| self.error("an attribute name"))?;
        if !self.eat("=") {
            return Ok(Predicate::HasAttribute(name));
        }
        self.skip_spaces();
        let quote = self.rest().chars().next().filter(|c| *c == '\'' || *c == '"').ok_or_else(|| self.error("a literal"))?;
        self.position += 1;
        let end = self.rest().find(quote).ok_or_else(|| self.error("the end of the literal"))?;
        let value = self.rest()[..end].to_owned();
        self.position += end + 1;
        Ok(Predicate::AttributeEquals(name, value))
    }
}

/// The document: index 0 is the document node, index 1 the root element; the
/// nodes are in document order.
struct Document<N> {
    nodes: Vec<Option<N>>,
    parents: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    tag_names: Vec<String>,
}

/// The most nodes a document is read with, so that a tree that changes while
/// it is read cannot make the walk endless.
const MAX_NODES: usize = 50_000;

impl<N: Clone> Document<N> {
    fn read<T: Tree<Node = N>>(tree: &T, root: &N) -> Self {
        let mut document =
            Document { nodes: vec![None], parents: vec![None], children: vec![Vec::new()], tag_names: vec![String::new()] };
        fn add<T: Tree>(tree: &T, document: &mut Document<T::Node>, node: T::Node, parent: usize) {
            if document.nodes.len() >= MAX_NODES {
                return;
            }
            let index = document.nodes.len();
            document.tag_names.push(tree.tag_name(&node));
            document.nodes.push(Some(node.clone()));
            document.parents.push(Some(parent));
            document.children.push(Vec::new());
            document.children[parent].push(index);
            for child in tree.children(&node) {
                add(tree, document, child, index);
            }
        }
        add(tree, &mut document, root.clone(), 0);
        document
    }

    fn descendants(&self, index: usize, out: &mut Vec<usize>) {
        for &child in &self.children[index] {
            out.push(child);
            self.descendants(child, out);
        }
    }

    fn axis(&self, index: usize, axis: Axis) -> Vec<usize> {
        match axis {
            Axis::Child => self.children[index].clone(),
            Axis::Parent => self.parents[index].into_iter().collect(),
            Axis::SelfAxis => vec![index],
            Axis::Descendant => {
                let mut out = Vec::new();
                self.descendants(index, &mut out);
                out
            }
            Axis::DescendantOrSelf => {
                let mut out = vec![index];
                self.descendants(index, &mut out);
                out
            }
        }
    }

    fn matches_test(&self, index: usize, test: &NodeTest) -> bool {
        match test {
            NodeTest::Node => true,
            // The document node is no element.
            NodeTest::Any => index != 0,
            NodeTest::Name(name) => index != 0 && &self.tag_names[index] == name,
        }
    }
}

/// The nodes the expression selects below `root`, in document order.
pub fn evaluate<T: Tree>(tree: &T, root: &T::Node, expression: &str) -> Result<Vec<T::Node>, String> {
    let path = Parser { text: expression, position: 0 }.path()?;
    let document = Document::read(tree, root);
    let mut context: Vec<usize> = vec![if path.absolute { 0 } else { 1 }];
    for step in &path.steps {
        let mut selected = BTreeSet::new();
        for &node in &context {
            let mut candidates: Vec<usize> =
                document.axis(node, step.axis).into_iter().filter(|&n| document.matches_test(n, &step.test)).collect();
            for predicate in &step.predicates {
                candidates = match predicate {
                    Predicate::Position(position) => candidates.get(position.wrapping_sub(1)).copied().into_iter().collect(),
                    Predicate::HasAttribute(name) => candidates
                        .into_iter()
                        .filter(|&n| document.nodes[n].as_ref().is_some_and(|node| tree.attribute(node, name).is_some()))
                        .collect(),
                    Predicate::AttributeEquals(name, value) => candidates
                        .into_iter()
                        .filter(|&n| {
                            document.nodes[n].as_ref().and_then(|node| tree.attribute(node, name)).as_deref() == Some(value)
                        })
                        .collect(),
                };
            }
            selected.extend(candidates);
        }
        context = selected.into_iter().collect();
    }
    Ok(context.into_iter().filter_map(|index| document.nodes[index].clone()).collect())
}
