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
  autoversion [--json] check --current VERSION --published-init FILE --candidate-init FILE [--expect VERSION] [--breaking] [--fallback]
  autoversion surface --python-init FILE [--fallback]
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
        let mut flags = Flags { values: BTreeMap::new(), switches: Vec::new() };
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
    let text = std::fs::read_to_string(path).map_err(|error| Failure::Refused(format!("{path}: {error}")))?;
    let document: Value =
        serde_json::from_str(&text).map_err(|error| Failure::Refused(format!("{path}: not JSON: {error}")))?;
    let names = document.get("surface").ok_or_else(|| {
        Failure::Refused(format!(
            "{path}: no \"surface\" key. A surface document is {{\"surface\": [\"name\", ...]}}"
        ))
    })?;
    names
        .as_array()
        .and_then(|names| names.iter().map(|name| name.as_str().map(str::to_string)).collect())
        .ok_or_else(|| Failure::Refused(format!("{path}: \"surface\" is not a list of names")))
}

/// The answer as sorted, indented JSON, or one `key: value` line per field
/// that holds something.
fn emit(payload: &[(&str, Value)], as_json: bool) {
    if as_json {
        let sorted: BTreeMap<&str, &Value> = payload.iter().map(|(key, value)| (*key, value)).collect();
        println!("{}", serde_json::to_string_pretty(&sorted).unwrap_or_default());
        return;
    }
    for (key, value) in payload {
        let rendered = match value {
            Value::Array(items) => items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        if !rendered.is_empty() {
            println!("{key}: {rendered}");
        }
    }
}

fn answer_fields(answer: &Value) -> Vec<(&'static str, Value)> {
    ["current", "change", "next", "removed", "added"]
        .into_iter()
        .map(|key| (key, answer.get(key).cloned().unwrap_or(Value::Null)))
        .collect()
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
            let answer =
                autoversion::decide(flags.required("--current")?, &published, &candidate, flags.switch("--breaking"))?;
            emit(&answer_fields(&answer), global_json || flags.switch("--json"));
            Ok(())
        }
        "check" => {
            let flags = Flags::parse(
                rest,
                &["--current", "--published-init", "--candidate-init", "--expect"],
                &["--breaking", "--fallback", "--json"],
            )?;
            let fallback = flags.switch("--fallback");
            let (published, published_guessed) =
                surfaces::python_surface(Path::new(flags.required("--published-init")?), fallback)?;
            let (candidate, candidate_guessed) =
                surfaces::python_surface(Path::new(flags.required("--candidate-init")?), fallback)?;
            let answer =
                autoversion::decide(flags.required("--current")?, &published, &candidate, flags.switch("--breaking"))?;
            let mut fields = answer_fields(&answer);
            if published_guessed || candidate_guessed {
                fields.push(("surface", json!("inferred, because __all__ is absent")));
            }
            emit(&fields, global_json || flags.switch("--json"));
            let next = answer.get("next").and_then(Value::as_str).unwrap_or_default();
            match flags.values.get("--expect") {
                Some(expected) if expected != next => Err(Failure::Refused(format!(
                    "the surface change requires {next}, but the product declares {expected}"
                ))),
                _ => Ok(()),
            }
        }
        "surface" => {
            let flags = Flags::parse(rest, &["--python-init"], &["--fallback", "--json"])?;
            let (names, guessed) =
                surfaces::python_surface(Path::new(flags.required("--python-init")?), flags.switch("--fallback"))?;
            let mut document = BTreeMap::new();
            document.insert("surface", json!(names));
            if guessed {
                document.insert("inferred", json!("no __all__; names were read from module-level bindings"));
            }
            println!("{}", serde_json::to_string_pretty(&document).unwrap_or_default());
            Ok(())
        }
        "order" => {
            let flags = Flags::parse(rest, &["--older", "--newer"], &["--json"])?;
            let older = flags.required("--older")?;
            let newer = flags.required("--newer")?;
            let is_newer = if autoversion::version_newer(older, newer) { "True" } else { "False" };
            emit(
                &[("older", json!(older)), ("newer", json!(newer)), ("is_newer", json!(is_newer))],
                global_json || flags.switch("--json"),
            );
            Ok(())
        }
        other => Err(Failure::Usage(format!("invalid choice: {other}"))),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Refused(reason)) => {
            eprintln!("refused: {reason}");
            ExitCode::FAILURE
        }
        Err(Failure::Usage(reason)) => {
            eprintln!("autoversion: error: {reason}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}
