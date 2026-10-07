//! Surface extractors for language conventions, not for products.
//!
//! The rule takes a set of public names and does not care where they came from.
//! A convention every project in a language shares — `__all__` in a Python
//! package — is read here once, so no repository writes the same syntax walk.
//! Where a product keeps its files, what it calls its entry point, and which of
//! its modules are public is product knowledge and stays with the product.
//!
//! Sources are parsed, never imported: importing a package to learn its names
//! runs its side effects and needs its dependencies, which a release decision
//! should not.

use std::fmt;
use std::path::Path;

use tree_sitter::{Node, Parser, Tree};

/// The surface could not be read from where the caller pointed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceError(pub String);

impl fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SurfaceError {}

fn parse(path: &Path) -> Result<(String, Tree), SurfaceError> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| SurfaceError(format!("{}: {error}", path.display())))?;
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|error| SurfaceError(format!("the Python grammar did not load: {error}")))?;
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| SurfaceError(format!("{}: the parser returned no tree", path.display())))?;
    if tree.root_node().has_error() {
        return Err(SurfaceError(format!(
            "{}: not parseable Python",
            path.display()
        )));
    }
    Ok((source, tree))
}

fn text<'s>(node: Node, source: &'s str) -> &'s str {
    &source[node.byte_range()]
}

/// The value of a plain string literal: no prefix that makes it bytes or a
/// template, no interpolation, no escape to decode.
fn string_literal(node: Node, source: &str) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    let mut value = String::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "string_start" => {
                let prefix = text(child, source)
                    .trim_end_matches(['"', '\''])
                    .to_ascii_lowercase();
                if prefix.contains('b') || prefix.contains('f') || prefix.contains('t') {
                    return None;
                }
            }
            "string_content" => {
                let mut inner = child.walk();
                if child.children(&mut inner).next().is_some() {
                    return None;
                }
                value.push_str(text(child, source));
            }
            "string_end" => {}
            _ => return None,
        }
    }
    Some(value)
}

/// Every node of the tree, depth first.
fn walk(root: Node) -> Vec<Node> {
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        found.push(node);
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        pending.extend(children.into_iter().rev());
    }
    found
}

/// The names in a Python module's `__all__`.
pub fn python_all(path: &Path) -> Result<Vec<String>, SurfaceError> {
    let (source, tree) = parse(path)?;
    for node in walk(tree.root_node()) {
        if node.kind() != "assignment" {
            continue;
        }
        let Some(left) = node.child_by_field_name("left") else {
            continue;
        };
        if left.kind() != "identifier" || text(left, &source) != "__all__" {
            continue;
        }
        let value = node.child_by_field_name("right");
        let Some(value) = value
            .filter(|value| matches!(value.kind(), "list" | "tuple" | "set" | "expression_list"))
        else {
            return Err(SurfaceError(format!(
                "{}: __all__ is not a literal list, tuple or set, so its contents cannot be read \
                 without executing the module",
                path.display()
            )));
        };
        let mut names = Vec::new();
        let mut cursor = value.walk();
        for element in value.named_children(&mut cursor) {
            if element.kind() == "comment" {
                continue;
            }
            let name = string_literal(element, &source).ok_or_else(|| {
                SurfaceError(format!(
                    "{}: __all__ contains an entry that is not a string literal",
                    path.display()
                ))
            })?;
            names.push(name);
        }
        return Ok(names);
    }
    Err(SurfaceError(format!(
        "{}: no __all__ found. A package without one has no declared public surface, so a \
         release cannot be classified from it — declare __all__, or supply a surface the \
         product extracts itself",
        path.display()
    )))
}

/// Public names a module binds, for packages that have not declared `__all__`:
/// module-level definitions, assignments and `from … import` re-exports whose
/// names do not begin with an underscore. A plain `import os` is skipped, so
/// rearranging imports does not read as an added or removed promise.
pub fn python_declared(path: &Path) -> Result<Vec<String>, SurfaceError> {
    let (source, tree) = parse(path)?;
    let mut names: Vec<String> = Vec::new();
    let mut keep = |name: &str| {
        if !name.starts_with('_') && !names.iter().any(|kept| kept == name) {
            names.push(name.to_string());
        }
    };
    let root = tree.root_node();
    let mut cursor = root.walk();
    for statement in root.named_children(&mut cursor) {
        let statement = match statement.kind() {
            "decorated_definition" => match statement.child_by_field_name("definition") {
                Some(definition) => definition,
                None => continue,
            },
            _ => statement,
        };
        match statement.kind() {
            "function_definition" | "class_definition" => {
                if let Some(name) = statement.child_by_field_name("name") {
                    keep(text(name, &source));
                }
            }
            "expression_statement" => {
                let mut inner = statement.walk();
                for expression in statement.named_children(&mut inner) {
                    let mut assignment = Some(expression);
                    while let Some(node) = assignment.filter(|node| node.kind() == "assignment") {
                        if let Some(left) = node.child_by_field_name("left") {
                            if left.kind() == "identifier" {
                                keep(text(left, &source));
                            }
                        }
                        assignment = node.child_by_field_name("right");
                    }
                }
            }
            "import_from_statement" => {
                let mut inner = statement.walk();
                for imported in statement.children_by_field_name("name", &mut inner) {
                    match imported.kind() {
                        "aliased_import" => {
                            if let Some(alias) = imported.child_by_field_name("alias") {
                                keep(text(alias, &source));
                            }
                        }
                        _ => keep(text(imported, &source)),
                    }
                }
            }
            _ => {}
        }
    }
    if names.is_empty() {
        return Err(SurfaceError(format!(
            "{}: no public names found, with or without __all__",
            path.display()
        )));
    }
    Ok(names)
}

/// The surface the caller names: the module's `__all__`, or with
/// `module_bindings` its public module-level bindings. Never one after the
/// other: a tree without `__all__` is refused rather than read by another
/// rule than the tree it is compared with.
pub fn python_surface(path: &Path, module_bindings: bool) -> Result<Vec<String>, SurfaceError> {
    if module_bindings {
        return python_declared(path);
    }
    python_all(path).map_err(|error| {
        SurfaceError(format!(
            "{error}; name the module bindings as the surface with --module-bindings if that is what the package promises"
        ))
    })
}
