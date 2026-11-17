//! `autoversion`: the rule on the command line, so a product in any language
//! can ask it. There is one implementation of the rule and no repository holds
//! a copy of it.
//!
//! Exit codes are the contract for automation: 0 when the question was
//! answered, 1 when it was refused (the refusal says what the rule declined to
//! invent), 2 when the invocation itself is wrong.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use serde_json::{json, Value};

use autoversion::surfaces;

const USAGE: &str = "usage:
  autoversion [--json] decide --current VERSION --published-surface FILE --candidate-surface FILE [--breaking]
  autoversion [--json] check --current VERSION --published-init FILE --candidate-init FILE [--expect VERSION] [--breaking] [--module-bindings]
  autoversion surface --python-init FILE [--module-bindings]
  autoversion [--json] order --older VERSION --newer VERSION";

enum Failure {
    Usage(String),
    Refused(String),
}

impl From<autoversion::RuleError> for Failure {
    fn from(error: autoversion::RuleError) -> Self {
        Failure::Refused(error.to_string())
    }
}

impl From<surfaces::SurfaceError> for Failure {
    fn from(error: surfaces::SurfaceError) -> Self {
        Failure::Refused(error.to_string())
    }
}

/// The flags after a subcommand: values by name, switches present or not.
struct Flags {
    values: BTreeMap<String, String>,
    switches: Vec<String>,
}

impl Flags {
    fn parse(words: &[String], valued: &[&str], switches: &[&str]) -> Result<Self, Failure> {
        let mut flags = Flags {
            values: BTreeMap::new(),
            switches: Vec::new(),
        };
        let mut words = words.iter();
        while let Some(word) = words.next() {
            if valued.contains(&word.as_str()) {
                let value = words
                    .next()
                    .ok_or_else(|| Failure::Usage(format!("{word} needs a value")))?;
                flags.values.insert(word.clone(), value.clone());
            } else if switches.contains(&word.as_str()) {
                flags.switches.push(word.clone());
            } else {
                return Err(Failure::Usage(format!("unrecognized argument: {word}")));
            }
        }
        Ok(flags)
    }

    fn required(&self, flag: &str) -> Result<&str, Failure> {
        self.values
            .get(flag)
            .map(String::as_str)
            .ok_or_else(|| Failure::Usage(format!("the following argument is required: {flag}")))
    }

    fn switch(&self, flag: &str) -> bool {
        self.switches.iter().any(|present| present == flag)
    }
}

fn read_surface(path: &str) -> Result<Vec<String>, Failure> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| Failure::Refused(format!("{path}: {error}")))?;
    let document: Value = serde_json::from_str(&text)
        .map_err(|error| Failure::Refused(format!("{path}: not JSON: {error}")))?;
    let names = document.get("surface").ok_or_else(|| {
        Failure::Refused(format!(
            "{path}: no \"surface\" key. A surface document is {{\"surface\": [\"name\", ...]}}"
        ))
    })?;
    names
        .as_array()
        .and_then(|names| {
            names
                .iter()
                .map(|name| name.as_str().map(str::to_string))
                .collect()
        })
        .ok_or_else(|| Failure::Refused(format!("{path}: \"surface\" is not a list of names")))
}

/// `document` as indented JSON. A document that cannot be written is
/// refused naming why, never printed as an empty line that reads as an
/// answer.
fn json_text(document: &Value) -> Result<String, Failure> {
    serde_json::to_string_pretty(document).map_err(|error| {
        Failure::Refused(format!(
            "the answer could not be written as JSON: {error}; it held {document}"
        ))
    })
}

/// The answer as sorted, indented JSON, or one `key: value` line per field
/// that holds something.
fn emit(payload: &[(&str, Value)], as_json: bool) -> Result<(), Failure> {
    if as_json {
        let sorted: BTreeMap<String, Value> = payload
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect();
        println!("{}", json_text(&Value::Object(sorted.into_iter().collect()))?);
        return Ok(());
    }
    for (key, value) in payload {
        let rendered = match value {
            Value::Array(items) => items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        if !rendered.is_empty() {
            println!("{key}: {rendered}");
        }
    }
    Ok(())
}

/// The five fields every decision holds; one that is missing is a decision
/// this binary cannot report, and is refused naming it.
fn answer_fields(answer: &Value) -> Result<Vec<(&'static str, Value)>, Failure> {
    ["current", "change", "next", "removed", "added"]
        .into_iter()
        .map(|key| match answer.get(key) {
            Some(value) => Ok((key, value.clone())),
            None => Err(Failure::Refused(format!(
                "the version decision holds no `{key}`; it held {answer}"
            ))),
        })
        .collect()
}

/// The version a decision requires, which every decision states as text.
fn next_version(answer: &Value) -> Result<&str, Failure> {
    match answer.get("next").and_then(Value::as_str) {
        Some(next) => Ok(next),
        None => Err(Failure::Refused(format!(
            "the version decision states no next version as text; it held {answer}"
        ))),
    }
}

fn run(arguments: &[String]) -> Result<(), Failure> {
    let (global_json, arguments) = match arguments.split_first() {
        Some((first, rest)) if first == "--json" => (true, rest),
        _ => (false, arguments),
    };
    let (command, rest) = arguments
        .split_first()
        .ok_or_else(|| Failure::Usage("the following argument is required: command".to_string()))?;
    match command.as_str() {
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        "decide" => {
            let flags = Flags::parse(
                rest,
                &["--current", "--published-surface", "--candidate-surface"],
                &["--breaking", "--json"],
            )?;
            let published = read_surface(flags.required("--published-surface")?)?;
            let candidate = read_surface(flags.required("--candidate-surface")?)?;
            let answer = autoversion::decide(
                flags.required("--current")?,
                &published,
                &candidate,
                flags.switch("--breaking"),
            )?;
            emit(
                &answer_fields(&answer)?,
                global_json || flags.switch("--json"),
            )
        }
        "check" => {
            let flags = Flags::parse(
                rest,
                &[
                    "--current",
                    "--published-init",
                    "--candidate-init",
                    "--expect",
                ],
                &["--breaking", "--module-bindings", "--json"],
            )?;
            let bindings = flags.switch("--module-bindings");
            let published =
                surfaces::python_surface(Path::new(flags.required("--published-init")?), bindings)?;
            let candidate =
                surfaces::python_surface(Path::new(flags.required("--candidate-init")?), bindings)?;
            let answer = autoversion::decide(
                flags.required("--current")?,
                &published,
                &candidate,
                flags.switch("--breaking"),
            )?;
            let mut fields = answer_fields(&answer)?;
            if bindings {
                fields.push(("surface", json!("module bindings")));
            }
            emit(&fields, global_json || flags.switch("--json"))?;
            let next = next_version(&answer)?;
            match flags.values.get("--expect") {
                Some(expected) if expected != next => Err(Failure::Refused(format!(
                    "the surface change requires {next}, but the product declares {expected}"
                ))),
                _ => Ok(()),
            }
        }
        "surface" => {
            let flags = Flags::parse(rest, &["--python-init"], &["--module-bindings", "--json"])?;
            let bindings = flags.switch("--module-bindings");
            let names =
                surfaces::python_surface(Path::new(flags.required("--python-init")?), bindings)?;
            // Sorted by key, as every JSON answer here is.
            let mut document = BTreeMap::new();
            document.insert("surface".to_string(), json!(names));
            if bindings {
                document.insert("source".to_string(), json!("module bindings"));
            }
            println!("{}", json_text(&Value::Object(document.into_iter().collect()))?);
            Ok(())
        }
        "order" => {
            let flags = Flags::parse(rest, &["--older", "--newer"], &["--json"])?;
            let older = flags.required("--older")?;
            let newer = flags.required("--newer")?;
            let is_newer = if autoversion::version_newer(older, newer) {
                "True"
            } else {
                "False"
            };
            emit(
                &[
                    ("older", json!(older)),
                    ("newer", json!(newer)),
                    ("is_newer", json!(is_newer)),
                ],
                global_json || flags.switch("--json"),
            )
        }
        other => Err(Failure::Usage(format!("invalid choice: {other}"))),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1 /* https://doc.rust-lang.org/std/env/fn.args.html */).collect();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Refused(reason)) => {
            eprintln!("refused: {reason}");
            ExitCode::FAILURE
        }
        Err(Failure::Usage(reason)) => {
            eprintln!("autoversion: error: {reason}\n{USAGE}");
            ExitCode::from(2) /* https://docs.rs/clap/latest/clap/error/struct.Error.html#method.exit */
        }
    }
}
