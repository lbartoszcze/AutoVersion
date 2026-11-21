//! `autoversion`: the rule on the command line, so a product in any language
//! can ask it. There is one implementation of the rule and no repository holds
//! a copy of it.
//!
//! Exit codes are the contract for automation: 0 when the question was
//! answered, 1 when it was refused (the refusal says what the rule declined to
//! invent), 2 when the invocation itself is wrong. Every refusal is a cause
//! `errors` declares: its sentence on stderr as it always read, then the
//! wisent-errors envelope as one JSON line.

mod errors;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use serde_json::{json, Value};

use autoversion::surfaces;
use errors::commands::EXPECTED_VERSION_UNMADE;
use errors::document::{DOCUMENT_NOT_JSON, DOCUMENT_UNREADABLE, SURFACE_KEY_MISSING, SURFACE_NOT_NAMES};
use errors::product::{COMMAND_REFUSED, FLAG_INCOMPLETE, FLAG_UNRECOGNIZED};
use errors::{refuse, Declared, AUTOVERSION, CHECK, DOCUMENT};

const USAGE: &str = "usage:
  autoversion [--json] decide --current VERSION --published-surface FILE --candidate-surface FILE [--breaking]
  autoversion [--json] check --current VERSION --published-init FILE --candidate-init FILE [--expect VERSION] [--breaking] [--module-bindings]
  autoversion surface --python-init FILE [--module-bindings]
  autoversion [--json] order --older VERSION --newer VERSION";

/// The flags after a subcommand: values by name, switches present or not.
struct Flags {
    values: BTreeMap<String, String>,
    switches: Vec<String>,
}

impl Flags {
    fn parse(words: &[String], valued: &[&str], switches: &[&str]) -> Result<Self, Declared> {
        let mut flags = Flags {
            values: BTreeMap::new(),
            switches: Vec::new(),
        };
        let mut words = words.iter();
        while let Some(word) = words.next() {
            if valued.contains(&word.as_str()) {
                let value = words.next().ok_or_else(|| {
                    refuse(&AUTOVERSION, &FLAG_INCOMPLETE, format!("{word} needs a value"))
                })?;
                flags.values.insert(word.clone(), value.clone());
            } else if switches.contains(&word.as_str()) {
                flags.switches.push(word.clone());
            } else {
                return Err(refuse(&AUTOVERSION, &FLAG_UNRECOGNIZED, format!("unrecognized argument: {word}")));
            }
        }
        Ok(flags)
    }

    fn required(&self, flag: &str) -> Result<&str, Declared> {
        self.values.get(flag).map(String::as_str).ok_or_else(|| {
            refuse(&AUTOVERSION, &FLAG_INCOMPLETE, format!("the following argument is required: {flag}"))
        })
    }

    fn switch(&self, flag: &str) -> bool {
        self.switches.iter().any(|present| present == flag)
    }
}

fn read_surface(path: &str) -> Result<Vec<String>, Declared> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| refuse(&DOCUMENT, &DOCUMENT_UNREADABLE, format!("{path}: {error}")))?;
    let document: Value = serde_json::from_str(&text)
        .map_err(|error| refuse(&DOCUMENT, &DOCUMENT_NOT_JSON, format!("{path}: not JSON: {error}")))?;
    let names = document.get("surface").ok_or_else(|| {
        refuse(
            &DOCUMENT,
            &SURFACE_KEY_MISSING,
            format!("{path}: no \"surface\" key. A surface document is {{\"surface\": [\"name\", ...]}}"),
        )
    })?;
    names
        .as_array()
        .and_then(|names| {
            names
                .iter()
                .map(|name| name.as_str().map(str::to_string))
                .collect()
        })
        .ok_or_else(|| refuse(&DOCUMENT, &SURFACE_NOT_NAMES, format!("{path}: \"surface\" is not a list of names")))
}

/// The answer as sorted, indented JSON, or one `key: value` line per field
/// that holds something.
fn emit(payload: Vec<(&str, Value)>, as_json: bool) {
    if as_json {
        // serde_json's Map keeps its keys sorted, and `{:#}` indents it.
        let object: serde_json::Map<String, Value> = payload
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        println!("{:#}", Value::Object(object));
        return;
    }
    for (key, value) in payload {
        let rendered = match value {
            Value::Array(items) => items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
            Value::String(text) => text,
            other => other.to_string(),
        };
        if !rendered.is_empty() {
            println!("{key}: {rendered}");
        }
    }
}

/// The five fields of a decision, in the order a person reads them.
fn decision_fields(decision: &autoversion::Decision) -> Vec<(&'static str, Value)> {
    vec![
        ("current", json!(decision.current.to_string())),
        ("change", json!(decision.change)),
        ("next", json!(decision.next.to_string())),
        ("removed", json!(decision.removed)),
        ("added", json!(decision.added)),
    ]
}

fn run(arguments: &[String]) -> Result<(), Declared> {
    let (global_json, arguments) = match arguments.split_first() {
        Some((first, rest)) if first == "--json" => (true, rest),
        _ => (false, arguments),
    };
    let (command, rest) = arguments.split_first().ok_or_else(|| {
        refuse(&AUTOVERSION, &COMMAND_REFUSED, "the following argument is required: command")
    })?;
    // --help is answered at every level: after a command as well as alone.
    let asks_help = |word: &String| matches!(word.as_str(), "--help" | "-h");
    if command == "help" || asks_help(command) || rest.iter().any(asks_help) {
        println!("{USAGE}");
        return Ok(());
    }
    match command.as_str() {
        "decide" => {
            let flags = Flags::parse(
                rest,
                &["--current", "--published-surface", "--candidate-surface"],
                &["--breaking", "--json"],
            )?;
            let published = read_surface(flags.required("--published-surface")?)?;
            let candidate = read_surface(flags.required("--candidate-surface")?)?;
            let decision = autoversion::decide(
                flags.required("--current")?,
                &published,
                &candidate,
                flags.switch("--breaking"),
            )?;
            emit(
                decision_fields(&decision),
                global_json || flags.switch("--json"),
            );
            Ok(())
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
            let decision = autoversion::decide(
                flags.required("--current")?,
                &published,
                &candidate,
                flags.switch("--breaking"),
            )?;
            let mut fields = decision_fields(&decision);
            if bindings {
                fields.push(("surface", json!("module bindings")));
            }
            emit(fields, global_json || flags.switch("--json"));
            let next = decision.next.to_string();
            match flags.values.get("--expect") {
                Some(expected) if *expected != next => Err(refuse(
                    &CHECK,
                    &EXPECTED_VERSION_UNMADE,
                    format!("the surface change requires {next}, but the product declares {expected}"),
                )),
                _ => Ok(()),
            }
        }
        "surface" => {
            let flags = Flags::parse(rest, &["--python-init"], &["--module-bindings", "--json"])?;
            let bindings = flags.switch("--module-bindings");
            let names =
                surfaces::python_surface(Path::new(flags.required("--python-init")?), bindings)?;
            // serde_json's Map keeps its keys sorted, as every JSON answer here is.
            let mut document = serde_json::Map::new();
            document.insert("surface".to_string(), json!(names));
            if bindings {
                document.insert("source".to_string(), json!("module bindings"));
            }
            println!("{:#}", Value::Object(document));
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
                vec![
                    ("older", json!(older)),
                    ("newer", json!(newer)),
                    ("is_newer", json!(is_newer)),
                ],
                global_json || flags.switch("--json"),
            );
            Ok(())
        }
        other => Err(refuse(&AUTOVERSION, &COMMAND_REFUSED, format!("invalid choice: {other}"))),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1 /* https://doc.rust-lang.org/std/env/fn.args.html */).collect();
    let refusal = match run(&arguments) {
        Ok(()) => return ExitCode::SUCCESS,
        Err(refusal) => refusal,
    };
    if refusal.usage() {
        eprintln!("autoversion: error: {refusal}\n{USAGE}");
    } else {
        eprintln!("refused: {refusal}");
    }
    eprintln!("{}", refusal.failure().to_json());
    status(refusal.exit_code())
}

/// The process status a refusal's exit code is: 1 or 2, each a byte.
fn status(code: i32) -> ExitCode {
    match u8::try_from(code) {
        Ok(code) => ExitCode::from(code),
        // fallback: no declared status leaves a byte (they are 1 and 2); were
        // one to, the command still ends as a failure.
        Err(_) => ExitCode::FAILURE,
    }
}
