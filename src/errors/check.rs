//! The whole catalogue held to the error standard. A cause names the real
//! test that provokes it: a function of that name in the file it names
//! (`fn name(`). A test no local state can provoke is listed, with why, in
//! `tests/untested-causes.json`, and the list is held to the catalogue as
//! well: an entry for a cause nobody declares, for a test already written, or
//! naming another test than the cause does is a problem, so the list cannot
//! hide a test that was promised and never written.

use std::path::Path;

use serde_json::Value;
use wisent_errors::{Operation, Warning};

use super::OPERATIONS;

/// Where the checkout lists the causes whose test cannot run locally.
pub const UNTESTED: &str = "tests/untested-causes.json";

/// The fields every entry of [`UNTESTED`] holds, and no other.
const FIELDS: &[&str] = &["operation", "cause", "test", "why"];

/// One cause whose test needs a state no local run provokes.
#[derive(Debug)]
struct Untested {
    operation: String,
    cause: String,
    test: String,
    why: String,
}

/// Everything that keeps the catalogue from meeting the standard: each
/// declaration's own problems, the whole catalogue's (a functionality nothing
/// starts, one used but not listed), each cause or warning whose test is
/// neither written under `root` nor listed in its untested causes, and each
/// listed entry the catalogue does not back.
pub fn problems(root: &Path) -> Vec<String> {
    let mut problems = wisent_errors::catalogue_problems(OPERATIONS);
    let listed = untested(root, &mut problems);
    for operation in OPERATIONS {
        problems.extend(operation.problems());
        for cause in operation.causes {
            let entry = listed
                .iter()
                .find(|entry| entry.operation == operation.id && entry.cause == cause.id);
            if let Some(problem) = tested(root, &format!("{}.{}", operation.id, cause.id), cause.test, entry) {
                problems.push(problem);
            }
        }
        for warning in operation.warnings {
            if let Some(problem) = warned(root, operation, warning) {
                problems.push(problem);
            }
        }
    }
    for entry in &listed {
        let declared = OPERATIONS
            .iter()
            .any(|operation| operation.id == entry.operation && operation.cause(&entry.cause).is_some());
        if !declared {
            problems.push(format!(
                "{UNTESTED} lists {}.{}, which the catalogue does not declare; remove the entry",
                entry.operation, entry.cause
            ));
        }
    }
    problems
}

/// The untested causes the checkout lists; none when it keeps no list.
fn untested(root: &Path, problems: &mut Vec<String>) -> Vec<Untested> {
    let path = root.join(UNTESTED);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => {
            problems.push(format!("{} cannot be read: {error}", path.display()));
            return Vec::new();
        }
    };
    let shape = || format!("{} is not a list of {{operation, cause, test, why}}", path.display());
    let entries = match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(entries)) => entries,
        Ok(other) => {
            problems.push(format!("{}: it holds {other}", shape()));
            return Vec::new();
        }
        Err(error) => {
            problems.push(format!("{}: {error}", shape()));
            return Vec::new();
        }
    };
    let mut listed = Vec::new();
    for entry in entries {
        let field = |name: &str| entry.get(name).and_then(Value::as_str).map(str::to_owned);
        // Only the four fields an entry holds: a misspelt key is refused, not ignored.
        let only_fields = entry
            .as_object()
            .is_some_and(|object| object.keys().all(|key| FIELDS.contains(&key.as_str())));
        match (field("operation"), field("cause"), field("test"), field("why"), only_fields) {
            (Some(operation), Some(cause), Some(test), Some(why), true) => listed.push(Untested {
                operation,
                cause,
                test,
                why,
            }),
            _ => problems.push(format!("{}: an entry is {entry}", shape())),
        }
    }
    listed
}

/// Whether the file at `path` declares the test function `name`.
fn written(path: &Path, name: &str) -> std::io::Result<bool> {
    let text = std::fs::read_to_string(path)?;
    Ok(text.contains(&format!("fn {name}(")))
}

/// The problem with the test of the cause `id`, if it has one: written and
/// listed both, neither written nor listed, or listed under another test or
/// without why.
fn tested(root: &Path, id: &str, test: &str, entry: Option<&Untested>) -> Option<String> {
    let (file, name) = test.split_once("::")?;
    let path = root.join(file);
    let written = match written(&path, name) {
        Ok(written) => written,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Some(format!("cause {id}: its test file {} cannot be read: {error}", path.display())),
    };
    match entry {
        None if written => None,
        None => Some(format!(
            "cause {id}: {} declares no test function `{name}`, and {UNTESTED} does not list it; write the test, or list it with why no local state provokes it",
            path.display()
        )),
        Some(_) if written => Some(format!(
            "cause {id}: its test {test} is written, so {UNTESTED} must not list it; remove the entry"
        )),
        Some(entry) if entry.test != test => Some(format!(
            "cause {id}: {UNTESTED} lists test {}, but the cause names {test}",
            entry.test
        )),
        Some(entry) if entry.why.trim().is_empty() => Some(format!(
            "cause {id}: {UNTESTED} lists it without why no local state provokes it"
        )),
        Some(_) => None,
    }
}

/// The problem with `warning`'s test, if it is not written: a warning's
/// test refuses without the confirmation, which any local run provokes, so
/// none is listed as untested.
fn warned(root: &Path, operation: &Operation, warning: &Warning) -> Option<String> {
    let (file, name) = warning.test.split_once("::")?;
    let path = root.join(file);
    match written(&path, name) {
        Ok(true) => None,
        Ok(false) => Some(format!(
            "warning {}.{}: {} declares no test function `{name}`; write it",
            operation.id,
            warning.id,
            path.display()
        )),
        Err(error) => Some(format!(
            "warning {}.{}: its test file {} cannot be read: {error}",
            operation.id,
            warning.id,
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_catalogue_meets_the_error_standard() {
        let problems = super::problems(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}
